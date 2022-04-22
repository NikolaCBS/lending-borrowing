//! Autogenerated weights for demeter_farming_platform
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2022-03-22, STEPS: [], REPEAT: 10, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("main-coded"), DB CACHE: 128

// Executed Command:
// target\release\framenode.exe
// benchmark
// --chain
// main-coded
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet
// demeter_farming_platform
// --extrinsic
// *
// --repeat
// 10
// --raw
// --output
// ./

#![allow(unused_parens)]
#![allow(unused_imports)]

use common::weights::constants::EXTRINSIC_FIXED_WEIGHT;
use frame_support::traits::Get;
use frame_support::weights::Weight;
use sp_std::marker::PhantomData;

/// Weight functions for demeter_farming_platform.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
    fn register_token() -> Weight {
        (65_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn add_pool() -> Weight {
        (87_800_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn deposit() -> Weight {
        (223_100_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(6 as Weight))
            .saturating_add(T::DbWeight::get().writes(5 as Weight))
    }
    fn get_rewards() -> Weight {
        (207_100_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn withdraw() -> Weight {
        (181_500_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(4 as Weight))
            .saturating_add(T::DbWeight::get().writes(4 as Weight))
    }
    fn remove_pool() -> Weight {
        (70_500_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    fn change_pool_multiplier() -> Weight {
        (89_700_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(2 as Weight))
    }
    fn change_pool_deposit_fee() -> Weight {
        (62_300_000 as Weight).saturating_add(T::DbWeight::get().reads(2 as Weight))
    }
    fn change_token_info() -> Weight {
        (69_400_000 as Weight)
            .saturating_add(T::DbWeight::get().reads(2 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

impl crate::WeightInfo for () {
    fn register_token() -> Weight {
        EXTRINSIC_FIXED_WEIGHT
    }
    fn add_pool() -> Weight {
        EXTRINSIC_FIXED_WEIGHT
    }
    fn deposit() -> Weight {
        2 * EXTRINSIC_FIXED_WEIGHT
    }
    fn get_rewards() -> Weight {
        2 * EXTRINSIC_FIXED_WEIGHT
    }
    fn withdraw() -> Weight {
        2 * EXTRINSIC_FIXED_WEIGHT
    }
    fn remove_pool() -> Weight {
        EXTRINSIC_FIXED_WEIGHT
    }
    fn change_pool_multiplier() -> Weight {
        EXTRINSIC_FIXED_WEIGHT
    }
    fn change_pool_deposit_fee() -> Weight {
        EXTRINSIC_FIXED_WEIGHT
    }
    fn change_token_info() -> Weight {
        EXTRINSIC_FIXED_WEIGHT
    }
}
