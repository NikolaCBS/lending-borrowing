use frame_support::sp_runtime::traits::BadOrigin;
use frame_support::sp_runtime::DispatchError;
use frame_support::traits::Get;
use frame_support::Parameter;
use frame_system::RawOrigin;

/// Check on origin that it is a DEX owner.
pub trait EnsureDEXOwner<DEXId, AccountId, Error> {
    fn ensure_dex_owner<OuterOrigin>(
        dex_id: &DEXId,
        origin: OuterOrigin,
    ) -> Result<Option<AccountId>, Error>
    where
        OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>>;
}

impl<DEXId, AccountId> EnsureDEXOwner<DEXId, AccountId, DispatchError> for () {
    fn ensure_dex_owner<OuterOrigin>(
        _dex_id: &DEXId,
        origin: OuterOrigin,
    ) -> Result<Option<AccountId>, DispatchError>
    where
        OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>>,
    {
        match origin.into() {
            Ok(RawOrigin::Signed(t)) => Ok(Some(t)),
            Ok(RawOrigin::Root) => Ok(None),
            _ => Err(BadOrigin.into()),
        }
    }
}

pub type AssetIdOf<T> = <T as Trait>::AssetId;
pub type AccountIdOf<T> = <T as frame_system::Trait>::AccountId;

/// Common DEX trait. Used for DEX-related pallets.
pub trait Trait: frame_system::Trait + currencies::Trait {
    /// DEX identifier.
    type DEXId: Parameter;
    /// DEX assets (currency) identifier.
    type AssetId: Parameter + Ord + Default;
    /// The base asset as the core asset in all trading pairs
    type GetBaseAssetId: Get<AssetIdOf<Self>>;
    /// Performs checks for origin is a DEX owner.
    type EnsureDEXOwner: EnsureDEXOwner<Self::DEXId, Self::AccountId, DispatchError>;

    fn ensure_dex_owner<OuterOrigin>(
        dex_id: &Self::DEXId,
        origin: OuterOrigin,
    ) -> Result<Option<Self::AccountId>, DispatchError>
    where
        OuterOrigin: Into<Result<RawOrigin<Self::AccountId>, OuterOrigin>>,
    {
        Self::EnsureDEXOwner::ensure_dex_owner(dex_id, origin)
    }
}
