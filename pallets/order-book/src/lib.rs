// This file is part of the SORA network and Polkaswap app.

// Copyright (c) 2020, 2021, Polka Biome Ltd. All rights reserved.
// SPDX-License-Identifier: BSD-4-Clause

// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:

// Redistributions of source code must retain the above copyright notice, this list
// of conditions and the following disclaimer.
// Redistributions in binary form must reproduce the above copyright notice, this
// list of conditions and the following disclaimer in the documentation and/or other
// materials provided with the distribution.
//
// All advertising materials mentioning features or use of this software must display
// the following acknowledgement: This product includes software developed by Polka Biome
// Ltd., SORA, and Polkaswap.
//
// Neither the name of the Polka Biome Ltd. nor the names of its contributors may be used
// to endorse or promote products derived from this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY Polka Biome Ltd. AS IS AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL Polka Biome Ltd. BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
// BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
// STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)] // todo (m.tagirov) remove

use assets::AssetIdOf;
use common::prelude::{EnsureTradingPairExists, QuoteAmount, SwapAmount, SwapOutcome, TradingPair};
use common::{
    AssetInfoProvider, AssetName, AssetSymbol, Balance, BalancePrecision, ContentSource,
    Description, DexInfoProvider, LiquiditySource, PriceVariant, RewardReason,
    ToOrderTechUnitFromDEXAndTradingPair,
};
use core::fmt::Debug;
use frame_support::sp_runtime::DispatchError;
use frame_support::traits::{Get, Time};
use frame_support::weights::{Weight, WeightMeter};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeDisplay, One, Zero};
use sp_runtime::{Perbill, Saturating};
use sp_std::vec::Vec;

pub mod weights;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod cache_data_layer;
mod limit_order;
mod market_order;
mod order_book;
pub mod storage_data_layer;
pub mod traits;
pub mod types;

pub use crate::order_book::{OrderBook, OrderBookStatus};
use cache_data_layer::CacheDataLayer;
pub use limit_order::LimitOrder;
pub use market_order::MarketOrder;
pub use traits::{CurrencyLocker, CurrencyUnlocker, DataLayer};
pub use types::{MarketSide, OrderBookId, OrderPrice, OrderVolume, PriceOrders, UserOrders};
pub use weights::WeightInfo;

pub use pallet::*;

pub type MomentOf<T> = <<T as Config>::Time as Time>::Moment;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use common::DEXInfo;
    use frame_support::{
        pallet_prelude::{OptionQuery, *},
        traits::Hooks,
        Blake2_128Concat, Twox128,
    };
    use frame_system::pallet_prelude::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + assets::Config + technical::Config {
        const MAX_ORDER_LIFETIME: MomentOf<Self>;
        const MIN_ORDER_LIFETIME: MomentOf<Self>;
        const MILLISECS_PER_BLOCK: MomentOf<Self>;
        const MAX_PRICE_SHIFT: Perbill;

        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type OrderId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + AtLeast32BitUnsigned
            + Copy
            + Ord
            + PartialEq
            + Eq
            + MaxEncodedLen
            + scale_info::TypeInfo;
        type MaxOpenedLimitOrdersPerUser: Get<u32>;
        type MaxLimitOrdersForPrice: Get<u32>;
        type MaxSidePriceCount: Get<u32>;
        type MaxExpiringOrdersPerBlock: Get<u32>;
        type MaxExpirationWeightPerBlock: Get<Weight>;
        type EnsureTradingPairExists: EnsureTradingPairExists<
            Self::DEXId,
            Self::AssetId,
            DispatchError,
        >;
        type AssetInfoProvider: AssetInfoProvider<
            Self::AssetId,
            Self::AccountId,
            AssetSymbol,
            AssetName,
            BalancePrecision,
            ContentSource,
            Description,
        >;
        type DexInfoProvider: DexInfoProvider<Self::DEXId, DEXInfo<Self::AssetId>>;
        type Time: Time;
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    #[pallet::getter(fn order_books)]
    pub type OrderBooks<T: Config> =
        StorageMap<_, Blake2_128Concat, OrderBookId<AssetIdOf<T>>, OrderBook<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn limit_orders)]
    pub type LimitOrders<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        OrderBookId<AssetIdOf<T>>,
        Blake2_128Concat,
        T::OrderId,
        LimitOrder<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn bids)]
    pub type Bids<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        OrderBookId<AssetIdOf<T>>,
        Blake2_128Concat,
        OrderPrice,
        PriceOrders<T::OrderId, T::MaxLimitOrdersForPrice>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn asks)]
    pub type Asks<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        OrderBookId<AssetIdOf<T>>,
        Blake2_128Concat,
        OrderPrice,
        PriceOrders<T::OrderId, T::MaxLimitOrdersForPrice>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn aggregated_bids)]
    pub type AggregatedBids<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        OrderBookId<AssetIdOf<T>>,
        MarketSide<T::MaxSidePriceCount>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn aggregated_asks)]
    pub type AggregatedAsks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        OrderBookId<AssetIdOf<T>>,
        MarketSide<T::MaxSidePriceCount>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn user_limit_orders)]
    pub type UserLimitOrders<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        OrderBookId<AssetIdOf<T>>,
        UserOrders<T::OrderId, T::MaxOpenedLimitOrdersPerUser>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn expired_orders_at)]
    pub type ExpirationsAgenda<T: Config> = StorageMap<
        _,
        Twox128,
        T::BlockNumber,
        BoundedVec<(OrderBookId<AssetIdOf<T>>, T::OrderId), T::MaxExpiringOrdersPerBlock>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn incomplete_expirations_since)]
    pub type IncompleteExpirationsSince<T: Config> = StorageValue<_, T::BlockNumber>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New order book is created by user
        OrderBookCreated {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            dex_id: T::DEXId,
            creator: T::AccountId,
        },

        /// Order book is deleted
        OrderBookDeleted {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            dex_id: T::DEXId,
            count_of_canceled_orders: u32,
        },

        /// Order book attributes are updated
        OrderBookUpdated {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            dex_id: T::DEXId,
        },

        /// User placed new limit order
        OrderPlaced {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            dex_id: T::DEXId,
            order_id: T::OrderId,
            owner_id: T::AccountId,
        },

        /// User canceled their limit order
        OrderCanceled {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            dex_id: T::DEXId,
            order_id: T::OrderId,
            owner_id: T::AccountId,
        },

        /// The order has reached the end of its lifespan
        OrderExpired {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            dex_id: T::DEXId,
            order_id: T::OrderId,
            owner_id: T::AccountId,
        },

        /// Failed to cancel expired order
        ExpirationFailure {
            order_book_id: OrderBookId<AssetIdOf<T>>,
            order_id: T::OrderId,
            error: DispatchError,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Order book does not exist for this trading pair
        UnknownOrderBook,
        /// Order book already exists for this trading pair
        OrderBookAlreadyExists,
        /// Limit order does not exist for this trading pair and order id
        UnknownLimitOrder,
        /// Limit order already exists for this trading pair and order id
        LimitOrderAlreadyExists,
        /// It is impossible to insert the limit order because the bounds have been reached
        LimitOrderStorageOverflow,
        /// It is impossible to update the limit order
        UpdateLimitOrderError,
        /// It is impossible to delete the limit order
        DeleteLimitOrderError,
        /// Expiration schedule for expiration block is full
        BlockScheduleFull,
        /// Could not find expiration in given block schedule
        ExpirationNotFound,
        /// There are no bids/asks for the price
        NoDataForPrice,
        /// There are no aggregated bids/asks for the order book
        NoAggregatedData,
        /// There is not enough liquidity in the order book to cover the deal
        NotEnoughLiquidity,
        /// Cannot create order book with equal base and target assets
        ForbiddenToCreateOrderBookWithSameAssets,
        /// The asset is not allowed to be base. Only dex base asset can be a base asset for order book
        NotAllowedBaseAsset,
        /// User cannot create an order book with NFT if they don't have NFT
        UserHasNoNft,
        /// Lifespan exceeds defined limits
        InvalidLifespan,
        /// The order amount (limit or market) does not meet the requirements
        InvalidOrderAmount,
        /// The limit order price does not meet the requirements
        InvalidLimitOrderPrice,
        /// User cannot set the price of limit order too far from actual market price
        LimitOrderPriceIsTooFarFromSpread,
        /// At the moment, Trading is forbidden in the current order book
        TradingIsForbidden,
        /// At the moment, Users cannot place new limit orders in the current order book
        PlacementOfLimitOrdersIsForbidden,
        /// At the moment, Users cannot cancel their limit orders in the current order book
        CancellationOfLimitOrdersIsForbidden,
        /// User has the max available count of open limit orders in the current order book
        UserHasMaxCountOfOpenedOrders,
        /// It is impossible to place the limit order because bounds of the max count of orders at the current price have been reached
        PriceReachedMaxCountOfLimitOrders,
        /// It is impossible to place the limit order because bounds of the max count of prices for the side have been reached
        OrderBookReachedMaxCountOfPricesForSide,
        /// An error occurred while calculating the amount
        AmountCalculationFailed,
        /// Unauthorized action
        Unauthorized,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Execute the scheduled calls
        fn on_initialize(now: T::BlockNumber) -> Weight {
            let mut weight_counter = WeightMeter::from_limit(T::MaxExpirationWeightPerBlock::get());
            Self::service(now, &mut weight_counter);
            weight_counter.consumed
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::create_orderbook())]
        pub fn create_orderbook(
            origin: OriginFor<T>,
            dex_id: T::DEXId,
            order_book_id: OrderBookId<AssetIdOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                order_book_id.base != order_book_id.quote,
                Error::<T>::ForbiddenToCreateOrderBookWithSameAssets
            );
            let dex_info = T::DexInfoProvider::get_dex_info(&dex_id)?;
            // the base asset of DEX must be a quote asset of order book
            ensure!(
                order_book_id.quote == dex_info.base_asset_id,
                Error::<T>::NotAllowedBaseAsset
            );
            T::AssetInfoProvider::ensure_asset_exists(&order_book_id.base)?;
            T::EnsureTradingPairExists::ensure_trading_pair_exists(
                &dex_id,
                &order_book_id.quote.into(),
                &order_book_id.base.into(),
            )?;
            ensure!(
                !<OrderBooks<T>>::contains_key(order_book_id),
                Error::<T>::OrderBookAlreadyExists
            );

            let order_book = if T::AssetInfoProvider::get_asset_info(&order_book_id.base).2 != 0 {
                // regular asset
                OrderBook::<T>::default(order_book_id, dex_id)
            } else {
                // nft
                // ensure the user has nft
                ensure!(
                    T::AssetInfoProvider::total_balance(&order_book_id.base, &who)?
                        > Balance::zero(),
                    Error::<T>::UserHasNoNft
                );
                OrderBook::<T>::default_nft(order_book_id, dex_id)
            };

            <OrderBooks<T>>::insert(order_book_id, order_book);
            Self::register_tech_account(dex_id, order_book_id)?;

            Self::deposit_event(Event::<T>::OrderBookCreated {
                order_book_id,
                dex_id,
                creator: who,
            });
            Ok(().into())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::delete_orderbook())]
        pub fn delete_orderbook(
            origin: OriginFor<T>,
            order_book_id: OrderBookId<AssetIdOf<T>>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let order_book =
                <OrderBooks<T>>::get(order_book_id).ok_or(Error::<T>::UnknownOrderBook)?;
            let dex_id = order_book.dex_id;

            let mut data = CacheDataLayer::<T>::new();
            let count_of_canceled_orders =
                order_book.cancel_all_limit_orders::<Self>(&mut data)? as u32;

            data.commit();
            <OrderBooks<T>>::remove(order_book_id);

            Self::deregister_tech_account(order_book.dex_id, order_book_id)?;
            Self::deposit_event(Event::<T>::OrderBookDeleted {
                order_book_id,
                dex_id,
                count_of_canceled_orders,
            });
            Ok(().into())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::update_orderbook())]
        pub fn update_orderbook(
            origin: OriginFor<T>,
            order_book_id: OrderBookId<AssetIdOf<T>>,
            _tick_size: OrderPrice,
            _step_lot_size: OrderVolume,
            _min_lot_size: OrderVolume,
            _max_lot_size: OrderVolume,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                <OrderBooks<T>>::contains_key(order_book_id),
                Error::<T>::UnknownOrderBook
            );
            // todo (m.tagirov)
            todo!()
        }

        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::change_orderbook_status())]
        pub fn change_orderbook_status(
            origin: OriginFor<T>,
            _order_book_id: OrderBookId<AssetIdOf<T>>,
            _status: OrderBookStatus,
        ) -> DispatchResult {
            ensure_root(origin)?;
            // todo (m.tagirov)
            todo!()
        }

        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::place_limit_order())]
        pub fn place_limit_order(
            origin: OriginFor<T>,
            order_book_id: OrderBookId<AssetIdOf<T>>,
            price: OrderPrice,
            amount: OrderVolume,
            side: PriceVariant,
            lifespan: Option<MomentOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut order_book =
                <OrderBooks<T>>::get(order_book_id).ok_or(Error::<T>::UnknownOrderBook)?;
            let dex_id = order_book.dex_id;
            let order_id = order_book.next_order_id();
            let time = T::Time::now();
            let now = frame_system::Pallet::<T>::block_number();
            let lifespan = lifespan.unwrap_or(T::MAX_ORDER_LIFETIME);
            let order = LimitOrder::<T>::new(
                order_id,
                who.clone(),
                side,
                price,
                amount,
                time,
                lifespan,
                now,
            );
            let expires_at = order.expires_at;

            let mut data = CacheDataLayer::<T>::new();
            order_book.place_limit_order::<Self>(order, &mut data)?;

            data.commit();
            <OrderBooks<T>>::insert(order_book_id, order_book);
            Self::schedule(expires_at, order_book_id, order_id)
                .map_err(|e| <SchedulerError as Into<Error<T>>>::into(e))?;
            Self::deposit_event(Event::<T>::OrderPlaced {
                order_book_id,
                dex_id,
                order_id,
                owner_id: who,
            });
            Ok(().into())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::cancel_limit_order())]
        pub fn cancel_limit_order(
            origin: OriginFor<T>,
            order_book_id: OrderBookId<AssetIdOf<T>>,
            order_id: T::OrderId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut data = CacheDataLayer::<T>::new();
            let order = data.get_limit_order(&order_book_id, order_id)?;
            let expires_at = order.expires_at;

            ensure!(order.owner == who, Error::<T>::Unauthorized);

            let order_book =
                <OrderBooks<T>>::get(order_book_id).ok_or(Error::<T>::UnknownOrderBook)?;
            let dex_id = order_book.dex_id;

            order_book.cancel_limit_order::<Self>(order, &mut data)?;
            Self::unschedule(expires_at, order_book_id, order_id)
                .map_err(|e| <SchedulerError as Into<Error<T>>>::into(e))?;
            data.commit();
            Self::deposit_event(Event::<T>::OrderCanceled {
                order_book_id,
                dex_id,
                order_id,
                owner_id: who,
            });
            Ok(().into())
        }
    }
}

impl<T: Config> CurrencyLocker<T::AccountId, T::AssetId, T::DEXId, DispatchError> for Pallet<T> {
    fn lock_liquidity(
        dex_id: T::DEXId,
        account: &T::AccountId,
        order_book_id: OrderBookId<T::AssetId>,
        asset_id: &T::AssetId,
        amount: OrderVolume,
    ) -> Result<(), DispatchError> {
        let tech_account = Self::tech_account_for_order_book(dex_id, order_book_id);
        technical::Pallet::<T>::transfer_in(asset_id, account, &tech_account, amount.into())
    }
}

impl<T: Config> CurrencyUnlocker<T::AccountId, T::AssetId, T::DEXId, DispatchError> for Pallet<T> {
    fn unlock_liquidity(
        dex_id: T::DEXId,
        account: &T::AccountId,
        order_book_id: OrderBookId<T::AssetId>,
        asset_id: &T::AssetId,
        amount: OrderVolume,
    ) -> Result<(), DispatchError> {
        let tech_account = Self::tech_account_for_order_book(dex_id, order_book_id);
        technical::Pallet::<T>::transfer_out(asset_id, &tech_account, account, amount.into())
    }
}

impl<T: Config> Pallet<T> {
    pub fn tech_account_for_order_book(
        dex_id: T::DEXId,
        order_book_id: OrderBookId<AssetIdOf<T>>,
    ) -> <T as technical::Config>::TechAccountId {
        let trading_pair: TradingPair<AssetIdOf<T>> = order_book_id.into();
        // Same as in xyk accounts
        let tech_pair = trading_pair.map(|a| a.into());
        <T as technical::Config>::TechAccountId::to_order_tech_unit_from_dex_and_trading_pair(
            dex_id, tech_pair,
        )
    }

    // todo: make pub(tests) (k.ivanov)
    /// Validity of asset ids (for example, to have the same base asset
    /// for dex and pair) should be done beforehand
    pub fn register_tech_account(
        dex_id: T::DEXId,
        order_book_id: OrderBookId<AssetIdOf<T>>,
    ) -> Result<(), DispatchError> {
        let tech_account = Self::tech_account_for_order_book(dex_id, order_book_id);
        technical::Pallet::<T>::register_tech_account_id(tech_account)
    }

    // todo: make pub(tests) (k.ivanov)
    /// Validity of asset ids (for example, to have the same base asset
    /// for dex and pair) should be done beforehand
    pub fn deregister_tech_account(
        dex_id: T::DEXId,
        order_book_id: OrderBookId<AssetIdOf<T>>,
    ) -> Result<(), DispatchError> {
        let tech_account = Self::tech_account_for_order_book(dex_id, order_book_id);
        technical::Pallet::<T>::deregister_tech_account_id(tech_account)
    }
}

pub trait ExpirationScheduler<BlockNumber, OrderBookId, OrderId, Error> {
    fn service(now: BlockNumber, weight: &mut WeightMeter);
    fn schedule(
        when: BlockNumber,
        order_book_id: OrderBookId,
        order_id: OrderId,
    ) -> Result<(), Error>;
    fn unschedule(
        when: BlockNumber,
        order_book_id: OrderBookId,
        order_id: OrderId,
    ) -> Result<(), Error>;
}

enum SchedulerError {
    /// Expiration schedule for this block is full
    BlockScheduleFull,
    /// Could not find expiration in given block schedule
    ExpirationNotFound,
}

impl<T: Config> Into<Error<T>> for SchedulerError {
    fn into(self) -> Error<T> {
        match self {
            SchedulerError::BlockScheduleFull => Error::<T>::BlockScheduleFull,
            SchedulerError::ExpirationNotFound => Error::<T>::ExpirationNotFound,
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Try to consume the given weight `max_n` times. If weight is only
    /// enough to consume `n <= max_n` times, it consumes it `n` times
    /// and returns `n`.
    fn check_accrue_n(meter: &mut WeightMeter, w: Weight, max_n: u64) -> u64 {
        let n = {
            let weight_left = meter.remaining();
            // Maximum possible subtractions that we can do on each value
            // If None, then can subtract the value infinitely
            // thus we can use max value (more will likely be infeasible)
            let n_ref_time = weight_left
                .ref_time()
                .checked_div(w.ref_time())
                .unwrap_or(u64::MAX);
            let n_proof_size = weight_left
                .proof_size()
                .checked_div(w.proof_size())
                .unwrap_or(u64::MAX);
            let max_possible_n = n_ref_time.min(n_proof_size);
            max_possible_n.min(max_n)
        };
        // `n` was obtained as integer division `left/w`, so multiplying `n*w` will not exceed `left`;
        // it means it will fit into u64
        let to_consume = w.saturating_mul(n);
        meter.defensive_saturating_accrue(to_consume);
        n
    }

    fn service_single_expiration(
        data_layer: &mut CacheDataLayer<T>,
        order_book_id: OrderBookId<AssetIdOf<T>>,
        order_id: T::OrderId,
    ) {
        let Ok(order) = data_layer.get_limit_order(&order_book_id, order_id) else {
            debug_assert!(false, "apparently removal of order book or order did not cleanup expiration schedule");
            return;
        };
        let Some(order_book) = <OrderBooks<T>>::get(order_book_id) else {
            debug_assert!(false, "apparently removal of order book did not cleanup expiration schedule");
            return;
        };

        if let Err(error) = order_book.cancel_limit_order_unchecked::<Self>(order, data_layer) {
            Self::deposit_event(Event::<T>::ExpirationFailure {
                order_book_id,
                order_id,
                error,
            });
        }
    }

    /// Expire orders that are scheduled to expire at `block`.
    /// `weight` is used to track weight spent on the expirations, so that
    /// it doesn't accidentally spend weight of the entire block (or even more).
    ///
    /// Returns `true` if all expirations were processed and `false` if some expirations
    /// need to be retried with more available weight.
    fn service_block(
        data_layer: &mut CacheDataLayer<T>,
        block: T::BlockNumber,
        weight: &mut WeightMeter,
    ) -> bool {
        if !weight.check_accrue(<T as Config>::WeightInfo::service_base()) {
            return false;
        }

        let mut expired_orders = <ExpirationsAgenda<T>>::take(block);
        if expired_orders.is_empty() {
            return true;
        }
        // how many we can service with remaining weight;
        // the weight is consumed right away
        let to_service = Self::check_accrue_n(
            weight,
            <T as Config>::WeightInfo::service_single_expiration(),
            expired_orders.len() as u64,
        );
        let postponed = expired_orders.len() as u64 - to_service;
        let mut serviced = 0;
        while let Some((order_book_id, order_id)) = expired_orders.pop() {
            if serviced >= to_service {
                break;
            }
            Self::service_single_expiration(data_layer, order_book_id, order_id);
            serviced += 1;
        }
        postponed == 0
    }
}

impl<T: Config>
    ExpirationScheduler<T::BlockNumber, OrderBookId<AssetIdOf<T>>, T::OrderId, SchedulerError>
    for Pallet<T>
{
    fn service(now: T::BlockNumber, weight: &mut WeightMeter) {
        if !weight.check_accrue(<T as Config>::WeightInfo::service_block_base()) {
            return;
        }

        let mut incomplete_since = now + One::one();
        let mut when = IncompleteExpirationsSince::<T>::take().unwrap_or(now);

        let service_block_base_weight = <T as Config>::WeightInfo::service_block_base();
        let mut data_layer = CacheDataLayer::<T>::new();
        while when <= now && weight.can_accrue(service_block_base_weight) {
            if !Self::service_block(&mut data_layer, when, weight) {
                incomplete_since = incomplete_since.min(when);
            }
            when.saturating_inc();
        }
        incomplete_since = incomplete_since.min(when);
        if incomplete_since <= now {
            IncompleteExpirationsSince::<T>::put(incomplete_since);
        }
        data_layer.commit();
    }

    fn schedule(
        when: T::BlockNumber,
        order_book_id: OrderBookId<AssetIdOf<T>>,
        order_id: T::OrderId,
    ) -> Result<(), SchedulerError> {
        <ExpirationsAgenda<T>>::try_mutate(when, |block_expirations| {
            block_expirations
                .try_push((order_book_id, order_id))
                .map_err(|_| SchedulerError::BlockScheduleFull)
        })
    }

    fn unschedule(
        when: T::BlockNumber,
        order_book_id: OrderBookId<AssetIdOf<T>>,
        order_id: T::OrderId,
    ) -> Result<(), SchedulerError> {
        <ExpirationsAgenda<T>>::try_mutate(when, |block_expirations| {
            let Some(remove_index) = block_expirations.iter().position(|next| next == &(order_book_id, order_id)) else {
                return Err(SchedulerError::ExpirationNotFound);
            };
            block_expirations.remove(remove_index);
            Ok(())
        })
    }
}

impl<T: Config> LiquiditySource<T::DEXId, T::AccountId, T::AssetId, Balance, DispatchError>
    for Pallet<T>
{
    fn can_exchange(
        _dex_id: &T::DEXId,
        _input_asset_id: &T::AssetId,
        _output_asset_id: &T::AssetId,
    ) -> bool {
        // todo (m.tagirov)
        todo!()
    }

    fn quote(
        _dex_id: &T::DEXId,
        _input_asset_id: &T::AssetId,
        _output_asset_id: &T::AssetId,
        _amount: QuoteAmount<Balance>,
        _deduce_fee: bool,
    ) -> Result<(SwapOutcome<Balance>, Weight), DispatchError> {
        // todo (m.tagirov)
        todo!()
    }

    fn exchange(
        _sender: &T::AccountId,
        _receiver: &T::AccountId,
        _dex_id: &T::DEXId,
        _input_asset_id: &T::AssetId,
        _output_asset_id: &T::AssetId,
        _desired_amount: SwapAmount<Balance>,
    ) -> Result<(SwapOutcome<Balance>, Weight), DispatchError> {
        // todo (m.tagirov)
        todo!()
    }

    fn check_rewards(
        _dex_id: &T::DEXId,
        _input_asset_id: &T::AssetId,
        _output_asset_id: &T::AssetId,
        _input_amount: Balance,
        _output_amount: Balance,
    ) -> Result<(Vec<(Balance, T::AssetId, RewardReason)>, Weight), DispatchError> {
        Ok((Vec::new(), Weight::zero())) // no rewards for Order Book
    }

    fn quote_without_impact(
        _dex_id: &T::DEXId,
        _input_asset_id: &T::AssetId,
        _output_asset_id: &T::AssetId,
        _amount: QuoteAmount<Balance>,
        _deduce_fee: bool,
    ) -> Result<SwapOutcome<Balance>, DispatchError> {
        // todo (m.tagirov)
        todo!()
    }

    fn quote_weight() -> Weight {
        <T as Config>::WeightInfo::quote()
    }

    fn exchange_weight() -> Weight {
        <T as Config>::WeightInfo::exchange()
    }

    fn check_rewards_weight() -> Weight {
        Weight::zero()
    }
}
