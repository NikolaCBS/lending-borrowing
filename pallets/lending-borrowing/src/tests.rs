mod tests {
    use crate::mock::*;
    use crate::{pallet, Error};
    use common::{balance, generate_storage_instance, AssetInfoProvider, CERES_ASSET_ID, XOR};
    use frame_support::pallet_prelude::StorageMap;
    use frame_support::storage::types::ValueQuery;
    use frame_support::traits::Hooks;
    use frame_support::PalletId;
    use frame_support::{assert_err, assert_ok, Identity};
    use sp_runtime::traits::AccountIdConversion;

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
        });
    }

    /// Lending   
    #[test]
    fn lend_tokens_nonexisting_pool() {
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
        });
    }

    /// Borrowing
    #[test]
    fn borrow_tokens_nonexisting_pool() {
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

            assert_ok!(LendingBorrowing::lend_tokens(
                RuntimeOrigin::signed(BOB),
                CERES_ASSET_ID.into(),
                balance!(400)
            ));

            assert_ok!(LendingBorrowing::borrow_tokens(
                RuntimeOrigin::signed(ALICE),
                CERES_ASSET_ID.into(),
                balance!(10)
            ));
        });
    }
}
