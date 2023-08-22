#![cfg_attr(not(feature = "std"), no_std)]

use common::Balance;
use codec::{Decode, Encode};

#[derive(Encode, Decode, Default, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Pool<AssetId> {
    // Asset pool is using
    asset_id: AssetId,
    // Total balance of the pool
    pool_balance: Balance,
    // Lending interest
    lending_interest: Balance,
    // Borrowing interest
    borrowing_interest: Balance,
    // Is the pool lending tokens
    is_lending: bool,
}

#[derive(Encode, Decode, PartialEq, Eq, Default, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct User<BlockNumber, AssetId> {
    // Lending info 

    // Lended asset
    pub lended_token: AssetId,
    // Lended amount
    pub lended_amount: Balance,
    // Last time User has leded tokens (which block number)
    pub last_time_lended: BlockNumber,  
    // Interest earned
    pub interest_earned: Balance,
    
    // Borrowing info

    // Borrowed asset
    pub borrowed_token: AssetId,
    // Borrowed amount
    pub borrowed_amount: Balance,
    // Last time User has borrowed tokens (which block number)
    pub last_time_borrowed: BlockNumber,
    // Interest on borrowed amount
    pub debt_interest: Balance,
    // User's collateral
    pub collateral: Balance,
    // Is user liquidated
    pub liquidated: bool,
    // Has user paid its debt
    pub debt_paid: bool,
}


pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    // Pallet imports
    use crate::{Pool, User};
    use common::prelude::{AssetInfoProvider, Balance};
    use frame_support::pallet_prelude::*;
    use frame_support::transactional;
    use frame_support::PalletId;
    use frame_support::sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
    use frame_system::pallet_prelude::*;
    use common::balance;
    use hex_literal::hex;

    /// Pallet ID
    const PALLET_ID: PalletId = PalletId(*b"tplndbrw");

    #[pallet::config]  
    pub trait Config: frame_system::Config + assets::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    /// Aliases
    type Assets<T> = assets::Pallet<T>;
    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
    pub type AssetIdOf<T> = <T as assets::Config>::AssetId;
    pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);


    /// Events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]  
    pub enum Event<T: Config> {
                /// Pool Created [asset, initial_pool_balance, is_lending]
                PoolCreated(AssetIdOf<T>, Balance, bool),
                /// Assets Lended [who, asset, amount]
                AssetsLended(AccountIdOf<T>, AssetIdOf<T>, Balance),
                /// Assets Borrowed [who, asset, borrowed_amount, collateral]
                AssetsBorrowed(AccountIdOf<T>, AssetIdOf<T>, Balance, Balance),
                /// Amount withdrawn
                AmountWithdrawn(AccountIdOf<T>, AssetIdOf<T>, Balance),
                /// Full Lending Amount Withdrawn [who, asset, amount]
                FullAmountWithdrawn(AccountIdOf<T>, AssetIdOf<T>, Balance, Balance),
                /// Debt repaid in full
                DebtFullyRepaid(AccountIdOf<T>, AssetIdOf<T>, Balance),
                /// Debt partially repaid
                DebtPartiallyRepaid(AccountIdOf<T>, AssetIdOf<T>, Balance),
    } 


    /// Errors
    #[pallet::error]
    pub enum Error<T> {
        /// Pool is unavailable for lending
        LendingUnavailable,
        /// Unauthorized
        Unauthorized,
        /// Insufficient funds
        InsufficientFunds,
        /// Invalid asset
        InvalidAsset,
        /// Invalid interest proportion
        InvalidInterestProportion,
        /// Insufficient funds in pool
        NotEnoughTokensInPool,
        /// Insufficient collateral
        InsufficientCollateral,
        /// Excessive amount
        ExcessiveAmount,
        /// Non-Existing user
        UserDoesNotExist,
    }         

    /// Pool info storage
    #[pallet::storage]
    #[pallet::getter(fn pool_data)]
    pub type PoolInfo<T: Config> = StorageMap<_, Identity, AssetIdOf<T>, Pool<AssetIdOf<T>>, ValueQuery>; 
    
    /// User info storage
    #[pallet::storage]
    #[pallet::getter(fn user_data)]
    pub type UserInfo<T: Config> = StorageMap<_, Identity, AccountIdOf<T>, User<BlockNumberFor<T>, AssetIdOf<T>>, OptionQuery>;

    #[pallet::type_value]
    pub fn DefaultForAuthorityAccount<T: Config>() -> AccountIdOf<T> {
        let bytes = hex!("96ea3c9c0be7bbc7b0656a1983db5eed75210256891a9609012362e36815b132");
        AccountIdOf::<T>::decode(&mut &bytes[..]).unwrap()
    }
    
    /// Authority account storage
    #[pallet::storage]
    #[pallet::getter(fn authority_account)]
    pub type AuthorityAccount<T: Config> =
        StorageValue<_, AccountIdOf<T>, ValueQuery, DefaultForAuthorityAccount<T>>;

   
    #[pallet::call]  
     impl<T: Config> Pallet<T> {
        
        /// Create pool
        
        #[pallet::call_index(0)]
        #[pallet::weight(10000)]
        pub fn create_pool(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            pool_balance: Balance,
            lending_interest: Balance,
            borrowing_interest: Balance,
            is_lending: bool
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            // Check if authority account is calling the function
            if user != AuthorityAccount::<T>::get() {
                return Err(Error::<T>::Unauthorized.into());
            }

            // Check if ledning and borrowing interests are in the right proportion 
            ensure!(
                lending_interest == borrowing_interest * balance!(0.7),
                Error::<T>::InvalidInterestProportion
            );

            // Create pool
            let pool = Pool {
                    asset_id,
                    pool_balance,
                    lending_interest,
                    borrowing_interest,
                    is_lending: true,
                }; 
             
            // Add pool to the storage
            <PoolInfo<T>>::insert(asset_id, pool);

            // Emit an event
            Self::deposit_event(Event::PoolCreated(asset_id, pool_balance, is_lending));

            Ok(().into())
        }

        /// Lend tokens to the pool

        #[pallet::call_index(1)]
        #[pallet::weight(10000)]
        #[transactional]
        pub fn lend(
            origin: OriginFor<T>,
            lended_token: AssetIdOf<T>,
            lended_amount: Balance
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let user_info = <UserInfo<T>>::get(&user);
            let current_block = frame_system::Pallet::<T>::block_number();
            let mut pool = <PoolInfo<T>>::get(&lended_token);

            // Check if there is a pool available for lending
            ensure!(pool.is_lending, Error::<T>::LendingUnavailable);
            // Check if the lending asset is accepted by the pool
            ensure!(pool.asset_id == lended_token, Error::<T>::InvalidAsset);
            // Check if user has enough funds to lend to the pool
            ensure!(
                Assets::<T>::free_balance(&lended_token, &user).unwrap_or(0) >= lended_amount,
                Error::<T>::InsufficientFunds
            );

            // Add lended tokens to the pool
            pool.pool_balance += lended_amount;

            // Add the lended amount to existing amount (if user exists)
            // Create new user with lended amount(if user doesn't exist
            if let Some(mut user_info) = user_info {
                user_info.lended_amount += lended_amount;
                user_info.interest_earned += Self::calculate_interest(&pool.lending_interest, &user_info.lended_amount, &user_info.last_time_lended);
                user_info.last_time_lended = current_block;
            } else {
                let new_user_info = User {
                    lended_token, 
                    lended_amount,
                    last_time_lended: <frame_system::Pallet<T>>::block_number(),
                    ..Default::default()
                };

                <UserInfo<T>>::insert(&user, new_user_info);
            }

            // Transfer assets from user to the pool
            Assets::<T>::transfer_from(&lended_token, &user, &Self::account_id(), lended_amount)?;

            // Emit an event
            Self::deposit_event(Event::AssetsLended(user, lended_token, lended_amount));

            Ok(().into())
        }


        /// Borrow tokens from the pool

        #[pallet::call_index(2)]
        #[pallet::weight(10000)]
        #[transactional]
        pub fn borrow(
            origin: OriginFor<T>,
            borrowed_token: AssetIdOf<T>,
            borrowed_amount: Balance,
            collateral: Balance
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let user_info = UserInfo::<T>::get(&user);
            let mut pool = PoolInfo::<T>::get(&borrowed_token);
            // Check if the pool is lending
            ensure!(pool.is_lending, Error::<T>::LendingUnavailable);
            // Check if there is enough tokens in pool to borrow
            ensure!(pool.pool_balance < borrowed_amount, Error::<T>::NotEnoughTokensInPool);
            // Check if the collateral right amount of tokens
            ensure!(
                borrowed_amount == collateral * balance!(0.75),
                Error::<T>::InsufficientCollateral
            );
            // Check if borrower has enough tokens for collateral
            ensure!(
                Assets::<T>::free_balance(&borrowed_token, &user).unwrap_or(0) >= collateral,
                Error::<T>::InsufficientFunds
            );

            // Deduct borrowed tokens from the pool
            pool.pool_balance -= borrowed_amount;

            // Check if user exists, if not, create a new user
            if let Some(mut user_info) = user_info {
                user_info.borrowed_amount += borrowed_amount;
                user_info.collateral += collateral;
                user_info.debt_interest += Self::calculate_interest(&pool.borrowing_interest, &user_info.borrowed_amount, &user_info.last_time_borrowed);
                user_info.last_time_borrowed = <frame_system::Pallet<T>>::block_number();
            } else {
                let new_user_info = User {
                    borrowed_token: borrowed_token,
                    borrowed_amount,
                    collateral,
                    last_time_borrowed: <frame_system::Pallet<T>>::block_number(),
                    ..Default::default()
                };

                // Add new user to the storage
                <UserInfo<T>>::insert(&user, new_user_info);
            }

            // Transfer collateral from user to pool
            Assets::<T>::transfer_from(&borrowed_token, &user, &Self::account_id(), collateral)?;

            // Transfer tokens from pool to user
            Assets::<T>::transfer_from(
                &borrowed_token,
                &Self::account_id(),
                &user,
                borrowed_amount
            )?;

            // Emit an event
            Self::deposit_event(
                Event::AssetsBorrowed(user, borrowed_token, borrowed_amount, collateral)
            );

            Ok(().into())
        }

        /// Repay debt

        #[pallet::call_index(3)]
        #[pallet::weight(10000)]
        #[transactional]
        pub fn repay(origin: OriginFor<T>,
             borrowed_token: AssetIdOf<T>,
             repay_amount: Balance
            ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let user_info = <UserInfo<T>>::get(&user);
            let pool_info = PoolInfo::<T>::get(&borrowed_token);
            let current_block = frame_system::Pallet::<T>::block_number();
            let borrowing_interest = pool_info.borrowing_interest;
            
            // Check if borrowed is repaying the valid asset
            ensure!(pool_info.asset_id == borrowed_token, Error::<T>::InvalidAsset);

            if let Some(mut user_info) = user_info {
                // Calculate the interest for the debt 
                let last_interest = Self::calculate_interest(&borrowing_interest, &user_info.debt_interest, &user_info.last_time_borrowed);
                // Add that interest to the interest debt
                let debt_interest = user_info.debt_interest + last_interest;

                // Check if repay amount is less or equal to borrowed amount + interest
                ensure!(repay_amount <= user_info.borrowed_amount + debt_interest, Error::<T>::ExcessiveAmount);

                // If repay amount = borrowed amount + debt interest, the debt is paid in full
                if repay_amount == user_info.borrowed_amount + debt_interest {
                    // Transfer tokens from user to platform. Debt repayed
                    Assets::<T>::transfer_from(&borrowed_token, &user, &Self::account_id(), repay_amount)?;
                    // Transfer collateral from platform back to the user
                    Assets::<T>::transfer_from(&borrowed_token, &Self::account_id(), &user, user_info.collateral)?;
                    // Set borrow-related fields to default
                    user_info.collateral = 0;
                    user_info.borrowed_amount = 0;
                    user_info.debt_interest = 0;
                    user_info.debt_paid = true;
                    // Emit an event
                    Self::deposit_event(Event::DebtFullyRepaid(user, borrowed_token, repay_amount));
                } else {
                    // Transfer repayed amount from user to the platform
                    Assets::<T>::transfer_from(&borrowed_token, &user, &Self::account_id(), repay_amount)?;
                    // Deduct repayed amount from borrowed amount 
                    user_info.borrowed_amount -= repay_amount;
                    // Set current block as the new block from which the interest will be calculated based on amount left on the platform
                    user_info.last_time_borrowed = current_block;
                    // Emit an event
                    Self::deposit_event(Event::DebtPartiallyRepaid(user, borrowed_token, repay_amount));
                }
            } else {
                return Err(Error::<T>::UserDoesNotExist.into());
            }

            Ok(().into())
        }


        /// Withdraw lended tokens

        #[pallet::call_index(5)]
        #[pallet::weight(10000)]
        pub fn withdraw(origin: OriginFor<T>, lended_token: AssetIdOf<T>, withdraw_amount: Balance) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let user_info = UserInfo::<T>::get(&user);
            let pool_info = PoolInfo::<T>::get(&lended_token);
            let current_block = frame_system::Pallet::<T>::block_number();
            let lending_interest = pool_info.lending_interest;

            // Check if Lender is withdrawing the valid asset
            ensure!(pool_info.asset_id == lended_token, Error::<T>::InvalidAsset);

            // If user withdraws whole amount lended, full interest earned will be withdrawn
            // If user withdraws partial amount lended, from this moment on, the interest will be calculated based on the amount left on the platform
            if let Some(mut user_info) = user_info {
                // Check if amout for withdrawing is less or equal to lended amount
                ensure!(withdraw_amount <= user_info.lended_amount, Error::<T>::ExcessiveAmount);

                if withdraw_amount == user_info.lended_amount {
                    // Calculate the interest on the amount ledned in the pool
                    let last_interest = Self::calculate_interest(&lending_interest, &user_info.lended_amount, &user_info.last_time_lended);
                    // Add that interest to the total interest earned 
                    user_info.interest_earned += last_interest;
                    // Transfer withdrawn tokens(fully lended amount + the interest earned) from the pool to the lender
                    Assets::<T>::transfer_from(&lended_token, &Self::account_id(), &user, withdraw_amount + user_info.interest_earned)?;
                    // Set lended amount and interest earned on zero
                    user_info.lended_amount = 0;
                    user_info.interest_earned = 0;
                    // Emit an event
                    Self::deposit_event(Event::FullAmountWithdrawn(user, lended_token, withdraw_amount, user_info.interest_earned));
                } else {
                    // Transfer amount of lended tokens user wants to withdraw from platform
                    Assets::<T>::transfer_from(&lended_token, &Self::account_id(), &user, withdraw_amount)?;
                    // Deduct withdrawn amount from the lended amount
                    user_info.lended_amount -= withdraw_amount;
                    // Set current block as the new block from which the interest will be calculated based on amount left on the platform
                    user_info.last_time_lended = current_block;
                    // Emit an event 
                    Self::deposit_event(Event::AmountWithdrawn(user, lended_token, withdraw_amount));
                }
            } else {
                return Err(Error::<T>::UserDoesNotExist.into());
            }

            Ok(().into())
        }

     }


     impl<T: Config> Pallet<T> {
        /// The account ID of pallet
        fn account_id() -> T::AccountId {
            PALLET_ID.into_account_truncating()
        }

        fn calculate_interest(interest: &Balance, amount: &Balance, last_time: &BlockNumber<T>) -> Balance {
            let interest_per_block = interest / 432000;
            let current_block = <frame_system::Pallet<T>>::block_number();
            let block_difference: u128 = (current_block - *last_time).unique_saturated_into();
            amount * (block_difference * interest_per_block)
        } 
    }
}


