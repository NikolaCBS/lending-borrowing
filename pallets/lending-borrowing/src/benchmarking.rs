//! Lending borrowing platform module benchmarking

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use codec::Decode;
use common::{balance, CERES_ASSET_ID};
use frame_benchmarking::benchmarks;
use frame_support::traits::Hooks;
use frame_system::{EventRecord, RawOrigin};
use hex_literal::hex;
use sp_std::prelude::*;

use crate::Pallet as LendingBorrowing;

/// Support Functions

fn alice<T: Config>() -> T::AccountId {
    let bytes = hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");

    T::AccountId::decode(&mut &bytes[..]).unwrap()
}

fn bob<T: Config>() -> T::AccountId {
    let bytes = hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48");

    T::AccountId::decode(&mut &bytes[..]).unwrap()
}

fn authority<T: Config>() -> T::AccountId {
    let bytes = hex!("96ea3c9c0be7bbc7b0656a1983db5eed75210256891a9609012362e36815b132");

    T::AccountId::decode(&mut &bytes[..]).unwrap()
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();

    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];

    assert_eq!(event, &system_event);
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

benchmarks! {
    create_pool {
        let platform_deployer = authority::<T>();
        let asset_id = CERES_ASSET_ID;
        let lending_rate = balance!(0.3);
        let borrow_rate = balance!(0.51);
        let collateral_factor = balance!(0.7);
    } : {
        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(platform_deployer.clone()).into(),
            asset_id.into(),
            lending_rate,
            borrow_rate,
            collateral_factor
        );
    } verify {
        assert_last_event::<T>(Event::<T>::PoolCreated(asset_id.into()).into());
    }

    /*
    lend_tokens_new_user {
        let platform_deployer = authority::<T>();
        let asset_id = CERES_ASSET_ID;
        let lending_rate = balance!(0.3);
        let borrow_rate = balance!(0.51);
        let collateral_factor = balance!(0.7);

        LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(platform_deployer.clone()).into(),
            asset_id.into(),
            lending_rate,
            borrow_rate,
            collateral_factor,
        ).unwrap();

        let caller = alice::<T>();
        let lending_amount = balance!(50);
    } : {
        let _ = LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller.clone()).into(),
            asset_id.into(),
            lending_amount,
        );
    }
    verify {
        assert_last_event::<T>(Event::<T>::UserLendedTokens(caller, asset_id.into(), lending_amount).into());
    }

    lend_tokens_old_user {
        let platform_deployer = authority::<T>();
        let asset_id = CERES_ASSET_ID;
        let lending_rate = balance!(0.3);
        let borrow_rate = balance!(0.51);
        let collateral_factor = balance!(0.7);

        LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(platform_deployer.clone()).into(),
            asset_id.into(),
            lending_rate,
            borrow_rate,
            collateral_factor,
        ).unwrap();

        let caller_alice = alice::<T>();
        let alice_lending_amount = balance!(50);

        LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
            alice_lending_amount,
        ).unwrap();
    } : {
        let _ =LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
            alice_lending_amount,
        );
    }
    verify {
        assert_last_event::<T>(Event::<T>::UserLendedTokens(caller_alice, asset_id.into(), alice_lending_amount).into());
    }

    borrow_tokens_new_user {
        let platform_deployer = authority::<T>();
        let asset_id = CERES_ASSET_ID;
        let lending_rate = balance!(0.3);
        let borrow_rate = balance!(0.51);
        let collateral_factor = balance!(0.7);

        LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(platform_deployer.clone()).into(),
            asset_id.into(),
            lending_rate,
            borrow_rate,
            collateral_factor,
        ).unwrap();

        let caller_bob = bob::<T>();
        let bob_lending_amount = balance!(500);

        LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller_bob.clone()).into(),
            asset_id.into(),
            bob_lending_amount,
        ).unwrap();

        let caller_alice = alice::<T>();
        let alice_borrowed_amount = balance!(50);
    } : {
        let _ = LendingBorrowing::<T>::borrow_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
            alice_borrowed_amount,
        );
    }
    verify {
        assert_last_event::<T>(Event::<T>::UserBorrowedTokens(caller_alice, asset_id.into(), alice_borrowed_amount).into());
    }

    borrow_tokens_old_user {
        let platform_deployer = authority::<T>();
        let asset_id = CERES_ASSET_ID;
        let lending_rate = balance!(0.3);
        let borrow_rate = balance!(0.51);
        let collateral_factor = balance!(0.7);

        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(platform_deployer.clone()).into(),
            asset_id.into(),
            lending_rate,
            borrow_rate,
            collateral_factor,
        );

        let caller_bob = bob::<T>();
        let bob_lending_amount = balance!(500);

        let _ = LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller_bob.clone()).into(),
            asset_id.into(),
            bob_lending_amount,
        );

        let caller_alice = alice::<T>();
        let alice_borrowed_amount = balance!(50);

        let _ = LendingBorrowing::<T>::borrow_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
            alice_borrowed_amount,
        );
    } : {
        LendingBorrowing::<T>::borrow_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
            alice_borrowed_amount,
        ).unwrap();
    }
    verify {
        assert_last_event::<T>(Event::<T>::UserBorrowedTokens(caller_alice, asset_id.into(), alice_borrowed_amount).into());
    }

    token_withdrawal {
        let platform_deployer = authority::<T>();
        let asset_id = CERES_ASSET_ID;
        let lending_rate = balance!(0.3);
        let borrow_rate = balance!(0.51);
        let collateral_factor = balance!(0.7);

        let _ = LendingBorrowing::<T>::create_pool(
            RawOrigin::Signed(platform_deployer.clone()).into(),
            asset_id.into(),
            lending_rate,
            borrow_rate,
            collateral_factor,
        );

        let caller_bob = bob::<T>();
        let bob_lending_amount = balance!(500);

        let _ = LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller_bob.clone()).into(),
            asset_id.into(),
            bob_lending_amount,
        );

        let caller_alice = alice::<T>();
        let alice_lending_amount = balance!(100);

        let _ = LendingBorrowing::<T>::lend_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
            alice_lending_amount,
        );

        run_to_block::<T>(10000);


    } : {
        LendingBorrowing::<T>::withdraw_tokens(
            RawOrigin::Signed(caller_alice.clone()).into(),
            asset_id.into(),
        ).unwrap();
    }
    verify {
        // Should calculate total earnings

        assert_last_event::<T>(Event::<T>::UserWithdrewLendedTokens(caller_alice, asset_id.into(), balance!(0)).into());
    }
    */
}
