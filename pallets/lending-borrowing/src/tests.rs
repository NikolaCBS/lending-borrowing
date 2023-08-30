mod tests {
    use crate::mock::*;
    use crate::{pallet, AccountIdOf, AssetIdOf, Error};
    use common::prelude::{AssetInfoProvider, Balance};
    use common::{balance, CERES_ASSET_ID, XOR};
    use frame_support::pallet_prelude::{DispatchResultWithPostInfo, StorageMap};
    use frame_support::sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
    use frame_support::traits::Hooks;
    use frame_support::{assert_err, assert_ok, Identity, PalletId};

    struct Before;

    impl Before {
        fn create_pool() -> DispatchResultWithPostInfo {
            let asset_id = XOR;
            let pool_balance = balance!(0);
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                asset_id,
                pool_balance,
                lending_interest,
                borrowing_interest,
            )
        }

        fn lend(user: AccountId) -> DispatchResultWithPostInfo {
            let lended_token = XOR;
            let lended_amount = balance!(100);

            LendingBorrowing::lend(RuntimeOrigin::signed(user), lended_token, lended_amount)
        }
    }

    /// Create_pool tests

    #[test]
    fn create_existing_pool() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let _ = Before::create_pool();

            let asset_id = XOR;
            let pool_balance = balance!(0);
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    asset_id,
                    pool_balance,
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
            let pool_balance = balance!(0);
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(ALICE),
                    asset_id,
                    pool_balance,
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
            let pool_balance = balance!(0);
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.06);

            assert_err!(
                LendingBorrowing::create_pool(
                    RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                    asset_id,
                    pool_balance,
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
                pool_balance,
                lending_interest,
                borrowing_interest,
            ));

            let pool_info = pallet::PoolInfo::<Runtime>::get(&asset_id);

            assert_eq!(pool_info.asset_id, asset_id);
            assert_eq!(pool_info.pool_balance, pool_balance);
            assert_eq!(
                pool_info.lending_interest,
                lending_interest / balance!(5256000)
            );
            assert_eq!(
                pool_info.borrowing_interest,
                borrowing_interest / balance!(5256000)
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
            let lended_amount = balance!(1001);
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

            assert_eq!(user_balance_after_lend.unwrap_or(0), balance!(900));
        })
    }

    #[test]
    fn lend_second_time() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            run_to_block(1);
            let _ = Before::create_pool();
            let _ = Before::lend(CHARLIE);

            let user_info = pallet::UserInfo::<Runtime>::get(&CHARLIE);
            let lended_amount_after_first_lend = user_info.as_ref().unwrap().lended_amount;
            let last_time_lended = user_info.as_ref().unwrap().last_time_lended;
            let interest_earned = user_info.as_ref().unwrap().interest_earned;

            assert_eq!(last_time_lended, 1);

            run_to_block(60);            

            let _ = Before::lend(CHARLIE);

            let second_lend_user_info = pallet::UserInfo::<Runtime>::get(&CHARLIE);
            let lended_amount_after_second_lend =
                second_lend_user_info.as_ref().unwrap().lended_amount;
            let new_block_lended = second_lend_user_info.as_ref().unwrap().last_time_lended;
            let new_interest_earned = second_lend_user_info.as_ref().unwrap().interest_earned;
            
            assert_eq!(new_block_lended, 60);
            assert_eq!(lended_amount_after_second_lend, lended_amount_after_first_lend + balance!(100));

            let interest = LendingBorrowing::calculate_interest(&balance!(0.035), &balance!(100), 1);
            assert_eq!(200, interest);

            assert_ne!(new_interest_earned, interest_earned);
        })
    }
}
