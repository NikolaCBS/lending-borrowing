mod tests {
    use crate::mock::*;
    use crate::{pallet, AccountIdOf, AssetIdOf, Error};
    use common::prelude::{AssetInfoProvider, Balance};
    use common::{balance, XOR};
    use frame_support::pallet_prelude::StorageMap;
    use frame_support::sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
    use frame_support::traits::Hooks;
    use frame_support::{assert_err, assert_ok, Identity, PalletId};
    use hex_literal::hex;
    use lending_borrowing;

    // let xor = XOR.into();

    #[test]
    fn create_existing_pool() {
        let mut ext = ExtBuilder::default().build();
        ext.execute_with(|| {
            let asset_id = XOR;
            let pool_balance = balance!(0);
            let lending_interest = balance!(0.035);
            let borrowing_interest = balance!(0.05);

            let pool = LendingBorrowing::create_pool(
                RuntimeOrigin::signed(LendingBorrowing::authority_account()),
                asset_id,
                pool_balance,
                lending_interest,
                borrowing_interest,
            );

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
}
