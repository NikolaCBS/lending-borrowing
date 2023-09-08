#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod benchmarking;
pub mod weights;

use codec::{Decode, Encode};
use common::Balance;
pub use weights::WeightInfo;

#[derive(Encode, Decode, Default, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Pool<AssetId> {
    // Asset pool is using
    asset_id: AssetId,
    // Total balance of the pool
    pool_balance: Balance,
    // Lending interest per block
    lending_interest: Balance,
    // Borrowing interest per block
    borrowing_interest: Balance,
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
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

    // Pallet imports
    use crate::{Pool, User, WeightInfo};
    use common::balance;
    use common::prelude::{AssetInfoProvider, Balance, FixedWrapper};
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
    use frame_support::traits::Hooks;
    use frame_support::transactional;
    use frame_support::PalletId;
    use frame_system::pallet_prelude::*;
    use hex_literal::hex;

    /// Pallet ID
    const PALLET_ID: PalletId = PalletId(*b"tplndbrw");

    #[pallet::config]
    pub trait Config: frame_system::Config + assets::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
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
        /// Pool Created [asset, lending_interest, borrowing_interest]
        PoolCreated(AssetIdOf<T>, Balance, Balance),
        /// Assets Lended [who, asset, amount]
        AssetsLended(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// Assets Borrowed [who, asset, borrowed_amount, collateral]
        AssetsBorrowed(AccountIdOf<T>, AssetIdOf<T>, Balance, Balance),
        /// Amount withdrawn
        AmountWithdrawn(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// Full Lending Amount Withdrawn [who, asset, amount]
        FullAmountWithdrawn(AccountIdOf<T>, AssetIdOf<T>, Balance, Balance),
        /// Debt repaid in full [who, asset, amount]
        DebtFullyRepaid(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// Debt partially repaid [who, asset, amount]
        DebtPartiallyRepaid(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User liquidated [user, borrowed_amount, debt_interest, collateral]
        UserLiquidated(AccountIdOf<T>, Balance, Balance, Balance),
    }

    /// Errors
    #[pallet::error]
    pub enum Error<T> {
        /// Pool is already created
        PoolAlreadyCreated,
        /// Unauthorized
        Unauthorized,
        /// Insufficient funds
        InsufficientFunds,
        /// Pool does not exist
        PoolDoesNotExist,
        /// Invalid asset
        InvalidAsset,
        /// Invalid interest proportion
        InvalidInterestProportion,
        /// Insufficient funds in pool
        NotEnoughTokensInPool,
        /// Insufficient collateral
        InadequateCollateral,
        /// Excessive amount
        ExcessiveAmount,
        ///No debt to repay
        NoDebtToRepay,
        // No tokens lended
        NoTokensLended,
        /// Repay fully or part of principal
        RepayFullyOrPartOfPrincipal,
        /// Non-Existing user
        UserDoesNotExist,
    }

    /// Pool info storage
    #[pallet::storage]
    #[pallet::getter(fn pool_data)]
    pub type PoolInfo<T: Config> =
        StorageMap<_, Identity, AssetIdOf<T>, Pool<AssetIdOf<T>>, ValueQuery>;

    /// User info storage
    #[pallet::storage]
    #[pallet::getter(fn user_data)]
    pub type UserInfo<T: Config> =
        StorageMap<_, Identity, AccountIdOf<T>, User<BlockNumberFor<T>, AssetIdOf<T>>, OptionQuery>;

    #[pallet::type_value]
    pub fn DefaultForAuthorityAccount<T: Config>() -> AccountIdOf<T> {
        let bytes = hex!("6a6a3e59a8f514aa3521e23058f4571bc720d7c202ac5ced01dbac5874a10335");
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
        #[pallet::weight(<T as Config>::WeightInfo::create_pool())]
        pub fn create_pool(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            lending_interest: Balance,
            borrowing_interest: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            // Check if pool is already created
            if <PoolInfo<T>>::contains_key(&asset_id) {
                return Err(Error::<T>::PoolAlreadyCreated.into());
            }

            // Check if authority account is calling the function
            if user != AuthorityAccount::<T>::get() {
                return Err(Error::<T>::Unauthorized.into());
            }

            // Check if ledning and borrowing interests are in the right proportion
            ensure!(
                lending_interest
                    == (FixedWrapper::from(borrowing_interest) * FixedWrapper::from(balance!(0.7)))
                        .try_into_balance()
                        .unwrap_or(0),
                Error::<T>::InvalidInterestProportion
            );

            // Create pool
            let pool = Pool {
                asset_id,
                pool_balance: 0,
                lending_interest: (FixedWrapper::from(lending_interest)
                    / FixedWrapper::from(balance!(5256000)))
                .try_into_balance()
                .unwrap_or(0),
                borrowing_interest: (FixedWrapper::from(borrowing_interest)
                    / FixedWrapper::from(balance!(5256000)))
                .try_into_balance()
                .unwrap_or(0),
            };

            // Add pool to the storage
            <PoolInfo<T>>::insert(asset_id, pool);

            // Emit an event
            Self::deposit_event(Event::PoolCreated(
                asset_id,
                lending_interest,
                borrowing_interest,
            ));

            Ok(().into())
        }

        /// Lend tokens to the pool

        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::lend())]
        #[transactional]
        pub fn lend(
            origin: OriginFor<T>,
            lended_token: AssetIdOf<T>,
            lended_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            // Check if pool with this token exists
            if !<PoolInfo<T>>::contains_key(&lended_token) {
                return Err(Error::<T>::PoolDoesNotExist.into());
            }

            // Check if user has enough funds to lend to the pool
            ensure!(
                Assets::<T>::free_balance(&lended_token, &user).unwrap_or(0) >= lended_amount,
                Error::<T>::InsufficientFunds
            );

            let mut pool = <PoolInfo<T>>::get(&lended_token);

            let user_info = <UserInfo<T>>::get(&user);
            let current_block = frame_system::Pallet::<T>::block_number();

            // Add the lended amount to existing amount (if user exists)
            // Create new user with lended amount(if user doesn't exist
            if let Some(mut user_info) = user_info {
                let new_interest = Self::calculate_interest(
                    pool.lending_interest,
                    user_info.lended_amount,
                    user_info.last_time_lended,
                );
                user_info.interest_earned += new_interest;
                user_info.last_time_lended = current_block;
                user_info.lended_amount += lended_amount;
                <UserInfo<T>>::insert(&user, user_info);
            } else {
                let new_user_info = User {
                    lended_token,
                    lended_amount,
                    last_time_lended: current_block,
                    ..Default::default()
                };

                <UserInfo<T>>::insert(&user, new_user_info);
            }

            // Add lended tokens to the pool
            pool.pool_balance += lended_amount;
            <PoolInfo<T>>::insert(lended_token, pool);

            // Transfer assets from user to the pool
            Assets::<T>::transfer_from(&lended_token, &user, &Self::account_id(), lended_amount)?;

            // Emit an event
            Self::deposit_event(Event::AssetsLended(user, lended_token, lended_amount));

            Ok(().into())
        }

        /// Borrow tokens from the pool

        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::borrow())]
        #[transactional]
        pub fn borrow(
            origin: OriginFor<T>,
            borrowed_token: AssetIdOf<T>,
            borrowed_amount: Balance,
            collateral: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let mut pool = PoolInfo::<T>::get(&borrowed_token);

            // Check if there is enough tokens in pool to borrow
            ensure!(
                pool.pool_balance > borrowed_amount,
                Error::<T>::NotEnoughTokensInPool
            );
            // Check if the collateral right amount of tokens
            ensure!(
                borrowed_amount
                    == (FixedWrapper::from(collateral) * FixedWrapper::from(balance!(0.75)))
                        .try_into_balance()
                        .unwrap_or(0),
                Error::<T>::InadequateCollateral
            );
            // Check if borrower has enough tokens for collateral
            ensure!(
                Assets::<T>::free_balance(&borrowed_token, &user).unwrap_or(0) >= collateral,
                Error::<T>::InsufficientFunds
            );

            let user_info = UserInfo::<T>::get(&user);

            // Check if user exists, if not, create a new user
            if let Some(mut user_info) = user_info {
                user_info.collateral += collateral;
                user_info.debt_interest += Self::calculate_interest(
                    pool.borrowing_interest,
                    user_info.borrowed_amount,
                    user_info.last_time_borrowed,
                );
                user_info.borrowed_amount += borrowed_amount;
                user_info.last_time_borrowed = <frame_system::Pallet<T>>::block_number();
                <UserInfo<T>>::insert(&user, user_info)
            } else {
                let new_user_info = User {
                    borrowed_token,
                    borrowed_amount,
                    collateral,
                    last_time_borrowed: <frame_system::Pallet<T>>::block_number(),
                    ..Default::default()
                };

                // Add new user to the storage
                <UserInfo<T>>::insert(&user, new_user_info);
            }

            // Deduct borrowed tokens from the pool
            pool.pool_balance -= borrowed_amount;

            // Add collateral to the pool balance
            pool.pool_balance += collateral;

            <PoolInfo<T>>::insert(&borrowed_token, pool);
            // Transfer collateral from user to pool
            Assets::<T>::transfer_from(&borrowed_token, &user, &Self::account_id(), collateral)?;

            // Transfer tokens from pool to user
            Assets::<T>::transfer_from(
                &borrowed_token,
                &Self::account_id(),
                &user,
                borrowed_amount,
            )?;

            // Emit an event
            Self::deposit_event(Event::AssetsBorrowed(
                user,
                borrowed_token,
                borrowed_amount,
                collateral,
            ));

            Ok(().into())
        }

        /// Repay debt

        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::repay())]
        #[transactional]
        pub fn repay(
            origin: OriginFor<T>,
            borrowed_token: AssetIdOf<T>,
            repay_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let user_info = <UserInfo<T>>::get(&user);

            let mut pool_info = PoolInfo::<T>::get(&borrowed_token);
            let current_block = frame_system::Pallet::<T>::block_number();
            let borrowing_interest = pool_info.borrowing_interest;

            if let Some(mut user_info) = user_info {
                ensure!(user_info.borrowed_amount > 0, Error::<T>::NoDebtToRepay);

                // Calculate the interest for the debt
                let last_interest = Self::calculate_interest(
                    borrowing_interest,
                    user_info.borrowed_amount,
                    user_info.last_time_borrowed,
                );
                // Add that interest to the interest debt
                let debt_interest = user_info.debt_interest + last_interest;

                // Check if repay amount is greater than borrowed amount + interest
                ensure!(
                    repay_amount <= user_info.borrowed_amount + debt_interest,
                    Error::<T>::ExcessiveAmount
                );

                // If repay amount = borrowed amount + debt interest, the debt is paid in full
                if repay_amount == user_info.borrowed_amount + debt_interest {
                    // Transfer tokens from user to platform. Debt repayed
                    Assets::<T>::transfer_from(
                        &borrowed_token,
                        &user,
                        &Self::account_id(),
                        repay_amount,
                    )?;
                    // Transfer collateral from platform back to the user
                    Assets::<T>::transfer_from(
                        &borrowed_token,
                        &Self::account_id(),
                        &user,
                        user_info.collateral,
                    )?;
                    // Set pool related fields
                    pool_info.pool_balance += repay_amount;
                    pool_info.pool_balance -= user_info.collateral;
                    <PoolInfo<T>>::insert(&borrowed_token, pool_info);
                    // Set borrow-related fields to default
                    user_info.collateral = 0;
                    user_info.borrowed_amount = 0;
                    user_info.debt_interest = 0;
                    <UserInfo<T>>::insert(&user, user_info);
                    // Emit an event
                    Self::deposit_event(Event::DebtFullyRepaid(user, borrowed_token, repay_amount));
                } else if repay_amount < user_info.borrowed_amount {
                    // Transfer repayed amount from user to the platform
                    Assets::<T>::transfer_from(
                        &borrowed_token,
                        &user,
                        &Self::account_id(),
                        repay_amount,
                    )?;
                    // Add the interest debt to this moment to the total interest debt
                    user_info.debt_interest += Self::calculate_interest(
                        pool_info.borrowing_interest,
                        user_info.borrowed_amount,
                        user_info.last_time_borrowed,
                    );
                    // Deduct repayed amount from borrowed amount
                    user_info.borrowed_amount -= repay_amount;
                    // Set current block as the new block from which the interest will be calculated based on amount left on the platform
                    user_info.last_time_borrowed = current_block;

                    <UserInfo<T>>::insert(&user, user_info);
                    // Add repaid amount to the pool
                    pool_info.pool_balance += repay_amount;
                    <PoolInfo<T>>::insert(&borrowed_token, pool_info);
                    // Emit an event
                    Self::deposit_event(Event::DebtPartiallyRepaid(
                        user,
                        borrowed_token,
                        repay_amount,
                    ));
                } else {
                    return Err(Error::<T>::RepayFullyOrPartOfPrincipal.into());
                }
            } else {
                return Err(Error::<T>::UserDoesNotExist.into());
            }

            Ok(().into())
        }

        /// Withdraw lended tokens

        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::withdraw())]
        pub fn withdraw(
            origin: OriginFor<T>,
            lended_token: AssetIdOf<T>,
            withdraw_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;
            let user_info = UserInfo::<T>::get(&user);
            let mut pool_info = PoolInfo::<T>::get(&lended_token);

            ensure!(pool_info.asset_id == lended_token, Error::<T>::InvalidAsset);

            let current_block = frame_system::Pallet::<T>::block_number();
            let lending_interest = pool_info.lending_interest;

            // If user withdraws whole amount lended, full interest earned will be withdrawn
            // If user withdraws partial amount lended, from this moment on, the interest will be calculated based on the amount left on the platform
            if let Some(mut user_info) = user_info {
                ensure!(user_info.lended_amount > 0, Error::<T>::NoTokensLended);

                ensure!(
                    withdraw_amount <= user_info.lended_amount,
                    Error::<T>::ExcessiveAmount
                );

                if withdraw_amount == user_info.lended_amount {
                    // Calculate the interest on the amount ledned in the pool
                    let last_interest = Self::calculate_interest(
                        lending_interest,
                        user_info.lended_amount,
                        user_info.last_time_lended,
                    );
                    // Add that interest to the total interest earned
                    user_info.interest_earned += last_interest;
                    // Transfer withdrawn tokens(fully lended amount + the interest earned) from the pool to the lender
                    Assets::<T>::transfer_from(
                        &lended_token,
                        &Self::account_id(),
                        &user,
                        withdraw_amount + user_info.interest_earned,
                    )?;
                    // Set lended amount and interest earned on zero
                    user_info.lended_amount = 0;
                    user_info.interest_earned = 0;
                    <UserInfo<T>>::insert(&user, &user_info);
                    pool_info.pool_balance -= withdraw_amount;
                    <PoolInfo<T>>::insert(&lended_token, pool_info);
                    // Emit an event
                    Self::deposit_event(Event::FullAmountWithdrawn(
                        user,
                        lended_token,
                        withdraw_amount,
                        user_info.interest_earned,
                    ));
                } else {
                    // Transfer amount of lended tokens user wants to withdraw from platform
                    Assets::<T>::transfer_from(
                        &lended_token,
                        &Self::account_id(),
                        &user,
                        withdraw_amount,
                    )?;
                    // Add earned interest to this moment to the total earned interest
                    user_info.interest_earned += Self::calculate_interest(
                        pool_info.lending_interest,
                        user_info.lended_amount,
                        user_info.last_time_lended,
                    );
                    // Deduct withdrawn amount from the lended amount
                    user_info.lended_amount -= withdraw_amount;
                    // Set current block as the new block from which the interest will be calculated based on amount left on the platform
                    user_info.last_time_lended = current_block;
                    <UserInfo<T>>::insert(&user, user_info);
                    // Deduct withdrawn amount from the pool's balance
                    pool_info.pool_balance -= withdraw_amount;
                    <PoolInfo<T>>::insert(&lended_token, pool_info);
                    // Emit an event
                    Self::deposit_event(Event::AmountWithdrawn(
                        user,
                        lended_token,
                        withdraw_amount,
                    ));
                }
            } else {
                return Err(Error::<T>::UserDoesNotExist.into());
            }

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let counter = Self::check_debt(now);
            counter
        }
    }

    impl<T: Config> Pallet<T> {
        /// The account ID of pallet
        fn account_id() -> T::AccountId {
            PALLET_ID.into_account_truncating()
        }

        /// Calculate amount of interest
        pub fn calculate_interest(
            interest: Balance,
            amount: Balance,
            last_time: BlockNumber<T>,
        ) -> Balance {
            let current_block = <frame_system::Pallet<T>>::block_number();
            let block_difference: u128 = (current_block - last_time).unique_saturated_into();
            (FixedWrapper::from(interest) * FixedWrapper::from(amount))
                .try_into_balance()
                .unwrap_or(0)
                * block_difference
        }
        /// Check if debt has surpassed collateral amount
        fn check_debt(_current_block: T::BlockNumber) -> Weight {
            let mut counter: u64 = 0;

            for (account_id, mut user_info) in UserInfo::<T>::iter() {
                let pool_info = PoolInfo::<T>::get(user_info.borrowed_token);
                let debt = user_info.borrowed_amount
                    + user_info.debt_interest
                    + Self::calculate_interest(
                        pool_info.borrowing_interest,
                        user_info.borrowed_amount,
                        user_info.last_time_borrowed,
                    );

                if FixedWrapper::from(debt) > FixedWrapper::from(user_info.collateral) {
                    user_info.borrowed_amount = 0;
                    user_info.debt_interest = 0;
                    user_info.collateral = 0;
                    UserInfo::<T>::insert(&account_id, &user_info);
                    counter += 1;

                    Self::deposit_event(Event::UserLiquidated(
                        account_id,
                        user_info.borrowed_amount,
                        user_info.debt_interest,
                        user_info.collateral,
                    ));
                }
            }

            T::DbWeight::get()
                .reads(counter)
                .saturating_add(T::DbWeight::get().writes(counter))
        }
    }
}
