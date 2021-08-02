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

use core::convert::TryInto;
use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::traits::Get;
use frame_support::{ensure, fail};

use common::prelude::{Balance, SwapAmount};
use common::{AccountIdOf, ToFeeAccount, ToTechUnitFromDEXAndTradingPair, TradingPair};

use crate::aliases::{AssetIdOf, DEXManager, TechAccountIdOf, TechAssetIdOf};
use crate::bounds::*;
use crate::{Config, Error, Module, PoolProviders, TotalIssuances};

impl<T: Config> Module<T> {
    /// Using try into to get Result with some error, after this convert Result into Option,
    /// after this AssetDecodingError is used if None.
    pub fn try_decode_asset(asset: AssetIdOf<T>) -> Result<TechAssetIdOf<T>, DispatchError> {
        TryInto::<TechAssetIdOf<T>>::try_into(asset)
            .map_err(|_| Error::<T>::AssetDecodingError.into())
    }

    pub fn decide_is_fee_from_destination(
        asset_a: &AssetIdOf<T>,
        asset_b: &AssetIdOf<T>,
    ) -> Result<bool, DispatchError> {
        let base_asset_id: T::AssetId = T::GetBaseAssetId::get();
        if &base_asset_id == asset_a {
            Ok(false)
        } else if &base_asset_id == asset_b {
            Ok(true)
        } else {
            Err(Error::<T>::UnsupportedQuotePath.into())
        }
    }

    pub fn get_fee_account(
        tech_acc: &TechAccountIdOf<T>,
    ) -> Result<TechAccountIdOf<T>, DispatchError> {
        let fee_acc = tech_acc
            .to_fee_account()
            .ok_or(Error::<T>::UnableToDeriveFeeAccount)?;
        Ok(fee_acc)
    }

    pub fn is_fee_account_valid_for(
        _asset_id: AssetIdOf<T>,
        tech_acc: &TechAccountIdOf<T>,
        fee_acc: &TechAccountIdOf<T>,
    ) -> DispatchResult {
        let recommended = Self::get_fee_account(tech_acc)?;
        if fee_acc != &recommended {
            Err(Error::<T>::FeeAccountIsInvalid)?;
        }
        Ok(())
    }

    pub fn is_pool_account_valid_for(
        _asset_id: AssetIdOf<T>,
        tech_acc: &TechAccountIdOf<T>,
    ) -> DispatchResult {
        technical::Module::<T>::ensure_tech_account_registered(tech_acc)?;
        //TODO: Maybe checking that asset and dex is exist, it is not really needed if
        //registration of technical account is a garanty that pair and dex exist.
        Ok(())
    }

    pub fn tech_account_from_dex_and_asset_pair(
        dex_id: T::DEXId,
        asset_a: T::AssetId,
        asset_b: T::AssetId,
    ) -> Result<(common::TradingPair<TechAssetIdOf<T>>, TechAccountIdOf<T>), DispatchError> {
        let dexinfo = DEXManager::<T>::get_dex_info(&dex_id)?;
        let base_asset_id = dexinfo.base_asset_id;
        ensure!(asset_a != asset_b, Error::<T>::AssetsMustNotBeSame);
        let ba = Module::<T>::try_decode_asset(base_asset_id)?;
        let ta = if base_asset_id == asset_a {
            Module::<T>::try_decode_asset(asset_b)?
        } else if base_asset_id == asset_b {
            Module::<T>::try_decode_asset(asset_a)?
        } else {
            Err(Error::<T>::BaseAssetIsNotMatchedWithAnyAssetArguments)?
        };
        let tpair = common::TradingPair::<TechAssetIdOf<T>> {
            base_asset_id: ba,
            target_asset_id: ta,
        };
        Ok((
            tpair,
            TechAccountIdOf::<T>::to_tech_unit_from_dex_and_trading_pair(dex_id, tpair),
        ))
    }

    pub fn get_bounds_from_swap_amount(
        swap_amount: SwapAmount<Balance>,
    ) -> Result<(Bounds<Balance>, Bounds<Balance>), DispatchError> {
        match swap_amount {
            SwapAmount::WithDesiredInput {
                desired_amount_in,
                min_amount_out,
            } => Ok((
                Bounds::Desired(desired_amount_in),
                Bounds::Min(min_amount_out),
            )),
            SwapAmount::WithDesiredOutput {
                desired_amount_out,
                max_amount_in,
            } => Ok((
                Bounds::Max(max_amount_in),
                Bounds::Desired(desired_amount_out),
            )),
        }
    }

    #[allow(dead_code)]
    fn get_bounded_asset_pair_for_liquidity(
        dex_id: T::DEXId,
        asset_a: T::AssetId,
        asset_b: T::AssetId,
        swap_amount_a: SwapAmount<Balance>,
        swap_amount_b: SwapAmount<Balance>,
    ) -> Result<
        (
            Bounds<Balance>,
            Bounds<Balance>,
            Bounds<Balance>,
            Bounds<Balance>,
            TechAccountIdOf<T>,
        ),
        DispatchError,
    > {
        let (_, tech_acc_id) =
            Module::<T>::tech_account_from_dex_and_asset_pair(dex_id, asset_a, asset_b)?;
        let (source_amount_a, destination_amount_a) =
            Module::<T>::get_bounds_from_swap_amount(swap_amount_a)?;
        let (source_amount_b, destination_amount_b) =
            Module::<T>::get_bounds_from_swap_amount(swap_amount_b)?;
        Ok((
            source_amount_a,
            destination_amount_a,
            source_amount_b,
            destination_amount_b,
            tech_acc_id,
        ))
    }

    pub fn burn(
        pool_account: &AccountIdOf<T>,
        user_account: &AccountIdOf<T>,
        pool_tokens: Balance,
    ) -> Result<(), DispatchError> {
        let result: Result<_, Error<T>> =
            PoolProviders::<T>::mutate_exists(pool_account, user_account, |balance| {
                let old_balance = balance.ok_or(Error::<T>::AccountBalanceIsInvalid)?;
                let new_balance = old_balance
                    .checked_sub(pool_tokens)
                    .ok_or(Error::<T>::AccountBalanceIsInvalid)?;
                *balance = (new_balance != 0).then(|| new_balance);
                Ok(())
            });
        result?;
        let result: Result<_, Error<T>> = TotalIssuances::<T>::mutate(pool_account, |issuance| {
            let old_issuance = issuance.ok_or(Error::<T>::PoolIsInvalid)?;
            let new_issuance = old_issuance
                .checked_sub(pool_tokens)
                .ok_or(Error::<T>::PoolIsInvalid)?;
            *issuance = Some(new_issuance);
            Ok(())
        });
        result?;
        Ok(())
    }

    pub fn mint(
        pool_account: &AccountIdOf<T>,
        user_account: &AccountIdOf<T>,
        pool_tokens: Balance,
    ) -> Result<(), DispatchError> {
        let result: Result<_, Error<T>> =
            PoolProviders::<T>::mutate(pool_account, user_account, |balance| {
                if balance.is_none() {
                    frame_system::Module::<T>::inc_consumers(user_account)
                        .map_err(|_| Error::<T>::IncRefError)?;
                }
                *balance = Some(balance.unwrap_or(0) + pool_tokens);
                Ok(())
            });
        result?;
        let result: Result<_, Error<T>> = TotalIssuances::<T>::mutate(&pool_account, |issuance| {
            let new_issuance = issuance
                .unwrap_or(0)
                .checked_add(pool_tokens)
                .ok_or(Error::<T>::PoolTokenSupplyOverflow)?;
            *issuance = Some(new_issuance);
            Ok(())
        });
        result?;
        Ok(())
    }

    /// Sort assets into base and target assets of trading pair, if none of assets is base then return error.
    pub fn strict_sort_pair(
        asset_a: &T::AssetId,
        asset_b: &T::AssetId,
    ) -> Result<TradingPair<T::AssetId>, DispatchError> {
        let base_asset_id = T::GetBaseAssetId::get();
        ensure!(asset_a != asset_b, Error::<T>::AssetsMustNotBeSame);
        if asset_a == &base_asset_id {
            Ok(TradingPair {
                base_asset_id: asset_a.clone(),
                target_asset_id: asset_b.clone(),
            })
        } else if asset_b == &base_asset_id {
            Ok(TradingPair {
                base_asset_id: asset_b.clone(),
                target_asset_id: asset_a.clone(),
            })
        } else {
            fail!(Error::<T>::BaseAssetIsNotMatchedWithAnyAssetArguments)
        }
    }
}
