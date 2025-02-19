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

#![cfg(feature = "wip")] // order-book

use crate::tests::test_utils::*;
use assets::AssetIdOf;
use common::{balance, PriceVariant, DAI, VAL, XOR};
use frame_support::{assert_err, assert_ok};
use framenode_chain_spec::ext;
use framenode_runtime::order_book::{
    DealInfo, LimitOrder, MarketChange, OrderAmount, OrderBookId, Payment,
};
use framenode_runtime::Runtime;
use sp_std::collections::btree_map::BTreeMap;

#[test]
fn check_order_amount() {
    let base_balance = balance!(10).into();
    let quote_balance = balance!(11).into();

    let base = OrderAmount::Base(base_balance);
    let quote = OrderAmount::Quote(quote_balance);

    assert_eq!(*base.value(), base_balance);
    assert_eq!(*quote.value(), quote_balance);

    assert!(base.is_base());
    assert!(!quote.is_base());

    assert!(!base.is_quote());
    assert!(quote.is_quote());

    assert!(base.is_same(&base));
    assert!(quote.is_same(&quote));
    assert!(!base.is_same(&quote));
    assert!(!quote.is_same(&base));

    assert_eq!(
        base.copy_type(balance!(100).into()),
        OrderAmount::Base(balance!(100).into())
    );
    assert_eq!(
        quote.copy_type(balance!(110).into()),
        OrderAmount::Quote(balance!(110).into())
    );

    let order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
        dex_id: DEX.into(),
        base: VAL.into(),
        quote: XOR.into(),
    };

    assert_eq!(*base.associated_asset(&order_book_id), VAL);
    assert_eq!(*quote.associated_asset(&order_book_id), XOR);

    let base_balance2 = balance!(5).into();
    let quote_balance2 = balance!(6).into();

    let base2 = OrderAmount::Base(base_balance2);
    let quote2 = OrderAmount::Quote(quote_balance2);

    assert_eq!(
        (base + base2).unwrap(),
        OrderAmount::Base(base_balance + base_balance2)
    );
    assert_eq!(
        (quote + quote2).unwrap(),
        OrderAmount::Quote(quote_balance + quote_balance2)
    );
    assert_err!(base + quote, ());
    assert_err!(quote + base, ());

    assert_eq!(
        (base - base2).unwrap(),
        OrderAmount::Base(base_balance - base_balance2)
    );
    assert_eq!(
        (quote - quote2).unwrap(),
        OrderAmount::Quote(quote_balance - quote_balance2)
    );
    assert_err!(base - quote, ());
    assert_err!(quote - base, ());
}

#[test]
fn check_deal_info_valid() {
    // zero input amount
    assert!(!DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Quote(balance!(0).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Base(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(!DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Base(balance!(0).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Quote(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());

    // zero output amount
    assert!(!DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Quote(balance!(1).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Base(balance!(0).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(!DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Base(balance!(1).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Quote(balance!(0).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());

    // zero average price
    assert!(!DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Quote(balance!(1).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Base(balance!(2).into()),
        average_price: balance!(0).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(!DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Base(balance!(1).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Quote(balance!(2).into()),
        average_price: balance!(0).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());

    // equal assets
    assert!(!DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Quote(balance!(1).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Base(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(!DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Base(balance!(1).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Quote(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());

    // both are base
    assert!(!DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Base(balance!(1).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Base(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(!DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Base(balance!(1).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Base(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());

    // both are quote
    assert!(!DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Quote(balance!(1).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Quote(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(!DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Quote(balance!(1).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Quote(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());

    // valid
    assert!(DealInfo {
        input_asset_id: XOR,
        input_amount: OrderAmount::Quote(balance!(1).into()),
        output_asset_id: VAL,
        output_amount: OrderAmount::Base(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Buy
    }
    .is_valid());

    assert!(DealInfo {
        input_asset_id: VAL,
        input_amount: OrderAmount::Base(balance!(1).into()),
        output_asset_id: XOR,
        output_amount: OrderAmount::Quote(balance!(2).into()),
        average_price: balance!(0.5).into(),
        direction: PriceVariant::Sell
    }
    .is_valid());
}

#[test]
fn check_deal_info_amounts() {
    assert_eq!(
        DealInfo {
            input_asset_id: XOR,
            input_amount: OrderAmount::Quote(balance!(1).into()),
            output_asset_id: VAL,
            output_amount: OrderAmount::Base(balance!(2).into()),
            average_price: balance!(0.5).into(),
            direction: PriceVariant::Buy
        }
        .base_amount(),
        balance!(2).into()
    );

    assert_eq!(
        DealInfo {
            input_asset_id: VAL,
            input_amount: OrderAmount::Base(balance!(1).into()),
            output_asset_id: XOR,
            output_amount: OrderAmount::Quote(balance!(2).into()),
            average_price: balance!(0.5).into(),
            direction: PriceVariant::Sell
        }
        .base_amount(),
        balance!(1).into()
    );

    assert_eq!(
        DealInfo {
            input_asset_id: XOR,
            input_amount: OrderAmount::Quote(balance!(1).into()),
            output_asset_id: VAL,
            output_amount: OrderAmount::Base(balance!(2).into()),
            average_price: balance!(0.5).into(),
            direction: PriceVariant::Buy
        }
        .quote_amount(),
        balance!(1).into()
    );

    assert_eq!(
        DealInfo {
            input_asset_id: VAL,
            input_amount: OrderAmount::Base(balance!(1).into()),
            output_asset_id: XOR,
            output_amount: OrderAmount::Quote(balance!(2).into()),
            average_price: balance!(0.5).into(),
            direction: PriceVariant::Sell
        }
        .quote_amount(),
        balance!(2).into()
    );
}

#[test]
fn should_fail_payment_merge() {
    let order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
        dex_id: DEX.into(),
        base: VAL.into(),
        quote: XOR.into(),
    };

    let other_order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
        dex_id: DEX.into(),
        base: DAI.into(),
        quote: XOR.into(),
    };

    assert_err!(
        Payment {
            order_book_id,
            to_lock: BTreeMap::from([(XOR, BTreeMap::from([(alice(), balance!(100).into())]))]),
            to_unlock: BTreeMap::from([(VAL, BTreeMap::from([(bob(), balance!(50).into())]))])
        }
        .merge(&Payment {
            order_book_id: other_order_book_id,
            to_lock: BTreeMap::from([(XOR, BTreeMap::from([(alice(), balance!(100).into())]))]),
            to_unlock: BTreeMap::from([(DAI, BTreeMap::from([(bob(), balance!(50).into())]))])
        }),
        ()
    );
}

#[test]
fn check_payment_merge() {
    let order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
        dex_id: DEX.into(),
        base: VAL.into(),
        quote: XOR.into(),
    };

    let origin = Payment {
        order_book_id,
        to_lock: BTreeMap::from([
            (
                XOR,
                BTreeMap::from([(alice(), balance!(10).into()), (bob(), balance!(20).into())]),
            ),
            (
                VAL,
                BTreeMap::from([
                    (alice(), balance!(30).into()),
                    (charlie(), balance!(40).into()),
                ]),
            ),
        ]),
        to_unlock: BTreeMap::from([
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(50).into()),
                    (charlie(), balance!(60).into()),
                ]),
            ),
            (
                XOR,
                BTreeMap::from([(bob(), balance!(70).into()), (dave(), balance!(80).into())]),
            ),
        ]),
    };

    let different = Payment {
        order_book_id,
        to_lock: BTreeMap::from([
            (
                XOR,
                BTreeMap::from([
                    (charlie(), balance!(100).into()),
                    (dave(), balance!(110).into()),
                ]),
            ),
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(120).into()),
                    (dave(), balance!(130).into()),
                ]),
            ),
        ]),
        to_unlock: BTreeMap::from([
            (
                VAL,
                BTreeMap::from([
                    (alice(), balance!(140).into()),
                    (dave(), balance!(150).into()),
                ]),
            ),
            (
                XOR,
                BTreeMap::from([
                    (alice(), balance!(160).into()),
                    (charlie(), balance!(170).into()),
                ]),
            ),
        ]),
    };

    let mut payment = origin.clone();
    assert_ok!(payment.merge(&different));
    assert_eq!(
        payment,
        Payment {
            order_book_id,
            to_lock: BTreeMap::from([
                (
                    XOR,
                    BTreeMap::from([
                        (alice(), balance!(10).into()),
                        (bob(), balance!(20).into()),
                        (charlie(), balance!(100).into()),
                        (dave(), balance!(110).into())
                    ]),
                ),
                (
                    VAL,
                    BTreeMap::from([
                        (alice(), balance!(30).into()),
                        (bob(), balance!(120).into()),
                        (charlie(), balance!(40).into()),
                        (dave(), balance!(130).into())
                    ]),
                ),
            ]),
            to_unlock: BTreeMap::from([
                (
                    VAL,
                    BTreeMap::from([
                        (alice(), balance!(140).into()),
                        (bob(), balance!(50).into()),
                        (charlie(), balance!(60).into()),
                        (dave(), balance!(150).into())
                    ]),
                ),
                (
                    XOR,
                    BTreeMap::from([
                        (alice(), balance!(160).into()),
                        (bob(), balance!(70).into()),
                        (charlie(), balance!(170).into()),
                        (dave(), balance!(80).into())
                    ]),
                ),
            ]),
        }
    );

    let partial_match = Payment {
        order_book_id,
        to_lock: BTreeMap::from([
            (
                XOR,
                BTreeMap::from([
                    (alice(), balance!(200).into()),
                    (charlie(), balance!(210).into()),
                ]),
            ),
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(220).into()),
                    (charlie(), balance!(230).into()),
                ]),
            ),
        ]),
        to_unlock: BTreeMap::from([
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(240).into()),
                    (dave(), balance!(250).into()),
                ]),
            ),
            (
                XOR,
                BTreeMap::from([
                    (alice(), balance!(260).into()),
                    (dave(), balance!(270).into()),
                ]),
            ),
        ]),
    };

    payment = origin.clone();
    assert_ok!(payment.merge(&partial_match));
    assert_eq!(
        payment,
        Payment {
            order_book_id,
            to_lock: BTreeMap::from([
                (
                    XOR,
                    BTreeMap::from([
                        (alice(), balance!(210).into()),
                        (bob(), balance!(20).into()),
                        (charlie(), balance!(210).into())
                    ]),
                ),
                (
                    VAL,
                    BTreeMap::from([
                        (alice(), balance!(30).into()),
                        (bob(), balance!(220).into()),
                        (charlie(), balance!(270).into())
                    ]),
                ),
            ]),
            to_unlock: BTreeMap::from([
                (
                    VAL,
                    BTreeMap::from([
                        (bob(), balance!(290).into()),
                        (charlie(), balance!(60).into()),
                        (dave(), balance!(250).into())
                    ]),
                ),
                (
                    XOR,
                    BTreeMap::from([
                        (alice(), balance!(260).into()),
                        (bob(), balance!(70).into()),
                        (dave(), balance!(350).into())
                    ]),
                ),
            ]),
        }
    );

    let full_match = Payment {
        order_book_id,
        to_lock: BTreeMap::from([
            (
                XOR,
                BTreeMap::from([
                    (alice(), balance!(300).into()),
                    (bob(), balance!(310).into()),
                ]),
            ),
            (
                VAL,
                BTreeMap::from([
                    (alice(), balance!(320).into()),
                    (charlie(), balance!(330).into()),
                ]),
            ),
        ]),
        to_unlock: BTreeMap::from([
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(340).into()),
                    (charlie(), balance!(350).into()),
                ]),
            ),
            (
                XOR,
                BTreeMap::from([
                    (bob(), balance!(360).into()),
                    (dave(), balance!(370).into()),
                ]),
            ),
        ]),
    };

    payment = origin.clone();
    assert_ok!(payment.merge(&full_match));
    assert_eq!(
        payment,
        Payment {
            order_book_id,
            to_lock: BTreeMap::from([
                (
                    XOR,
                    BTreeMap::from([
                        (alice(), balance!(310).into()),
                        (bob(), balance!(330).into())
                    ]),
                ),
                (
                    VAL,
                    BTreeMap::from([
                        (alice(), balance!(350).into()),
                        (charlie(), balance!(370).into())
                    ]),
                ),
            ]),
            to_unlock: BTreeMap::from([
                (
                    VAL,
                    BTreeMap::from([
                        (bob(), balance!(390).into()),
                        (charlie(), balance!(410).into())
                    ]),
                ),
                (
                    XOR,
                    BTreeMap::from([
                        (bob(), balance!(430).into()),
                        (dave(), balance!(450).into())
                    ]),
                ),
            ]),
        }
    );

    let empty = Payment {
        order_book_id,
        to_lock: BTreeMap::new(),
        to_unlock: BTreeMap::new(),
    };

    payment = origin.clone();
    assert_ok!(payment.merge(&empty));
    assert_eq!(payment, origin);
}

#[test]
fn check_payment_execute_all() {
    ext().execute_with(|| {
        let order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
            dex_id: DEX.into(),
            base: VAL.into(),
            quote: XOR.into(),
        };

        OrderBookPallet::register_tech_account(order_book_id).unwrap();

        fill_balance(alice(), order_book_id);
        fill_balance(bob(), order_book_id);
        fill_balance(charlie(), order_book_id);
        fill_balance(dave(), order_book_id);

        let balance_diff = balance!(150);

        let alice_base_balance = free_balance(&order_book_id.base, &alice());
        let alice_quote_balance = free_balance(&order_book_id.quote, &alice());
        let bob_base_balance = free_balance(&order_book_id.base, &bob());
        let bob_quote_balance = free_balance(&order_book_id.quote, &bob());
        let charlie_base_balance = free_balance(&order_book_id.base, &charlie());
        let charlie_quote_balance = free_balance(&order_book_id.quote, &charlie());
        let dave_base_balance = free_balance(&order_book_id.base, &dave());
        let dave_quote_balance = free_balance(&order_book_id.quote, &dave());

        let payment = Payment {
            order_book_id,
            to_lock: BTreeMap::from([
                (
                    order_book_id.base,
                    BTreeMap::from([(alice(), balance_diff.into())]),
                ),
                (
                    order_book_id.quote,
                    BTreeMap::from([(bob(), balance_diff.into())]),
                ),
            ]),
            to_unlock: BTreeMap::from([
                (
                    order_book_id.base,
                    BTreeMap::from([(charlie(), balance_diff.into())]),
                ),
                (
                    order_book_id.quote,
                    BTreeMap::from([(dave(), balance_diff.into())]),
                ),
            ]),
        };

        assert_ok!(payment.execute_all::<OrderBookPallet, OrderBookPallet>());

        assert_eq!(
            alice_base_balance - balance_diff,
            free_balance(&order_book_id.base, &alice())
        );
        assert_eq!(
            alice_quote_balance,
            free_balance(&order_book_id.quote, &alice())
        );
        assert_eq!(bob_base_balance, free_balance(&order_book_id.base, &bob()));
        assert_eq!(
            bob_quote_balance - balance_diff,
            free_balance(&order_book_id.quote, &bob())
        );
        assert_eq!(
            charlie_base_balance + balance_diff,
            free_balance(&order_book_id.base, &charlie())
        );
        assert_eq!(
            charlie_quote_balance,
            free_balance(&order_book_id.quote, &charlie())
        );
        assert_eq!(
            dave_base_balance,
            free_balance(&order_book_id.base, &dave())
        );
        assert_eq!(
            dave_quote_balance + balance_diff,
            free_balance(&order_book_id.quote, &dave())
        );
    });
}

#[test]
fn should_fail_market_change_merge() {
    let order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
        dex_id: DEX.into(),
        base: VAL.into(),
        quote: XOR.into(),
    };

    let payment = Payment {
        order_book_id,
        to_lock: BTreeMap::from([
            (
                XOR,
                BTreeMap::from([(alice(), balance!(10).into()), (bob(), balance!(20).into())]),
            ),
            (
                VAL,
                BTreeMap::from([
                    (alice(), balance!(30).into()),
                    (charlie(), balance!(40).into()),
                ]),
            ),
        ]),
        to_unlock: BTreeMap::from([
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(50).into()),
                    (charlie(), balance!(60).into()),
                ]),
            ),
            (
                XOR,
                BTreeMap::from([(bob(), balance!(70).into()), (dave(), balance!(80).into())]),
            ),
        ]),
    };

    let origin = MarketChange {
        deal_input: None,
        deal_output: None,
        market_input: None,
        market_output: None,
        to_place: BTreeMap::from([(
            5,
            LimitOrder::<Runtime>::new(
                5,
                alice(),
                PriceVariant::Buy,
                balance!(10).into(),
                balance!(100).into(),
                1000,
                10000,
                100,
            ),
        )]),
        to_part_execute: BTreeMap::from([(
            4,
            (
                LimitOrder::<Runtime>::new(
                    4,
                    alice(),
                    PriceVariant::Buy,
                    balance!(20).into(),
                    balance!(100).into(),
                    1000,
                    10000,
                    100,
                ),
                OrderAmount::Base(balance!(10).into()),
            ),
        )]),
        to_full_execute: BTreeMap::from([(
            3,
            LimitOrder::<Runtime>::new(
                3,
                alice(),
                PriceVariant::Buy,
                balance!(20).into(),
                balance!(100).into(),
                1000,
                10000,
                100,
            ),
        )]),
        to_cancel: BTreeMap::from([(
            2,
            LimitOrder::<Runtime>::new(
                2,
                alice(),
                PriceVariant::Buy,
                balance!(10).into(),
                balance!(100).into(),
                1000,
                10000,
                100,
            ),
        )]),
        to_force_update: BTreeMap::from([(
            1,
            LimitOrder::<Runtime>::new(
                1,
                alice(),
                PriceVariant::Buy,
                balance!(10).into(),
                balance!(100).into(),
                1000,
                10000,
                100,
            ),
        )]),
        payment,
        ignore_unschedule_error: false,
    };

    let mut market_change = origin.clone();
    market_change.deal_input = Some(OrderAmount::Base(balance!(100).into()));
    market_change.deal_output = Some(OrderAmount::Quote(balance!(200).into()));
    market_change.market_input = Some(OrderAmount::Base(balance!(300).into()));
    market_change.market_output = Some(OrderAmount::Quote(balance!(400).into()));

    let mut diff_deal_input = origin.clone();
    diff_deal_input.deal_input = Some(OrderAmount::Quote(balance!(50).into()));
    assert_err!(market_change.merge(diff_deal_input), ());

    let mut diff_deal_output = origin.clone();
    diff_deal_output.deal_output = Some(OrderAmount::Base(balance!(50).into()));
    assert_err!(market_change.merge(diff_deal_output), ());

    let mut diff_market_input = origin.clone();
    diff_market_input.market_input = Some(OrderAmount::Quote(balance!(50).into()));
    assert_err!(market_change.merge(diff_market_input), ());

    let mut diff_market_output = origin.clone();
    diff_market_output.market_output = Some(OrderAmount::Base(balance!(50).into()));
    assert_err!(market_change.merge(diff_market_output), ());
}

#[test]
fn check_market_change_merge() {
    let order_book_id = OrderBookId::<AssetIdOf<Runtime>, DEXId> {
        dex_id: DEX.into(),
        base: VAL.into(),
        quote: XOR.into(),
    };

    let payment = Payment {
        order_book_id,
        to_lock: BTreeMap::from([
            (
                XOR,
                BTreeMap::from([(alice(), balance!(10).into()), (bob(), balance!(20).into())]),
            ),
            (
                VAL,
                BTreeMap::from([
                    (alice(), balance!(30).into()),
                    (charlie(), balance!(40).into()),
                ]),
            ),
        ]),
        to_unlock: BTreeMap::from([
            (
                VAL,
                BTreeMap::from([
                    (bob(), balance!(50).into()),
                    (charlie(), balance!(60).into()),
                ]),
            ),
            (
                XOR,
                BTreeMap::from([(bob(), balance!(70).into()), (dave(), balance!(80).into())]),
            ),
        ]),
    };

    let empty_payment = Payment {
        order_book_id,
        to_lock: BTreeMap::new(),
        to_unlock: BTreeMap::new(),
    };

    let order_id1 = 101;
    let order_id2 = 102;
    let order_id3 = 103;
    let order_id4 = 104;
    let order_id5 = 105;

    let order1_origin = LimitOrder::<Runtime>::new(
        order_id1,
        alice(),
        PriceVariant::Buy,
        balance!(10).into(),
        balance!(100).into(),
        1000,
        10000,
        100,
    );

    let order1_other = LimitOrder::<Runtime>::new(
        order_id1,
        alice(),
        PriceVariant::Buy,
        balance!(9).into(),
        balance!(1000).into(),
        1000,
        10000,
        100,
    );

    let order2_origin = LimitOrder::<Runtime>::new(
        order_id2,
        bob(),
        PriceVariant::Sell,
        balance!(15).into(),
        balance!(100).into(),
        1000,
        10000,
        100,
    );

    let order2_other = LimitOrder::<Runtime>::new(
        order_id2,
        bob(),
        PriceVariant::Buy,
        balance!(14).into(),
        balance!(200).into(),
        1000,
        10000,
        100,
    );

    let order3_origin = LimitOrder::<Runtime>::new(
        order_id3,
        charlie(),
        PriceVariant::Buy,
        balance!(11).into(),
        balance!(100).into(),
        1000,
        10000,
        100,
    );

    let order3_other = LimitOrder::<Runtime>::new(
        order_id3,
        charlie(),
        PriceVariant::Buy,
        balance!(12).into(),
        balance!(1000).into(),
        1000,
        10000,
        100,
    );

    let order4_origin = LimitOrder::<Runtime>::new(
        order_id4,
        dave(),
        PriceVariant::Sell,
        balance!(16).into(),
        balance!(100).into(),
        1000,
        10000,
        100,
    );

    let order5_origin = LimitOrder::<Runtime>::new(
        order_id5,
        alice(),
        PriceVariant::Buy,
        balance!(12).into(),
        balance!(100).into(),
        1000,
        10000,
        100,
    );

    let origin = MarketChange {
        deal_input: Some(OrderAmount::Base(balance!(1000).into())),
        deal_output: Some(OrderAmount::Quote(balance!(2000).into())),
        market_input: Some(OrderAmount::Base(balance!(3000).into())),
        market_output: Some(OrderAmount::Quote(balance!(4000).into())),
        to_place: BTreeMap::from([
            (order_id1, order1_origin.clone()),
            (order_id2, order2_origin.clone()),
            (order_id3, order3_origin.clone()),
        ]),
        to_part_execute: BTreeMap::from([
            (
                order_id1,
                (
                    order1_origin.clone(),
                    OrderAmount::Base(balance!(20).into()),
                ),
            ),
            (
                order_id2,
                (
                    order2_origin.clone(),
                    OrderAmount::Base(balance!(30).into()),
                ),
            ),
            (
                order_id3,
                (
                    order3_origin.clone(),
                    OrderAmount::Base(balance!(40).into()),
                ),
            ),
        ]),
        to_full_execute: BTreeMap::from([
            (order_id1, order1_origin.clone()),
            (order_id2, order2_origin.clone()),
            (order_id3, order3_origin.clone()),
        ]),
        to_cancel: BTreeMap::from([
            (order_id1, order1_origin.clone()),
            (order_id2, order2_origin.clone()),
            (order_id3, order3_origin.clone()),
        ]),
        to_force_update: BTreeMap::from([
            (order_id1, order1_origin.clone()),
            (order_id2, order2_origin.clone()),
            (order_id3, order3_origin.clone()),
        ]),
        payment: payment.clone(),
        ignore_unschedule_error: false,
    };

    let different = MarketChange {
        deal_input: None,
        deal_output: None,
        market_input: None,
        market_output: None,
        to_place: BTreeMap::from([
            (order_id4, order4_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        to_part_execute: BTreeMap::from([
            (
                order_id4,
                (
                    order4_origin.clone(),
                    OrderAmount::Base(balance!(50).into()),
                ),
            ),
            (
                order_id5,
                (
                    order5_origin.clone(),
                    OrderAmount::Base(balance!(60).into()),
                ),
            ),
        ]),
        to_full_execute: BTreeMap::from([
            (order_id4, order4_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        to_cancel: BTreeMap::from([
            (order_id4, order4_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        to_force_update: BTreeMap::from([
            (order_id4, order4_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        payment: empty_payment.clone(),
        ignore_unschedule_error: false,
    };

    let mut market_change = origin.clone();
    assert_ok!(market_change.merge(different));
    assert_eq!(
        market_change,
        MarketChange {
            deal_input: Some(OrderAmount::Base(balance!(1000).into())),
            deal_output: Some(OrderAmount::Quote(balance!(2000).into())),
            market_input: Some(OrderAmount::Base(balance!(3000).into())),
            market_output: Some(OrderAmount::Quote(balance!(4000).into())),
            to_place: BTreeMap::from([
                (order_id1, order1_origin.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id4, order4_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            to_part_execute: BTreeMap::from([
                (
                    order_id1,
                    (
                        order1_origin.clone(),
                        OrderAmount::Base(balance!(20).into())
                    )
                ),
                (
                    order_id2,
                    (
                        order2_origin.clone(),
                        OrderAmount::Base(balance!(30).into())
                    )
                ),
                (
                    order_id3,
                    (
                        order3_origin.clone(),
                        OrderAmount::Base(balance!(40).into())
                    )
                ),
                (
                    order_id4,
                    (
                        order4_origin.clone(),
                        OrderAmount::Base(balance!(50).into())
                    )
                ),
                (
                    order_id5,
                    (
                        order5_origin.clone(),
                        OrderAmount::Base(balance!(60).into())
                    )
                ),
            ]),
            to_full_execute: BTreeMap::from([
                (order_id1, order1_origin.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id4, order4_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            to_cancel: BTreeMap::from([
                (order_id1, order1_origin.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id4, order4_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            to_force_update: BTreeMap::from([
                (order_id1, order1_origin.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id4, order4_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            payment: payment.clone(),
            ignore_unschedule_error: false
        }
    );

    let partial_match = MarketChange {
        deal_input: Some(OrderAmount::Base(balance!(7000).into())),
        deal_output: Some(OrderAmount::Quote(balance!(8000).into())),
        market_input: None,
        market_output: None,
        to_place: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        to_part_execute: BTreeMap::from([
            (
                order_id1,
                (
                    order1_other.clone(),
                    OrderAmount::Base(balance!(120).into()),
                ),
            ),
            (
                order_id2,
                (
                    order2_origin.clone(),
                    OrderAmount::Base(balance!(30).into()),
                ),
            ),
            (
                order_id5,
                (
                    order5_origin.clone(),
                    OrderAmount::Base(balance!(60).into()),
                ),
            ),
        ]),
        to_full_execute: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        to_cancel: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        to_force_update: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_origin.clone()),
            (order_id5, order5_origin.clone()),
        ]),
        payment: empty_payment.clone(),
        ignore_unschedule_error: false,
    };

    market_change = origin.clone();
    assert_ok!(market_change.merge(partial_match));
    assert_eq!(
        market_change,
        MarketChange {
            deal_input: Some(OrderAmount::Base(balance!(8000).into())),
            deal_output: Some(OrderAmount::Quote(balance!(10000).into())),
            market_input: Some(OrderAmount::Base(balance!(3000).into())),
            market_output: Some(OrderAmount::Quote(balance!(4000).into())),
            to_place: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            to_part_execute: BTreeMap::from([
                (
                    order_id1,
                    (
                        order1_other.clone(),
                        OrderAmount::Base(balance!(120).into())
                    )
                ),
                (
                    order_id2,
                    (
                        order2_origin.clone(),
                        OrderAmount::Base(balance!(30).into())
                    )
                ),
                (
                    order_id3,
                    (
                        order3_origin.clone(),
                        OrderAmount::Base(balance!(40).into())
                    )
                ),
                (
                    order_id5,
                    (
                        order5_origin.clone(),
                        OrderAmount::Base(balance!(60).into())
                    )
                ),
            ]),
            to_full_execute: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            to_cancel: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            to_force_update: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_origin.clone()),
                (order_id3, order3_origin.clone()),
                (order_id5, order5_origin.clone()),
            ]),
            payment: payment.clone(),
            ignore_unschedule_error: false
        }
    );

    let full_match = MarketChange {
        deal_input: Some(OrderAmount::Base(balance!(1000).into())),
        deal_output: Some(OrderAmount::Quote(balance!(2000).into())),
        market_input: Some(OrderAmount::Base(balance!(3000).into())),
        market_output: Some(OrderAmount::Quote(balance!(4000).into())),
        to_place: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_other.clone()),
            (order_id3, order3_other.clone()),
        ]),
        to_part_execute: BTreeMap::from([
            (
                order_id1,
                (
                    order1_other.clone(),
                    OrderAmount::Base(balance!(120).into()),
                ),
            ),
            (
                order_id2,
                (
                    order2_other.clone(),
                    OrderAmount::Base(balance!(130).into()),
                ),
            ),
            (
                order_id3,
                (
                    order3_other.clone(),
                    OrderAmount::Base(balance!(140).into()),
                ),
            ),
        ]),
        to_full_execute: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_other.clone()),
            (order_id3, order3_other.clone()),
        ]),
        to_cancel: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_other.clone()),
            (order_id3, order3_other.clone()),
        ]),
        to_force_update: BTreeMap::from([
            (order_id1, order1_other.clone()),
            (order_id2, order2_other.clone()),
            (order_id3, order3_other.clone()),
        ]),
        payment: empty_payment.clone(),
        ignore_unschedule_error: false,
    };

    market_change = origin.clone();
    assert_ok!(market_change.merge(full_match));
    assert_eq!(
        market_change,
        MarketChange {
            deal_input: Some(OrderAmount::Base(balance!(2000).into())),
            deal_output: Some(OrderAmount::Quote(balance!(4000).into())),
            market_input: Some(OrderAmount::Base(balance!(6000).into())),
            market_output: Some(OrderAmount::Quote(balance!(8000).into())),
            to_place: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_other.clone()),
                (order_id3, order3_other.clone()),
            ]),
            to_part_execute: BTreeMap::from([
                (
                    order_id1,
                    (
                        order1_other.clone(),
                        OrderAmount::Base(balance!(120).into())
                    )
                ),
                (
                    order_id2,
                    (
                        order2_other.clone(),
                        OrderAmount::Base(balance!(130).into())
                    )
                ),
                (
                    order_id3,
                    (
                        order3_other.clone(),
                        OrderAmount::Base(balance!(140).into())
                    )
                ),
            ]),
            to_full_execute: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_other.clone()),
                (order_id3, order3_other.clone()),
            ]),
            to_cancel: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_other.clone()),
                (order_id3, order3_other.clone()),
            ]),
            to_force_update: BTreeMap::from([
                (order_id1, order1_other.clone()),
                (order_id2, order2_other.clone()),
                (order_id3, order3_other.clone()),
            ]),
            payment: payment.clone(),
            ignore_unschedule_error: false
        }
    );

    let empty = MarketChange {
        deal_input: None,
        deal_output: None,
        market_input: None,
        market_output: None,
        to_place: BTreeMap::new(),
        to_part_execute: BTreeMap::new(),
        to_full_execute: BTreeMap::new(),
        to_cancel: BTreeMap::new(),
        to_force_update: BTreeMap::new(),
        payment: empty_payment.clone(),
        ignore_unschedule_error: false,
    };

    market_change = origin.clone();
    assert_ok!(market_change.merge(empty));
    assert_eq!(market_change, origin);
}
