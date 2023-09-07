#![cfg(feature = "runtime-benchmarks")]

use super::*;

use codec::Decode;
use common::{balance, XOR};
use frame_benchmarking::benchmarks;
use frame_support::traits::Hooks;
use frame_system::{EventRecord, RawOrigin};
use hex_literal::hex;
use sp_std::prelude::*;

use crate::Pallet as LendingBorrowing;
use assets::Pallet as Assets;

// Support Functions
fn authority_account<T: Config>() -> T::AccountId {
    let bytes = hex!("96ea3c9c0be7bbc7b0656a1983db5eed75210256891a9609012362e36815b132");
    AccountIdOf::<T>::decode(&mut &bytes[..]).unwrap()
}

fn alice<T: Config>() -> T::AccountId {
    let bytes = hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");
    T::AccountId::decode(&mut &bytes[..]).unwrap()
}

fn bob<T: Config>() -> T::AccountId {
    let bytes = hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27f");
    T::AccountId::decode(&mut &bytes[..]).expect("Failed to decode account ID")
}

fn run_to_block<T: Config>(n: u32) {
    while frame_system::Pallet::<T>::block_number() < n.into() {
        frame_system::Pallet::<T>::on_finalize(frame_system::Pallet::<T>::block_number().into());
        frame_system::Pallet::<T>::set_block_number(
            frame_system::Pallet::<T>::block_number() + 1u32.into(),
        );
        frame_system::Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number().into());
        LendingBorrowing::<T>::on_initialize(frame_system::Pallet::<T>::block_number().into());
    }
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

benchmarks! {
    create_pool {
        let caller = authority_account::<T>();
        let asset_id = XOR;
        let lending_interest = balance!(0.035);
        let borrowing_interest = balance!(0.05);

    }: {
        LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(caller.clone()).into(),
            asset_id.into(),
            lending_interest,
            borrowing_interest,
        ).unwrap();
    }
    verify {
        assert_last_event::<T>(Event::<T>::PoolCreated(asset_id.into(), lending_interest, borrowing_interest).into());
    }

    lend {
        let authority = authority_account::<T>();
        let caller = alice::<T>();
        let lended_token = XOR;
        let xor_amount = balance!(1000);
        let lended_amount = balance!(100);
        let owner: T::AccountId = assets::AssetOwners::<T>::get::<T::AssetId>(XOR.clone().into()).unwrap();

        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(authority.clone()).into(),
            lended_token.into(),
            balance!(0.035),
            balance!(0.05),
        ).unwrap();

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            lended_token.into(),
            caller.clone(),
            xor_amount,
        );
    }: {
        LendingBorrowing::<T>::lend(RawOrigin::Signed(caller.clone()).into(), lended_token.into(), lended_amount);
    }
    verify {
        assert_last_event::<T>(Event::AssetsLended(caller, lended_token.into(), lended_amount).into());
    }

    borrow {
        let authority = authority_account::<T>();
        let lender = alice::<T>();
        let borrower = bob::<T>();
        let asset_id = XOR;
        let xor_amount = balance!(1000);
        let lended_amount = balance!(200);
        let borrowed_amount = balance!(75);
        let collateral = balance!(100);
        let owner: T::AccountId = assets::AssetOwners::<T>::get::<T::AssetId>(XOR.clone().into()).unwrap();

        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(authority.clone()).into(),
            asset_id.into(),
            balance!(0.035),
            balance!(0.05),
        ).unwrap();

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            asset_id.into(),
            lender.clone(),
            xor_amount,
        );

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            asset_id.into(),
            borrower.clone(),
            xor_amount,
        );

        let _ = LendingBorrowing::<T>::lend(RawOrigin::Signed(lender.clone()).into(), asset_id.into(), lended_amount);

    }: {
        LendingBorrowing::<T>::borrow(RawOrigin::Signed(borrower.clone()).into(), asset_id.into(), borrowed_amount, collateral);
    }
    verify {
        assert_last_event::<T>(Event::AssetsBorrowed(borrower, asset_id.into(), borrowed_amount, collateral).into());
    }

    repay {
        let authority = authority_account::<T>();
        let lender = alice::<T>();
        let borrower = bob::<T>();
        let asset_id = XOR;
        let xor_amount = balance!(1000);
        let lended_amount = balance!(200);
        let borrowed_amount = balance!(75);
        let collateral = balance!(100);
        let owner: T::AccountId = assets::AssetOwners::<T>::get::<T::AssetId>(XOR.clone().into()).unwrap();

        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(authority.clone()).into(),
            asset_id.into(),
            balance!(0.035),
            balance!(0.05),
        ).unwrap();

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            asset_id.into(),
            lender.clone(),
            xor_amount,
        );

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            asset_id.into(),
            borrower.clone(),
            xor_amount,
        );

        let _ = LendingBorrowing::<T>::lend(RawOrigin::Signed(lender.clone()).into(), asset_id.into(), lended_amount);
        let _ = LendingBorrowing::<T>::borrow(RawOrigin::Signed(borrower.clone()).into(), asset_id.into(), borrowed_amount, collateral);
        run_to_block::<T>(100);
    }: {
        LendingBorrowing::<T>::repay(RawOrigin::Signed(borrower.clone()).into(), asset_id.into(), balance!(50));
    }
    verify {
        assert_last_event::<T>(Event::DebtPartiallyRepaid(borrower, asset_id.into(), balance!(50)).into());
    }

    withdraw {
        let authority = authority_account::<T>();
        let lender = alice::<T>();
        let borrower = bob::<T>();
        let asset_id = XOR;
        let xor_amount = balance!(1000);
        let lended_amount = balance!(200);
        let borrowed_amount = balance!(75);
        let collateral = balance!(100);
        let owner: T::AccountId = assets::AssetOwners::<T>::get::<T::AssetId>(XOR.clone().into()).unwrap();

        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(authority.clone()).into(),
            asset_id.into(),
            balance!(0.035),
            balance!(0.05),
        ).unwrap();

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            asset_id.into(),
            lender.clone(),
            xor_amount,
        );

        let _ = Assets::<T>::mint(
            RawOrigin::Signed(owner.clone()).into(),
            asset_id.into(),
            borrower.clone(),
            xor_amount,
        );

        let _ = LendingBorrowing::<T>::lend(RawOrigin::Signed(lender.clone()).into(), asset_id.into(), lended_amount);
        let _ = LendingBorrowing::<T>::borrow(RawOrigin::Signed(borrower.clone()).into(), asset_id.into(), borrowed_amount, collateral);
        run_to_block::<T>(100);
    }: {
        LendingBorrowing::<T>::withdraw(RawOrigin::Signed(lender.clone()).into(), asset_id.into(), balance!(150));
    }
    verify {
        assert_last_event::<T>(Event::AmountWithdrawn(lender, asset_id.into(), balance!(150)).into());
    }


    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::ExtBuilder::default().build(),
        crate::mock::Runtime
    );
}
