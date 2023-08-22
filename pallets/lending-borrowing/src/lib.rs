#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    /// Pallet imports
    use common::{balance, AssetInfoProvider, Balance};
    use frame_support::pallet_prelude::DispatchResultWithPostInfo;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::AccountIdConversion;
    use frame_support::transactional;
    use frame_support::PalletId;
    use frame_system::pallet_prelude::*;
    use hex_literal::hex;
    use sp_runtime::traits::UniqueSaturatedInto;
    use sp_std::prelude::*;

    /// Pallet id
    const PALLET_ID: PalletId = PalletId(*b"lendborw");

    /// Aliasing needed types
    type Assets<T> = assets::Pallet<T>;
    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
    pub type AssetIdOf<T> = <T as assets::Config>::AssetId;
    pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;

    //  Defining needed structs
    /// UserInfo struct
    #[derive(Encode, Decode, PartialEq, Eq, scale_info::TypeInfo, Default)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct UserInfo<BlockNumber> {
        //// Info for lending

        // Amount that was lended
        pub lending_amount: Balance,
        // Amount earned from lending (lending_amount * interest)
        pub lending_earnings: Balance,
        // Block number from which lending_earnings is calculated
        pub lending_start_block: BlockNumber,

        //// Info for borrowing

        // Amount that was borrowed
        pub borrowed_amount: Balance,
        // Amount that was used as collateral
        pub collateral_amount: Balance,
        // Debt that was acumulated (borrowed_amount * interest)
        pub acumulated_debt: Balance,
        // Block number from which acumulated_debt is calculated
        pub borrow_start_block: BlockNumber,
    }

    /// PoolInfo struct
    #[derive(Encode, Decode, Default, PartialEq, Eq, scale_info::TypeInfo)]
    #[cfg_attr(feature = "std", derive(Debug))]
    pub struct PoolInfo<AssetId> {
        pub asset_id: AssetId, // Asset ID of token that is being used for ledning/borrowing
        pub balance: Balance,  // Pool token balance
        pub lending_rate: Balance, // Interest rate for lending (used to calculate lending_earnings)
        pub borrowed_rate: Balance, // Interest rate for borrowing (used to calculate acumulated_debt)
        pub collateral_factor: Balance, // Collateral factor (used to calculate collateral_amount)
    }

    //  Pallet definition
    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    //  Pallet configuration
    #[pallet::config]
    pub trait Config: frame_system::Config + assets::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    //  Defining needed types
    /// Defining value for authority account (can create pool)
    #[pallet::type_value]
    pub fn DefaultForAuthorityAccount<T: Config>() -> AccountIdOf<T> {
        let bytes = hex!("96ea3c9c0be7bbc7b0656a1983db5eed75210256891a9609012362e36815b132"); // <--- Set public key of authority account
        AccountIdOf::<T>::decode(&mut &bytes[..]).unwrap()
    }

    //  Defining pallet storage
    /// Pools   AssetID -> PoolInfo
    #[pallet::storage]
    pub type Pools<T: Config> =
        StorageMap<_, Identity, AssetIdOf<T>, PoolInfo<AssetIdOf<T>>, ValueQuery>;

    /// PoolUsers   PoolInfo -> Vec<UserInfo>
    #[pallet::storage]
    pub type PoolUsers<T: Config> = StorageDoubleMap<
        _,
        Identity,
        AssetIdOf<T>,
        Identity,
        AccountIdOf<T>,
        UserInfo<BlockNumberFor<T>>,
        ValueQuery,
    >;

    /// Authority account storage
    #[pallet::storage]
    #[pallet::getter(fn authority_account)]
    pub type AuthorityAccount<T: Config> =
        StorageValue<_, AccountIdOf<T>, ValueQuery, DefaultForAuthorityAccount<T>>;

    //  Defining pallet events
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Pool created successfully [who, assetId]
        PoolCreated(AccountIdOf<T>, AssetIdOf<T>),
        /// New user lended a specific amount of tokens [who, assetId, amount]
        NewLendingUser(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User lended additional tokens [who, assetId, amount]
        UserLendedAdditionalTokens(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// New user has borrowed a specific amount of tokens [who, assetId, amount]
        NewBorrowingUser(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User borrowed additional tokens [who, assetId, amount]
        UserBorrowedAdditionalTokens(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User withdrew lended tokens [who, assetId, amount]
        UserWithdrewLendedTokens(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User fully returned borrowed debt [who, assetId, amount]
        UserFullyReturnedBorrowedTokens(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User fully payed off debt and a part of the borrowed tokens [who, assetId, amount]
        UserFullyPayedOffDebtAndPartOfBorrowed(AccountIdOf<T>, AssetIdOf<T>, Balance),
        /// User payed off part of his debt [who, assetId, amount]
        UserPayedPartOfDebt(AccountIdOf<T>, AssetIdOf<T>, Balance),
    }

    //  Defining pallet errors
    #[pallet::error]
    pub enum Error<T> {
        /// Unauthorized account used for pool creation
        UnauthorizedPoolCreation,
        /// Pool already exists
        PoolAlreadyExists,
        /// Pool doesn't exist
        PoolDoesntExist,
        /// Not enough funds to performe transaction
        InsufficientFunds,
        /// User doesn't exist on given asset pool
        UserDoesntExist,
        /// User hasn't lended any tokens
        UserHasntLendedTokens,
        /// User hasn't payed off his debts
        UserHasntPayedDebts,
        /// Pool doesn't have enough tokens
        InsufficientFundsOnPool,
    }

    //  Defining pallet calls
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(0)]
        pub fn create_poll(
            origin: OriginFor<T>,
            _asset_id: AssetIdOf<T>,
            _lending_rate: Balance,
            _borrowed_rate: Balance,
            _collateral_factor: Balance,
        ) -> DispatchResultWithPostInfo {
            let user_id = ensure_signed(origin)?;

            // Check if the user is authorized to create a lending/borrowing pool
            ensure!(
                user_id == AuthorityAccount::<T>::get(),
                Error::<T>::UnauthorizedPoolCreation
            );

            // Check if pool already exists
            ensure!(
                !Pools::<T>::contains_key(&_asset_id),
                Error::<T>::PoolAlreadyExists
            );

            // New lending/borrowing pool structure
            let new_pool = PoolInfo {
                asset_id: _asset_id,
                balance: 0,
                lending_rate: _lending_rate,
                borrowed_rate: _borrowed_rate,
                collateral_factor: _collateral_factor,
            };

            // Save new lending/borrowing pool
            Pools::<T>::set(_asset_id, new_pool);

            // Depositing event
            Self::deposit_event(Event::PoolCreated(user_id, _asset_id));

            Ok(().into())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(0)]
        pub fn lend_tokens(
            origin: OriginFor<T>,
            _asset_id: AssetIdOf<T>,
            _amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user_id = ensure_signed(origin)?;

            // Check if pool exists
            ensure!(
                Pools::<T>::contains_key(&_asset_id),
                Error::<T>::PoolDoesntExist
            );

            // Check if user has enough assets on account
            ensure!(
                Assets::<T>::free_balance(&_asset_id, &user_id).unwrap_or(0) >= _amount,
                Error::<T>::InsufficientFunds
            );

            // Get pool info
            let pool_info = Pools::<T>::get(&_asset_id);

            // Get current block
            let current_block = frame_system::Pallet::<T>::block_number();

            // Check if user is present
            if PoolUsers::<T>::contains_key(&_asset_id, &user_id) {
                // Get users info
                let user_info = PoolUsers::<T>::get(&_asset_id, &user_id);

                // Check if user lended tokens
                if user_info.lending_amount > 0 {
                    // Calculate block difference
                    let block_difference: u128 =
                        (current_block - user_info.lending_start_block).unique_saturated_into();

                    // Calculate earnings
                    let earnings = (block_difference * (pool_info.lending_rate / 432000))
                        * user_info.lending_amount;

                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: user_info.lending_amount + _amount,
                        lending_earnings: user_info.lending_earnings + earnings,
                        lending_start_block: current_block,
                        borrowed_amount: user_info.borrowed_amount,
                        acumulated_debt: user_info.acumulated_debt,
                        collateral_amount: user_info.collateral_amount,
                        borrow_start_block: user_info.borrow_start_block,
                    };

                    // Update pool info
                    let updated_pool_info = PoolInfo {
                        asset_id: pool_info.asset_id,
                        balance: pool_info.balance + _amount,
                        borrowed_rate: pool_info.borrowed_rate,
                        lending_rate: pool_info.lending_rate,
                        collateral_factor: pool_info.collateral_factor,
                    };

                    // Transfer tokens -> from user to pool
                    Assets::<T>::transfer_from(&_asset_id, &user_id, &Self::account_id(), _amount);

                    // Save updated user data
                    PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                    // Save updated pool data
                    Pools::<T>::set(_asset_id, updated_pool_info);

                    // Depositing event
                    Self::deposit_event(Event::UserLendedAdditionalTokens(
                        user_id, _asset_id, _amount,
                    ))
                } else {
                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: _amount,
                        lending_earnings: 0,
                        lending_start_block: current_block,
                        borrowed_amount: user_info.borrowed_amount,
                        acumulated_debt: user_info.acumulated_debt,
                        collateral_amount: user_info.collateral_amount,
                        borrow_start_block: user_info.borrow_start_block,
                    };

                    // Update pool info
                    let updated_pool_info = PoolInfo {
                        asset_id: pool_info.asset_id,
                        balance: pool_info.balance + _amount,
                        borrowed_rate: pool_info.borrowed_rate,
                        lending_rate: pool_info.lending_rate,
                        collateral_factor: pool_info.collateral_factor,
                    };

                    // Transfer tokens -> from user to pool
                    Assets::<T>::transfer_from(&_asset_id, &user_id, &Self::account_id(), _amount);

                    // Save updated user data
                    PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                    // Save updated pool data
                    Pools::<T>::set(_asset_id, updated_pool_info);

                    // Depositing event
                    Self::deposit_event(Event::NewLendingUser(user_id, _asset_id, _amount));
                }
            } else {
                // Creating new user
                let new_user_info = UserInfo {
                    lending_amount: _amount,
                    lending_earnings: 0,
                    lending_start_block: current_block,
                    borrowed_amount: 0,
                    collateral_amount: 0,
                    acumulated_debt: 0,
                    borrow_start_block: current_block,
                };

                // Update pool info
                let updated_pool_info = PoolInfo {
                    asset_id: pool_info.asset_id,
                    balance: pool_info.balance + _amount,
                    borrowed_rate: pool_info.borrowed_rate,
                    lending_rate: pool_info.lending_rate,
                    collateral_factor: pool_info.collateral_factor,
                };

                // Transfer tokens -> from user to pool
                Assets::<T>::transfer_from(&_asset_id, &user_id, &Self::account_id(), _amount);

                // Save new user data
                PoolUsers::<T>::set(_asset_id, &user_id, new_user_info);

                // Save updated pool data
                Pools::<T>::set(_asset_id, updated_pool_info);

                // Depositing event
                Self::deposit_event(Event::NewLendingUser(user_id, _asset_id, _amount));
            }

            Ok(().into())
        }

        #[transactional]
        #[pallet::call_index(2)]
        #[pallet::weight(0)]
        pub fn borrow_tokens(
            origin: OriginFor<T>,
            _asset_id: AssetIdOf<T>,
            _amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user_id = ensure_signed(origin)?;

            // Check if pool exists
            ensure!(
                Pools::<T>::contains_key(&_asset_id),
                Error::<T>::PoolDoesntExist
            );

            // Get pool info
            let pool_info = Pools::<T>::get(&_asset_id);

            // Check if pool has enough tokens
            ensure!(
                pool_info.balance >= _amount,
                Error::<T>::InsufficientFundsOnPool
            );

            // Check if user can pay colateral amount
            ensure!(
                Assets::<T>::free_balance(&_asset_id, &user_id).unwrap_or(0)
                    >= _amount * balance!(pool_info.collateral_factor),
                Error::<T>::InsufficientFunds
            );

            // Get current block
            let current_block = frame_system::Pallet::<T>::block_number();

            // Check if user is present
            if PoolUsers::<T>::contains_key(&_asset_id, &user_id) {
                // Get user info
                let user_info = PoolUsers::<T>::get(&_asset_id, &user_id);

                // Check if user has borrowed tokens
                if user_info.borrowed_amount > 0 {
                    // Calculate block difference
                    let block_difference: u128 =
                        (current_block - user_info.borrow_start_block).unique_saturated_into();

                    // Calculate debt
                    let debt = (block_difference * (pool_info.borrowed_rate / 432000))
                        * user_info.borrowed_amount;

                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: user_info.lending_amount,
                        lending_earnings: user_info.lending_earnings,
                        lending_start_block: user_info.lending_start_block,
                        borrowed_amount: user_info.borrowed_amount + _amount,
                        collateral_amount: user_info.collateral_amount
                            + (_amount * balance!(pool_info.collateral_factor)),
                        acumulated_debt: user_info.acumulated_debt + debt,
                        borrow_start_block: current_block,
                    };

                    // Update pool info
                    let updated_pool_info = PoolInfo {
                        asset_id: pool_info.asset_id,
                        balance: pool_info.balance - _amount,
                        lending_rate: pool_info.lending_rate,
                        borrowed_rate: pool_info.borrowed_rate,
                        collateral_factor: pool_info.collateral_factor,
                    };

                    // Transfer tokens -> From pool to user (borrowed_amount)
                    Assets::<T>::transfer_from(&_asset_id, &Self::account_id(), &user_id, _amount);
                    // Transfer tokens -> From user to pool (collateral_amount)
                    Assets::<T>::transfer_from(
                        &_asset_id,
                        &user_id,
                        &Self::account_id(),
                        updated_user_info.collateral_amount,
                    );

                    // Save updated user info
                    PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                    // Save updated pool info
                    Pools::<T>::set(_asset_id, updated_pool_info);

                    // Depositing event
                    Self::deposit_event(Event::UserBorrowedAdditionalTokens(
                        user_id, _asset_id, _amount,
                    ))
                } else {
                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: user_info.lending_amount,
                        lending_earnings: user_info.lending_earnings,
                        lending_start_block: user_info.lending_start_block,
                        borrowed_amount: _amount,
                        collateral_amount: _amount * balance!(pool_info.collateral_factor),
                        acumulated_debt: 0,
                        borrow_start_block: current_block,
                    };

                    // Update pool info
                    let updated_pool_info = PoolInfo {
                        asset_id: pool_info.asset_id,
                        balance: pool_info.balance - _amount,
                        lending_rate: pool_info.lending_rate,
                        borrowed_rate: pool_info.borrowed_rate,
                        collateral_factor: pool_info.collateral_factor,
                    };

                    // Transfer tokens -> From pool to user (borrowed_amount)
                    Assets::<T>::transfer_from(&_asset_id, &Self::account_id(), &user_id, _amount);
                    // Transfer tokens -> From user to pool (collateral_amount)
                    Assets::<T>::transfer_from(
                        &_asset_id,
                        &user_id,
                        &Self::account_id(),
                        updated_user_info.collateral_amount,
                    );

                    // Save new user info
                    PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                    // Save updated pool info
                    Pools::<T>::set(_asset_id, updated_pool_info);

                    // Depositing event
                    Self::deposit_event(Event::NewBorrowingUser(user_id, _asset_id, _amount));
                }
            } else {
                // Create new user
                let new_user_info = UserInfo {
                    lending_amount: 0,
                    lending_earnings: 0,
                    lending_start_block: current_block,
                    borrowed_amount: _amount,
                    collateral_amount: _amount * balance!(pool_info.collateral_factor),
                    acumulated_debt: 0,
                    borrow_start_block: current_block,
                };

                // Update pool info
                let updated_pool_info = PoolInfo {
                    asset_id: pool_info.asset_id,
                    balance: pool_info.balance - _amount,
                    lending_rate: pool_info.lending_rate,
                    borrowed_rate: pool_info.borrowed_rate,
                    collateral_factor: pool_info.collateral_factor,
                };

                // Transfer tokens -> From pool to user (borrowed_amount)
                Assets::<T>::transfer_from(&_asset_id, &Self::account_id(), &user_id, _amount);
                // Transfer tokens -> From user to pool (collateral_amount)
                Assets::<T>::transfer_from(
                    &_asset_id,
                    &user_id,
                    &Self::account_id(),
                    new_user_info.collateral_amount,
                );

                // Save new user info
                PoolUsers::<T>::set(_asset_id, &user_id, new_user_info);

                // Save updated pool info
                Pools::<T>::set(_asset_id, updated_pool_info);

                // Deposit event
                Self::deposit_event(Event::NewBorrowingUser(user_id, _asset_id, _amount));
            }

            Ok(().into())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(0)]
        pub fn withdraw_tokens(
            origin: OriginFor<T>,
            _asset_id: AssetIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            let user_id = ensure_signed(origin)?;

            // Check if pool exists
            ensure!(
                Pools::<T>::contains_key(&_asset_id),
                Error::<T>::PoolDoesntExist
            );

            // Get pool info
            let pool_info = Pools::<T>::get(&_asset_id);

            // Check if user exists
            ensure!(
                PoolUsers::<T>::contains_key(&_asset_id, &user_id),
                Error::<T>::UserDoesntExist
            );

            // Get user info
            let user_info = PoolUsers::<T>::get(&_asset_id, &user_id);

            // Check if user lended tokens
            ensure!(
                user_info.lending_amount > 0,
                Error::<T>::UserHasntLendedTokens
            );

            // Get current block
            let current_block = frame_system::Pallet::<T>::block_number();

            // Calculate block difference
            let block_difference: u128 =
                (current_block - user_info.borrow_start_block).unique_saturated_into();

            // Calculate current debt
            let current_debt =
                (block_difference * (pool_info.borrowed_rate / 432000)) * user_info.borrowed_amount;

            // Calculate total debt
            let total_debt = current_debt + user_info.acumulated_debt;

            // TODO: Function that calculates and updates debt

            // Check if user payed debts
            ensure!(total_debt == 0, Error::<T>::UserHasntPayedDebts);

            // Calculate current earnings
            let current_earnings =
                (block_difference * (pool_info.lending_rate / 432000)) * user_info.lending_amount;

            // Calculate total earnings
            let total_earnings = current_earnings + user_info.lending_earnings;

            // TODO: Function that calculates and updates earnings

            // Total amount needed to be withdrawn
            let withdrawl_total = user_info.lending_amount + total_earnings;

            // Check if pool has enough tokens
            ensure!(
                pool_info.balance >= withdrawl_total,
                Error::<T>::InsufficientFundsOnPool,
            );

            // Withdrawl tokens
            Assets::<T>::transfer_from(&_asset_id, &Self::account_id(), &user_id, withdrawl_total);

            // Update user info
            let updated_user_info = UserInfo {
                lending_amount: 0,
                lending_earnings: 0,
                lending_start_block: current_block,
                borrowed_amount: user_info.borrowed_amount,
                acumulated_debt: user_info.acumulated_debt,
                collateral_amount: user_info.collateral_amount,
                borrow_start_block: user_info.borrow_start_block,
            };

            // Update pool info
            let updated_pool_info = PoolInfo {
                asset_id: pool_info.asset_id,
                balance: pool_info.balance - withdrawl_total,
                lending_rate: pool_info.lending_rate,
                borrowed_rate: pool_info.borrowed_rate,
                collateral_factor: pool_info.collateral_factor,
            };

            // Save updated user info
            PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

            // Save updated pool info
            Pools::<T>::set(_asset_id, updated_pool_info);

            // Depositing event
            Self::deposit_event(Event::UserWithdrewLendedTokens(
                user_id,
                _asset_id,
                withdrawl_total,
            ));

            Ok(().into())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(0)]
        pub fn return_tokens(
            origin: OriginFor<T>,
            _asset_id: AssetIdOf<T>,
            _amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user_id = ensure_signed(origin)?;

            // Check if pool exists
            ensure!(
                Pools::<T>::contains_key(&_asset_id),
                Error::<T>::PoolDoesntExist
            );

            // Get pool info
            let pool_info = Pools::<T>::get(&_asset_id);

            // Check if user exists
            ensure!(
                PoolUsers::<T>::contains_key(&_asset_id, &user_id),
                Error::<T>::UserDoesntExist
            );

            // Retrieve user info
            let user_info = PoolUsers::<T>::get(&_asset_id, &user_id);

            // Check if user has borrowed tokens
            ensure!(
                user_info.borrowed_amount > 0,
                Error::<T>::UserHasntLendedTokens
            );

            // Check if user has enough tokens
            ensure!(
                Assets::<T>::free_balance(&_asset_id, &user_id).unwrap_or(0) >= _amount,
                Error::<T>::InsufficientFunds
            );

            // Get current block
            let current_block = frame_system::Pallet::<T>::block_number();

            // Calculate block difference
            let block_difference: u128 =
                (current_block - user_info.borrow_start_block).unique_saturated_into();

            // Calculate current debt
            let current_debt =
                (block_difference * (pool_info.borrowed_rate / 432000)) * user_info.borrowed_amount;

            // Calculate total debt
            let total_debt = current_debt + user_info.acumulated_debt;

            // Calculate payed debt difference
            let payed_debt_difference = _amount - total_debt;

            // TODO: Function that calculates and updates debt

            // Check if user payed of debt
            if payed_debt_difference > 0 {
                // Check if user payed off borrowing debts
                if _amount >= user_info.borrowed_amount + total_debt {
                    // Calculate adequate borrow return
                    let borrow_return = user_info.borrowed_amount + total_debt;

                    // Pay debt
                    Assets::<T>::transfer_from(
                        &_asset_id,
                        &user_id,
                        &Self::account_id(),
                        borrow_return,
                    );

                    // Return colateral
                    Assets::<T>::transfer_from(
                        &_asset_id,
                        &Self::account_id(),
                        &user_id,
                        user_info.collateral_amount,
                    );

                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: user_info.lending_amount,
                        lending_earnings: user_info.lending_earnings,
                        lending_start_block: user_info.lending_start_block,
                        borrowed_amount: 0,
                        acumulated_debt: 0,
                        collateral_amount: 0,
                        borrow_start_block: current_block,
                    };

                    // Update pool info
                    let updated_pool_info = PoolInfo {
                        asset_id: pool_info.asset_id,
                        balance: pool_info.balance + borrow_return,
                        borrowed_rate: pool_info.borrowed_rate,
                        lending_rate: pool_info.lending_rate,
                        collateral_factor: pool_info.collateral_factor,
                    };

                    // Save updated user info
                    PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                    // Save updated pool info
                    Pools::<T>::set(_asset_id, updated_pool_info);

                    // Depositing event
                    Self::deposit_event(Event::UserFullyReturnedBorrowedTokens(
                        user_id, _asset_id, _amount,
                    ));
                } else {
                    // Pay debt
                    Assets::<T>::transfer_from(&_asset_id, &user_id, &Self::account_id(), _amount);

                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: user_info.lending_amount,
                        lending_earnings: user_info.lending_earnings,
                        lending_start_block: user_info.lending_start_block,
                        borrowed_amount: 0,
                        acumulated_debt: 0,
                        collateral_amount: user_info.collateral_amount - payed_debt_difference,
                        borrow_start_block: current_block,
                    };

                    // Update pool info
                    let updated_pool_info = PoolInfo {
                        asset_id: pool_info.asset_id,
                        balance: pool_info.balance + _amount,
                        borrowed_rate: pool_info.borrowed_rate,
                        lending_rate: pool_info.lending_rate,
                        collateral_factor: pool_info.collateral_factor,
                    };

                    // Save updated user info
                    PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                    // Save updated pool info
                    Pools::<T>::set(_asset_id, updated_pool_info);

                    // Depositing event
                    Self::deposit_event(Event::UserFullyPayedOffDebtAndPartOfBorrowed(
                        user_id, _asset_id, _amount,
                    ));
                }
            } else {
                // Pay debt
                Assets::<T>::transfer_from(&_asset_id, &user_id, &Self::account_id(), _amount);

                // Update user info
                let updated_user_info = UserInfo {
                    lending_amount: user_info.lending_amount,
                    lending_earnings: user_info.lending_earnings,
                    lending_start_block: user_info.lending_start_block,
                    borrowed_amount: user_info.borrowed_amount,
                    acumulated_debt: user_info.acumulated_debt - _amount,
                    collateral_amount: user_info.collateral_amount,
                    borrow_start_block: user_info.borrow_start_block,
                };

                // Update pool info
                let updated_pool_info = PoolInfo {
                    asset_id: pool_info.asset_id,
                    balance: pool_info.balance + _amount,
                    borrowed_rate: pool_info.borrowed_rate,
                    lending_rate: pool_info.lending_rate,
                    collateral_factor: pool_info.collateral_factor,
                };

                // Save updated user info
                PoolUsers::<T>::set(_asset_id, &user_id, updated_user_info);

                // Save updated pool info
                Pools::<T>::set(_asset_id, updated_pool_info);

                // Depositing event
                Self::deposit_event(Event::UserPayedPartOfDebt(user_id, _asset_id, _amount));
            }

            Ok(().into())
        }
    }

    //  Defining pallet fooks
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let mut consumed_weight = Self::check_liquidity(now);

            consumed_weight
        }
    }

    //  Use to return the ID of the pallet
    impl<T: Config> Pallet<T> {
        fn account_id() -> T::AccountId {
            PALLET_ID.into_account_truncating()
        }

        fn check_liquidity(current_block: T::BlockNumber) -> Weight {
            let mut counter: u64 = 0;

            for (asset_id, user_id, user_info) in PoolUsers::<T>::iter() {
                let pool_info = Pools::<T>::get(asset_id);

                // Calculate block difference
                let block_difference: u128 =
                    (current_block - user_info.borrow_start_block).unique_saturated_into();

                // Calculate current debt
                let current_debt = (block_difference * (pool_info.borrowed_rate / 432000))
                    * user_info.borrowed_amount;

                // Calculate total debt
                let total_debt = current_debt + user_info.acumulated_debt;

                // Check if debt exceeds colateral
                if total_debt >= user_info.collateral_amount {
                    // Update user info
                    let updated_user_info = UserInfo {
                        lending_amount: user_info.lending_amount,
                        lending_earnings: user_info.lending_earnings,
                        lending_start_block: user_info.lending_start_block,
                        borrowed_amount: 0,
                        acumulated_debt: 0,
                        collateral_amount: 0,
                        borrow_start_block: current_block,
                    };

                    // Save updated user info
                    PoolUsers::<T>::set(asset_id, user_id, updated_user_info);

                    // Update counter
                    counter += 1;
                }
            }

            T::DbWeight::get()
                .reads(counter)
                .saturating_add(T::DbWeight::get().writes(counter))
        }
    }
}
