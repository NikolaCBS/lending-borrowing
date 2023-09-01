mod tests {
    use crate::mock::*;
    use crate::{pallet, Error, PoolInfo, UserInfo};
    use common::prelude::FixedWrapper;
    use common::{balance, AssetInfoProvider, Balance, CERES_ASSET_ID};
    use frame_support::PalletId;
    use frame_support::{assert_err, assert_ok};
    use sp_runtime::traits::AccountIdConversion;

    // Update earnings and debt
    #[test]
    fn update_earnings() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            run_to_block(10);

            let pool_info = pallet::Pools::<Runtime>::get(CERES_ASSET_ID);

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(500)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(0)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(500)
            );

            let mut user_info = pallet::PoolUsers::<Runtime>::get(CERES_ASSET_ID, BOB);

            // Test before update
            assert_eq!(user_info.lending_amount, balance!(500));
            assert_eq!(user_info.lending_earnings, balance!(0));
            assert_eq!(user_info.lending_start_block, 10);

            run_to_block(50);

            let earnings = self::calculate_earnings(&user_info, &pool_info);

            LendingBorrowing::update_earnings_and_debt(&mut user_info, &pool_info);

            // Test after update
            assert_eq!(user_info.lending_amount, balance!(500));
            assert_eq!(user_info.lending_earnings, earnings);
            assert_eq!(user_info.lending_start_block, 50);
        });
    }

    #[test]
    fn update_debt() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            run_to_block(10);

            let pool_info = pallet::Pools::<Runtime>::get(CERES_ASSET_ID);

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(500)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(0)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(500)
            );

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(142)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(142.6)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(457.4)
            );

            let mut user_info = pallet::PoolUsers::<Runtime>::get(CERES_ASSET_ID, ALICE);

            // Test before update
            assert_eq!(user_info.borrowed_amount, balance!(142));
            assert_eq!(user_info.borrow_start_block, 10);
            assert_eq!(user_info.collateral_amount, balance!(99.4));
            assert_eq!(user_info.accumulated_debt, balance!(0));

            run_to_block(50);

            let debt = self::calculate_debt(&user_info, &pool_info);

            LendingBorrowing::update_earnings_and_debt(&mut user_info, &pool_info);

            // Test after update
            assert_eq!(user_info.borrowed_amount, balance!(142));
            assert_eq!(user_info.borrow_start_block, 50);
            assert_eq!(user_info.collateral_amount, balance!(99.4));
            assert_eq!(user_info.accumulated_debt, debt);
        });
    }

    /// Create pool tests
    #[test]
    fn create_pool_unathorized() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(0.3),
                    balance!(0.45),
                    balance!(0.2),
                ),
                Error::<Runtime>::UnauthorizedPoolCreation
            );
        });
    }

    #[test]
    fn create_pool_already_exists() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                    balance!(0.3),
                    balance!(0.51),
                    balance!(0.2),
                ),
                Error::<Runtime>::PoolAlreadyExists
            );
        });
    }

    #[test]
    fn create_pool_invalid_lending_rate() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                    balance!(0),
                    balance!(0.45),
                    balance!(0.2),
                ),
                Error::<Runtime>::InvalidRateValues
            );
        });
    }

    #[test]
    fn create_pool_invalid_borrow_rate() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                    balance!(0.3),
                    balance!(0.2),
                    balance!(0.2),
                ),
                Error::<Runtime>::InvalidRateValues
            );
        });
    }

    #[test]
    fn create_pool_invalid_collateral_factor() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                    balance!(0.3),
                    balance!(0.51),
                    balance!(0),
                ),
                Error::<Runtime>::InvalidCollateralFactor
            );
        });
    }

    #[test]
    fn create_pool_ok() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.5),
            ));

            let pool_info = pallet::Pools::<Runtime>::get(CERES_ASSET_ID);

            // Check is parameters are set as needed
            let lending_rate = (FixedWrapper::from(balance!(0.3)) / FixedWrapper::from(5256000))
                .try_into_balance()
                .unwrap_or(0);
            let borrow_rate = (FixedWrapper::from(balance!(0.51)) / FixedWrapper::from(5256000))
                .try_into_balance()
                .unwrap_or(0);

            assert_eq!(pool_info.lending_rate, lending_rate);
            assert_eq!(pool_info.borrow_rate, borrow_rate);
            assert_eq!(pool_info.collateral_factor, balance!(0.5));
        });
    }

    /// Lending   
    #[test]
    fn lend_tokens_nonexistent_pool() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::lend_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(100)
                ),
                Error::<Runtime>::PoolDoesntExist
            );
        });
    }

    #[test]
    fn lend_tokens_insufficient_funds() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            assert_err!(
                LendingBorrowing::lend_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(500)
                ),
                Error::<Runtime>::InsufficientFunds
            );
        });
    }

    #[test]
    fn lend_tokens_ok() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(100)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(0)
            );

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(100)
            );
        });
    }

    /// Borrowing
    #[test]
    fn borrow_tokens_nonexistent_pool() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::borrow_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(100)
                ),
                Error::<Runtime>::PoolDoesntExist
            );
        });
    }

    #[test]
    fn borrow_tokens_insufficient_pool_funds() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            assert_err!(
                LendingBorrowing::borrow_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(100)
                ),
                Error::<Runtime>::InsufficientFundsOnPool
            );
        });
    }

    #[test]
    fn borrow_tokens_insufficient_collateral() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(400)
            ));

            assert_err!(
                LendingBorrowing::borrow_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(143)
                ),
                Error::<Runtime>::InsufficientFunds
            );
        });
    }

    #[test]
    fn borrow_tokens_ok() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            run_to_block(10);

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(400)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(100)
            );

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(400)
            );

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(10)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(103)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(397)
            );
        });
    }

    /// Withdraw tokens
    #[test]
    fn withdraw_tokens_nonexistent_pool() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::withdraw_tokens(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                ),
                Error::<Runtime>::PoolDoesntExist
            );
        });
    }

    #[test]
    fn withdraw_tokens_nonexistent_user() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            assert_err!(
                LendingBorrowing::withdraw_tokens(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                ),
                Error::<Runtime>::UserDoesntExist
            );
        });
    }

    #[test]
    fn withdraw_tokens_non_lending_user() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(500)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(500)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(0)
            );

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(10)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(103)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(497)
            );

            assert_err!(
                LendingBorrowing::withdraw_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                ),
                Error::<Runtime>::UserHasntLendedTokens
            );
        });
    }

    #[test]
    fn withdraw_tokens_user_has_unpayed_debts() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(500)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(500)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(0)
            );

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(142)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(142.6)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(457.4)
            );

            assert_err!(
                LendingBorrowing::withdraw_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                ),
                Error::<Runtime>::UserHasntLendedTokens
            );
        });
    }

    #[test]
    fn withdraw_tokens_insufficient_pool_funds() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            run_to_block(10);

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(50)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(50)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(50)
            );

            run_to_block(14_400);

            assert_err!(
                LendingBorrowing::withdraw_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                ),
                Error::<Runtime>::InsufficientFundsOnPool
            );
        });
    }

    /// Return tokens
    #[test]
    fn return_tokens_nonexistent_pool() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_err!(
                LendingBorrowing::return_tokens(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                    balance!(100),
                ),
                Error::<Runtime>::PoolDoesntExist
            );
        });
    }

    #[test]
    fn return_tokens_nonexistent_user() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            assert_err!(
                LendingBorrowing::return_tokens(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    CERES_ASSET_ID.into(),
                    balance!(100),
                ),
                Error::<Runtime>::UserDoesntExist
            );
        });
    }

    #[test]
    fn return_tokens_non_borrowing_user() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.2),
            ));

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(100)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(0)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(100)
            );

            assert_err!(
                LendingBorrowing::return_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(100)
                ),
                Error::<Runtime>::UserHasntBorrowedTokens
            );
        });
    }

    #[test]
    fn return_tokens_insufficient_user_funds() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(500)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(0)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(500)
            );

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(142)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(142.6)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(457.4)
            );

            assert_err!(
                LendingBorrowing::return_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(150)
                ),
                Error::<Runtime>::InsufficientFunds
            );
        });
    }

    /// Liquidity check
    #[test]
    fn liquidate_in_debt_user() {
        let mut ext = ExtBuilder::default().build();

        ext.execute_with(|| {
            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                CERES_ASSET_ID.into(),
                balance!(0.3),
                balance!(0.51),
                balance!(0.7),
            ));

            let pallet_id = PalletId(*b"lendborw").into_account_truncating();
            let pool_info = pallet::Pools::<Runtime>::get(CERES_ASSET_ID);

            run_to_block(10);

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(500)
            ));

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &BOB).unwrap(),
                balance!(0)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(500)
            );

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(142)
            ));

            let mut user_info = pallet::PoolUsers::<Runtime>::get(CERES_ASSET_ID, ALICE);

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &ALICE).unwrap(),
                balance!(142.6)
            );

            assert_eq!(
                Assets::free_balance(&CERES_ASSET_ID.into(), &pallet_id).unwrap(),
                balance!(457.4)
            );

            assert_eq!(user_info.borrow_start_block, 10);
            assert_eq!(user_info.borrowed_amount, balance!(142));
            assert_eq!(user_info.collateral_amount, balance!(99.4));
            assert_eq!(user_info.accumulated_debt, balance!(0));

            // Calculate blocks till liquidation
            // BUG: Returns 7214117 should return 7226471
            let blocks_till_liquidation = (FixedWrapper::from(user_info.collateral_amount)
                / (FixedWrapper::from(pool_info.borrow_rate)
                    * FixedWrapper::from(user_info.borrowed_amount)))
            .try_into_balance()
            .unwrap_or(0)
                + 20000;

            assert_eq!(
                pallet::PoolUsers::<Runtime>::contains_key(CERES_ASSET_ID, ALICE),
                true
            );
            assert_eq!(pallet::Pools::<Runtime>::contains_key(CERES_ASSET_ID), true);

            run_to_block(blocks_till_liquidation.try_into().unwrap());

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(100)
            ));

            user_info = pallet::PoolUsers::<Runtime>::get(CERES_ASSET_ID, ALICE);

            assert_eq!(
                user_info.lending_start_block,
                blocks_till_liquidation as u64
            );
            assert_eq!(user_info.lending_amount, balance!(100));

            assert_eq!(user_info.accumulated_debt, user_info.collateral_amount);
            /*
            let debt = self::calculate_debt(&user_info, &pool_info);

            assert_err!(
                LendingBorrowing::return_tokens(
                    RuntimeOrigin::signed(ALICE),
                    CERES_ASSET_ID.into(),
                    balance!(100)
                ),
                Error::<Runtime>::UserHasntBorrowedTokens
            );
            */
        });
    }

    fn calculate_debt(user_info: &UserInfo<BlockNumber>, pool_info: &PoolInfo<AssetId>) -> Balance {
        let block_difference = System::block_number() - user_info.borrow_start_block;
        let debt = ((FixedWrapper::from(block_difference)
            * FixedWrapper::from(pool_info.borrow_rate))
            * FixedWrapper::from(user_info.borrowed_amount))
        .try_into_balance()
        .unwrap_or(0);

        debt
    }

    fn calculate_earnings(
        user_info: &UserInfo<BlockNumber>,
        pool_info: &PoolInfo<AssetId>,
    ) -> Balance {
        let block_difference = System::block_number() - user_info.lending_start_block;
        let earnings = ((FixedWrapper::from(block_difference)
            * FixedWrapper::from(pool_info.lending_rate))
            * FixedWrapper::from(user_info.lending_amount))
        .try_into_balance()
        .unwrap_or(0);

        earnings
    }
}
