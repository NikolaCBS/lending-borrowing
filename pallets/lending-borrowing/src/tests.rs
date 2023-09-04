mod tests {
    use crate::mock::*;
    use crate::{pallet, Error};
    use common::prelude::{AssetInfoProvider, FixedWrapper};
    use common::{balance, CERES_ASSET_ID, XOR};
    use frame_support::pallet_prelude::DispatchResultWithPostInfo;
    use frame_support::{assert_err, assert_ok};

    struct Before;

    impl Before {
        fn create_pool() -> DispatchResultWithPostInfo {
            let asset_id = XOR;

            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                asset_id,
                lending_interest,
                borrowing_interest,
            )
        }

        fn lend(user: AccountId) -> DispatchResultWithPostInfo {
            let lended_token = XOR;
            let lended_amount = balance!(100);

            LendingBorrowing::lend(RuntimeOrigin::signed(user), lended_token, lended_amount)
        }

        fn borrow(user: AccountId) -> DispatchResultWithPostInfo {
            let borrowed_token = XOR;
            let borrowed_amount = balance!(75);
            let collateral = balance!(100);

            LendingBorrowing::borrow(
                RuntimeOrigin::signed(user),
                borrowed_token,
                borrowed_amount,
                collateral,
            )
        }
    }

    /// Create_pool tests

    #[test]
    fn create_existing_pool() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();

            let asset_id = XOR;
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    asset_id,
                    lending_interest,
                    borrowing_interest,
                ),
                Error::<Runtime>::PoolAlreadyCreated
            )
        })
    }

    #[test]
    fn create_pool_unauthorized_acc() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let asset_id = XOR;
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(ALICE),
                    asset_id,
                    lending_interest,
                    borrowing_interest
                ),
                Error::<Runtime>::Unauthorized
            );
        })
    }

    #[test]
    fn create_pool_disproportionate_interests() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let asset_id = XOR;
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.06);

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    asset_id,
                    lending_interest,
                    borrowing_interest,
                ),
                Error::<Runtime>::InvalidInterestProportion
            )
        })
    }

    #[test]
    fn create_pool_ok() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let asset_id = XOR;
            let pool_balance = balance!(0);
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            assert_ok!(LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                asset_id,
                lending_interest,
                borrowing_interest,
            ));

            let pool_info = pallet::PoolInfo::<Runtime>::get(&asset_id);

            assert_eq!(pool_info.asset_id, asset_id);
            assert_eq!(pool_info.pool_balance, pool_balance);
            assert_eq!(
                pool_info.lending_interest,
                (FixedWrapper::from(lending_interest) / FixedWrapper::from(balance!(5256000)))
                    .try_into_balance()
                    .unwrap_or(0)
            );
            assert_eq!(
                pool_info.borrowing_interest,
                (FixedWrapper::from(borrowing_interest) / FixedWrapper::from(balance!(5256000)))
                    .try_into_balance()
                    .unwrap_or(0)
            );
        })
    }

    /// Lend tests

    #[test]
    fn lend_to_non_existing_pool() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let lended_token = XOR;
            let lended_amount = balance!(1);
            assert_err!(
                LendingBorrowing::lend(RuntimeOrigin::signed(CHARLIE), lended_token, lended_amount,),
                Error::<Runtime>::PoolDoesNotExist
            )
        })
    }

    #[test]
    fn lend_more_than_balance() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let lended_token = XOR;
            let lended_amount = balance!(1600);
            assert_err!(
                LendingBorrowing::lend(RuntimeOrigin::signed(CHARLIE), lended_token, lended_amount),
                Error::<Runtime>::InsufficientFunds
            )
        })
    }

    // Invalid asset will also show that pool doesn't exist
    #[test]
    fn lend_invalid_asset() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let lended_token = CERES_ASSET_ID.into();
            let lended_amount = balance!(1);

            assert_err!(
                LendingBorrowing::lend(RuntimeOrigin::signed(ALICE), lended_token, lended_amount),
                Error::<Runtime>::PoolDoesNotExist
            )
        })
    }

    #[test]
    fn lend_first_time() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let lended_token = XOR;
            let lended_amount = balance!(100);

            let pool_info_before_lend = pallet::PoolInfo::<Runtime>::get(&lended_token);
            let user_info_before_lend = pallet::UserInfo::<Runtime>::get(&CHARLIE);
            let user_balance_before_lend = Assets::free_balance(&lended_token, &CHARLIE);
            assert_eq!(pool_info_before_lend.pool_balance, 0);
            assert!(user_info_before_lend.is_none());

            let _ = Before::lend(CHARLIE);

            let user_info_after_lend = pallet::UserInfo::<Runtime>::get(&CHARLIE);
            let pool_info_after_lend = pallet::PoolInfo::<Runtime>::get(&lended_token);
            let user_balance_after_lend = Assets::free_balance(&lended_token, &CHARLIE);

            assert!(user_info_after_lend.is_some());

            assert_eq!(pool_info_after_lend.pool_balance, balance!(100));

            assert_eq!(
                user_info_after_lend.as_ref().unwrap().lended_token,
                lended_token
            );
            assert_eq!(
                user_info_after_lend.as_ref().unwrap().lended_amount,
                lended_amount
            );

            assert_eq!(
                user_balance_after_lend.unwrap_or(0),
                user_balance_before_lend.unwrap_or(0) - balance!(100)
            );
        })
    }

    #[test]
    fn lend_second_time() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            run_to_block(1);
            let _ = Before::create_pool();

            // First lend
            let _ = Before::lend(CHARLIE);

            let user_info = pallet::UserInfo::<Runtime>::get(&CHARLIE);
            let lended_amount_after_first_lend = user_info.as_ref().unwrap().lended_amount;
            let last_time_lended = user_info.as_ref().unwrap().last_time_lended;
            let interest_earned = user_info.as_ref().unwrap().interest_earned;

            assert_eq!(last_time_lended, 1);

            run_to_block(60);

            // Second lend
            let _ = Before::lend(CHARLIE);

            let second_lend_user_info = pallet::UserInfo::<Runtime>::get(&CHARLIE);
            let pool_info = pallet::PoolInfo::<Runtime>::get(&XOR);
            let lended_amount_after_second_lend =
                second_lend_user_info.as_ref().unwrap().lended_amount;
            let new_block_lended = second_lend_user_info.as_ref().unwrap().last_time_lended;
            let new_interest_earned = second_lend_user_info.as_ref().unwrap().interest_earned;
            let user_balance_after_second_lend = Assets::free_balance(&XOR, &CHARLIE);

            assert_eq!(pool_info.pool_balance, balance!(200));
            assert_eq!(user_balance_after_second_lend.unwrap_or(0), balance!(1300));
            assert_eq!(new_block_lended, 60);
            assert_eq!(
                lended_amount_after_second_lend,
                lended_amount_after_first_lend + balance!(100)
            );

            assert_ne!(new_interest_earned, interest_earned);
        })
    }

    /// Borrow tests

    #[test]
    fn borrow_insufficient_pool_liquidity() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let asset = XOR;
            let borrowed_amount = balance!(200);
            let collateral = balance!(250);

            assert_err!(
                LendingBorrowing::borrow(
                    RuntimeOrigin::signed(DAVE),
                    asset,
                    borrowed_amount,
                    collateral
                ),
                Error::<Runtime>::NotEnoughTokensInPool
            )
        })
    }

    #[test]
    fn borrow_inadequate_collateral() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let asset = XOR;
            let borrowed_amount = balance!(90);
            let collateral = balance!(60);

            assert_err!(
                LendingBorrowing::borrow(
                    RuntimeOrigin::signed(DAVE),
                    asset,
                    borrowed_amount,
                    collateral
                ),
                Error::<Runtime>::InadequateCollateral
            )
        })
    }

    #[test]
    fn borrow_collatral_greater_than_balance() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            for i in 0..10 {
                run_to_block(i);
                let _ = Before::lend(CHARLIE);
            }
            let asset = XOR;
            let borrowed_amount = balance!(900);
            let collateral = balance!(1200);

            assert_err!(
                LendingBorrowing::borrow(
                    RuntimeOrigin::signed(DAVE),
                    asset,
                    borrowed_amount,
                    collateral
                ),
                Error::<Runtime>::InsufficientFunds
            )
        })
    }

    #[test]
    fn borrow_first_time_ok() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let asset = XOR;
            let borrowed_amount = balance!(75);
            let collateral = balance!(100);

            let user_balance_before_borrow = Assets::free_balance(&asset, &DAVE);
            let pool_info_before_borrow = pallet::PoolInfo::<Runtime>::get(asset);

            assert_ok!(LendingBorrowing::borrow(
                RuntimeOrigin::signed(DAVE),
                asset,
                borrowed_amount,
                collateral
            ));

            let user_info = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let user_balance_after_borrow = Assets::free_balance(&asset, &DAVE);
            let pool_info_after_borrow = pallet::PoolInfo::<Runtime>::get(asset);

            assert_eq!(user_info.borrowed_amount, borrowed_amount);
            assert_eq!(user_info.last_time_borrowed, 0);
            assert_eq!(user_info.debt_interest, 0);
            assert_eq!(user_info.collateral, collateral);
            assert_eq!(
                user_balance_after_borrow.unwrap_or(0),
                user_balance_before_borrow.unwrap_or(0) - balance!(100) + balance!(75)
            );

            assert_ne!(
                pool_info_before_borrow.pool_balance,
                pool_info_after_borrow.pool_balance
            );
            assert_eq!(
                pool_info_after_borrow.pool_balance,
                pool_info_before_borrow.pool_balance + balance!(100) - balance!(75)
            );
        })
    }

    #[test]
    fn borrow_again() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            let first_borrow_user_info = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let first_borrow_pool_info = pallet::PoolInfo::<Runtime>::get(&XOR);

            run_to_block(100);
            let _ = Before::borrow(DAVE);

            let second_borrow_user_info = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let second_borrow_pool_info = pallet::PoolInfo::<Runtime>::get(&XOR);

            assert_eq!(
                second_borrow_user_info.borrowed_amount,
                first_borrow_user_info.borrowed_amount + balance!(75)
            );
            assert_eq!(
                second_borrow_user_info.collateral,
                first_borrow_user_info.collateral + balance!(100)
            );
            assert_ne!(
                first_borrow_user_info.debt_interest,
                second_borrow_user_info.debt_interest
            );
            assert_eq!(
                second_borrow_pool_info.pool_balance,
                first_borrow_pool_info.pool_balance + first_borrow_user_info.collateral
                    - first_borrow_user_info.borrowed_amount
            );
        })
    }

    /// Repay tests

    #[test]
    fn repay_nonexisting_user() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            assert_err!(
                LendingBorrowing::repay(RuntimeOrigin::signed(CHARLIE), XOR, balance!(100)),
                Error::<Runtime>::UserDoesNotExist
            );
        })
    }

    #[test]
    fn repay_user_with_no_debt() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);

            let user_info = pallet::UserInfo::<Runtime>::get(&CHARLIE).unwrap();
            assert_eq!(user_info.debt_interest, 0);
            assert_err!(
                LendingBorrowing::repay(RuntimeOrigin::signed(CHARLIE), XOR, balance!(0)),
                Error::<Runtime>::NoDebtToRepay
            );
        })
    }

    #[test]
    fn over_repay() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            assert_err!(
                LendingBorrowing::repay(RuntimeOrigin::signed(DAVE), XOR, balance!(100)),
                Error::<Runtime>::ExcessiveAmount
            );
        })
    }

    #[test]
    fn repay_more_than_principal_less_than_full() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);
            run_to_block(100);

            assert_err!(
                LendingBorrowing::repay(RuntimeOrigin::signed(DAVE), XOR, balance!(75.00000007)),
                Error::<Runtime>::RepayFullyOrPartOfPrincipal
            );
        })
    }

    #[test]
    fn repay_full() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);
            let user_info_before_repay = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let pool_info_before_repay = pallet::PoolInfo::<Runtime>::get(&XOR);
            let user_balance_before_repay = Assets::free_balance(&XOR, &DAVE);

            run_to_block(100);
            let full_debt = balance!(0.000071347031962500) + balance!(75);

            assert_ok!(LendingBorrowing::repay(
                RuntimeOrigin::signed(DAVE),
                XOR,
                full_debt
            ));

            let user_info_after_repay = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let pool_info_after_repay = pallet::PoolInfo::<Runtime>::get(&XOR);
            let user_balance_after_repay = Assets::free_balance(&XOR, &DAVE);

            assert!(
                pool_info_before_repay.pool_balance
                    < pool_info_after_repay.pool_balance + user_info_before_repay.collateral
                        - (user_info_before_repay.borrowed_amount
                            + user_info_before_repay.debt_interest)
            );

            assert_eq!(
                user_balance_before_repay.unwrap(),
                user_balance_after_repay.unwrap() - user_info_before_repay.collateral + full_debt
            );
            assert_eq!(user_info_after_repay.borrowed_amount, 0);
            assert_eq!(user_info_after_repay.debt_interest, 0);
            assert_eq!(user_info_after_repay.collateral, 0);
        })
    }

    #[test]
    fn repay_partial() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);
            let user_info_before_repay = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let pool_info_before_repay = pallet::PoolInfo::<Runtime>::get(&XOR);
            let interest_before_repay = user_info_before_repay.debt_interest;

            run_to_block(100);
            let partial_debt = balance!(50);

            assert_ok!(LendingBorrowing::repay(
                RuntimeOrigin::signed(DAVE),
                XOR,
                partial_debt
            ));

            let user_info_after_repay = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();
            let pool_info_after_repay = pallet::PoolInfo::<Runtime>::get(&XOR);
            let interest_after_repay = user_info_after_repay.debt_interest;

            assert_eq!(
                pool_info_before_repay.pool_balance + partial_debt,
                pool_info_after_repay.pool_balance
            );
            assert_eq!(
                user_info_before_repay.borrowed_amount,
                user_info_after_repay.borrowed_amount + partial_debt
            );
            assert_eq!(interest_before_repay, 0);
            assert_eq!(interest_after_repay, balance!(0.000071347031962500));

            run_to_block(200);
            let _ = LendingBorrowing::repay(RuntimeOrigin::signed(DAVE), XOR, balance!(10));
            let user_info_second_repay = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();

            assert_ne!(
                user_info_after_repay.borrowed_amount,
                user_info_second_repay.borrowed_amount
            );

            assert_eq!(
                user_info_second_repay.debt_interest,
                user_info_after_repay.debt_interest
                    + (user_info_second_repay.debt_interest - user_info_after_repay.debt_interest)
            );
        })
    }

    /// Withdraw tests

    #[test]
    fn withdraw_invalid_asset() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            run_to_block(50);

            assert_err!(
                LendingBorrowing::withdraw(
                    RuntimeOrigin::signed(CHARLIE),
                    CERES_ASSET_ID.into(),
                    balance!(50)
                ),
                Error::<Runtime>::InvalidAsset
            );
        })
    }

    #[test]
    fn withdraw_nonexisting_user() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            run_to_block(50);

            assert_err!(
                LendingBorrowing::withdraw(RuntimeOrigin::signed(ALICE), XOR, balance!(50)),
                Error::<Runtime>::UserDoesNotExist
            );
        })
    }

    #[test]
    fn withdraw_no_tokens_lended() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            run_to_block(50);

            assert_err!(
                LendingBorrowing::withdraw(RuntimeOrigin::signed(DAVE), XOR, balance!(50)),
                Error::<Runtime>::NoTokensLended
            );
        })
    }

    #[test]
    fn withdraw_more_than_lended() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            run_to_block(50);

            assert_err!(
                LendingBorrowing::withdraw(RuntimeOrigin::signed(CHARLIE), XOR, balance!(101)),
                Error::<Runtime>::ExcessiveAmount
            );
        })
    }

    #[test]
    fn withdraw_full_amount() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            let user_info_before_withdraw = pallet::UserInfo::<Runtime>::get(&CHARLIE).unwrap();
            let user_balance_before_withdraw = Assets::free_balance(&XOR, &CHARLIE);
            let pool_info_before_withdraw = pallet::PoolInfo::<Runtime>::get(&XOR);

            assert_eq!(user_info_before_withdraw.lended_amount, balance!(100));
            assert_eq!(user_info_before_withdraw.interest_earned, 0);
            assert_eq!(user_info_before_withdraw.last_time_lended, 0);
            assert_eq!(user_balance_before_withdraw.unwrap(), balance!(1400));
            assert_eq!(pool_info_before_withdraw.pool_balance, balance!(125));

            run_to_block(100);
            let _ = LendingBorrowing::withdraw(RuntimeOrigin::signed(CHARLIE), XOR, balance!(100));

            let user_info_after_withdraw = pallet::UserInfo::<Runtime>::get(&CHARLIE).unwrap();
            let user_balance_after_withdraw = Assets::free_balance(&XOR, &CHARLIE);
            let pool_info_after_withdraw = pallet::PoolInfo::<Runtime>::get(&XOR);

            assert_eq!(user_info_after_withdraw.lended_amount, 0);
            assert_eq!(user_info_after_withdraw.interest_earned, 0);
            assert_eq!(user_info_after_withdraw.last_time_lended, 0);
            assert_eq!(
                user_balance_after_withdraw.unwrap(),
                balance!(1500) + balance!(0.000066590563160000)
            );
            assert_eq!(pool_info_after_withdraw.pool_balance, balance!(25));
        })
    }

    #[test]
    fn withdraw_partial_amount() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE);

            let user_balance_before_withdraw = Assets::free_balance(&XOR, &CHARLIE);
            let pool_info_before_withdraw = pallet::PoolInfo::<Runtime>::get(&XOR);

            run_to_block(100);
            let _ = LendingBorrowing::withdraw(RuntimeOrigin::signed(CHARLIE), XOR, balance!(50));

            let user_info_after_withdraw = pallet::UserInfo::<Runtime>::get(&CHARLIE).unwrap();
            let user_balance_after_withdraw = Assets::free_balance(&XOR, &CHARLIE);
            let pool_info_after_withdraw = pallet::PoolInfo::<Runtime>::get(&XOR);

            assert_eq!(
                user_balance_before_withdraw.unwrap(),
                user_balance_after_withdraw.unwrap() - balance!(50)
            );
            assert_eq!(
                user_info_after_withdraw.interest_earned,
                balance!(0.000066590563160000)
            );
            assert_eq!(user_info_after_withdraw.lended_amount, balance!(50));
            assert_eq!(
                pool_info_after_withdraw.pool_balance,
                pool_info_before_withdraw.pool_balance - balance!(50)
            );
        })
    }

    /// Liquidate test

    #[test]
    fn liquidate_user() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                XOR,
                balance!(3.5),
                balance!(5),
            );
            let _ = Before::lend(CHARLIE);
            let _ = Before::borrow(DAVE); // 75 XOR borrow, 100 XOR collateral

            // 5 / 5256000 = 0.00000095129375951293 (interest per block)
            // 0.00000095129375951293 * 75 = 0.00007134703196346975 (XOR debt per block for borrow of 75 XOR)
            // On the interest rate of 500% per year (balance!(5)), user who borrowed 75 XOR should be liquidated after
            // 350400 blocks (25 / 0.00007134703196346975). In other words, user will be liquidated after 24 days
            run_to_block(350401);

            let user_info_after_liquidation = pallet::UserInfo::<Runtime>::get(&DAVE).unwrap();

            assert_eq!(user_info_after_liquidation.debt_interest, 0);
            assert_eq!(user_info_after_liquidation.borrowed_amount, 0);
            assert_eq!(user_info_after_liquidation.collateral, 0);
        })
    }
}
