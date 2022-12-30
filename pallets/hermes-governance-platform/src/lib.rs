#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use codec::{Decode, Encode};
use common::Balance;

#[derive(Encode, Decode, Default, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct HermesVotingInfo {
    /// Voting option
    voting_option: u32,
    /// Number of hermes
    number_of_hermes: Balance,
    /// Hermes withdrawn
    hermes_withdrawn: bool,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct HermesPollInfo<AccountId, Moment> {
    /// Creator of poll
    pub creator: AccountId,
    /// Hermes Locked
    pub hermes_locked: Balance,
    /// Poll start timestamp
    pub poll_start_timestamp: Moment,
    /// Poll end timestamp
    pub poll_end_timestamp: Moment,
    /// Poll title
    pub title: String,
    /// Description
    pub description: String,
    /// Creator Hermes withdrawn
    pub creator_hermes_withdrawn: bool,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use crate::{HermesPollInfo, HermesVotingInfo};
    use common::balance;
    use common::prelude::Balance;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::AccountIdConversion;
    use frame_support::transactional;
    use frame_support::PalletId;
    use frame_system::ensure_signed;
    use frame_system::pallet_prelude::*;
    use hex_literal::hex;
    use pallet_timestamp as timestamp;
    use uuid::Uuid;

    const PALLET_ID: PalletId = PalletId(*b"hermsgov");

    #[pallet::config]
    pub trait Config:
        frame_system::Config + assets::Config + technical::Config + timestamp::Config
    {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Hermes asset id
        type HermesAssetId: Get<Self::AssetId>;
    }

    type Assets<T> = assets::Pallet<T>;
    pub type Timestamp<T> = timestamp::Pallet<T>;
    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    /// A vote of a particular user for a particular poll
    #[pallet::storage]
    #[pallet::getter(fn hermes_votings)]
    pub type HermesVotings<T: Config> = StorageDoubleMap<
        _,
        Identity,
        String,
        Identity,
        AccountIdOf<T>,
        HermesVotingInfo,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn hermes_poll_data)]
    pub type HermesPollData<T: Config> =
        StorageMap<_, Identity, String, HermesPollInfo<AccountIdOf<T>, T::Moment>, OptionQuery>;

    #[pallet::type_value]
    pub fn DefaultMinimumHermesVotingAmount<T: Config>() -> Balance {
        balance!(100)
    }

    #[pallet::storage]
    #[pallet::getter(fn min_hermes_for_voting)]
    pub type MinimumHermesVotingAmount<T: Config> =
        StorageValue<_, Balance, ValueQuery, DefaultMinimumHermesVotingAmount<T>>;

    #[pallet::type_value]
    pub fn DefaultMinimumHermesAmountForCreatingPoll<T: Config>() -> Balance {
        balance!(200)
    }

    #[pallet::storage]
    #[pallet::getter(fn min_hermes_for_creating_poll)]
    pub type MinimumHermesAmountForCreatingPoll<T: Config> =
        StorageValue<_, Balance, ValueQuery, DefaultMinimumHermesAmountForCreatingPoll<T>>;

    #[pallet::type_value]
    pub fn DefaultForAuthorityAccount<T: Config>() -> AccountIdOf<T> {
        let bytes = hex!("96ea3c9c0be7bbc7b0656a1983db5eed75210256891a9609012362e36815b132");
        AccountIdOf::<T>::decode(&mut &bytes[..]).unwrap()
    }

    /// Account which has permissions for changing Hermes minimum amount for voting and creating a poll
    #[pallet::storage]
    #[pallet::getter(fn authority_account)]
    pub type AuthorityAccount<T: Config> =
        StorageValue<_, AccountIdOf<T>, ValueQuery, DefaultForAuthorityAccount<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Voting [who, poll, option]
        Voted(AccountIdOf<T>, String, u32),
        /// Create poll [who, start_timestamp, end_timestamp]
        Created(AccountIdOf<T>, T::Moment, T::Moment),
        /// Voter Funds Withdrawn [who, balance]
        VoterFundsWithdrawn(AccountIdOf<T>, Balance),
        /// Creator Funds Withdrawn [who, balance]
        CreatorFundsWithdrawn(AccountIdOf<T>, Balance),
        /// Change minimum Hermes for voting [balance]
        MinimumHermesForVotingChanged(Balance),
        /// Change minimum Hermes for creating poll [balance]
        MinimumHermesForCreatingPollChanged(Balance),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Invalid Number Of Hermes
        InvalidAmountOfHermes,
        /// Poll Is Not Started
        PollIsNotStarted,
        ///Poll Is Finished
        PollIsFinished,
        /// Invalid Number Of Option
        InvalidNumberOfOption,
        /// Not Enough Funds
        NotEnoughFunds,
        /// Invalid Start Timestamp
        InvalidStartTimestamp,
        ///Invalid End Timestamp,
        InvalidEndTimestamp,
        /// Not Enough Hermes For Creating Poll
        NotEnoughHermesForCreatingPoll,
        /// Funds Already Withdrawn
        FundsAlreadyWithdrawn,
        /// Invalid Voting Option
        InvalidVotingOption,
        /// Not Enough Funds For CreatingPoll
        NotEnoughFundsForCreatingPoll,
        /// Poll Is Not Finished
        PollIsNotFinished,
        /// Poll Is Not Ended
        PollIsNotEnded,
        /// Creator Funds Already Withdrawn
        CreatorFundsAlreadyWithdrawn,
        /// You Are Not Creator
        YouAreNotCreator,
        /// Unauthorized
        Unauthorized,
        /// Poll Does Not Exist,
        PollDoesNotExist,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // Voting for option
        #[transactional]
        #[pallet::weight(10_000)]
        pub fn vote(
            origin: OriginFor<T>,
            poll_id: String,
            voting_option: u32,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            ensure!(
                voting_option == 1 || voting_option == 2,
                Error::<T>::InvalidNumberOfOption,
            );

            let current_timestamp = Timestamp::<T>::get();
            let hermes_poll_info =
                <HermesPollData<T>>::get(&poll_id).ok_or(Error::<T>::PollDoesNotExist)?;

            ensure!(
                current_timestamp >= hermes_poll_info.poll_start_timestamp,
                Error::<T>::PollIsNotStarted
            );

            ensure!(
                current_timestamp <= hermes_poll_info.poll_end_timestamp,
                Error::<T>::PollIsFinished
            );

            let mut hermes_voting_info = <HermesVotings<T>>::get(&poll_id, &user);

            ensure!(
                hermes_voting_info.number_of_hermes >= MinimumHermesVotingAmount::<T>::get(),
                Error::<T>::InvalidAmountOfHermes
            );

            ensure!(
                hermes_voting_info.voting_option != 0,
                Error::<T>::InvalidVotingOption
            );

            hermes_voting_info.voting_option = voting_option;
            hermes_voting_info.number_of_hermes = MinimumHermesVotingAmount::<T>::get();
            hermes_voting_info.hermes_withdrawn = false;

            // Transfer Hermes to pallet
            Assets::<T>::transfer_from(
                &T::HermesAssetId::get().into(),
                &user,
                &Self::account_id(),
                hermes_voting_info.number_of_hermes,
            )
            .map_err(|_assets_err| Error::<T>::NotEnoughFunds)?;

            // Update storage
            <HermesVotings<T>>::insert(&poll_id, &user, hermes_voting_info);

            //Emit event
            Self::deposit_event(Event::<T>::Voted(user, poll_id, voting_option));

            // Return a successful DispatchResult
            Ok(().into())
        }

        //Create poll
        #[pallet::weight(10_000)]
        pub fn create_poll(
            origin: OriginFor<T>,
            poll_start_timestamp: T::Moment,
            poll_end_timestamp: T::Moment,
            title: String,
            description: String,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            let id = Uuid::new_v4();
            let poll_id = id.to_string();
            let current_timestamp = Timestamp::<T>::get();

            ensure!(
                poll_start_timestamp >= current_timestamp,
                Error::<T>::InvalidStartTimestamp
            );

            ensure!(
                poll_end_timestamp > poll_start_timestamp,
                Error::<T>::InvalidEndTimestamp
            );

            ensure!(
                MinimumHermesAmountForCreatingPoll::<T>::get()
                    <= Assets::<T>::free_balance(&T::HermesAssetId::get().into(), &user)
                        .unwrap_or(0),
                Error::<T>::NotEnoughHermesForCreatingPoll
            );

            let hermes_poll_info = HermesPollInfo {
                creator: user.clone(),
                hermes_locked: MinimumHermesAmountForCreatingPoll::<T>::get(),
                poll_start_timestamp,
                poll_end_timestamp,
                title,
                description,
                creator_hermes_withdrawn: false,
            };

            // Transfer Hermes to pallet
            Assets::<T>::transfer_from(
                &T::HermesAssetId::get().into(),
                &user.clone(),
                &Self::account_id(),
                hermes_poll_info.hermes_locked,
            )
            .map_err(|_assets_err| Error::<T>::NotEnoughFundsForCreatingPoll)?;

            <HermesPollData<T>>::insert(&poll_id, hermes_poll_info);

            //Emit event
            Self::deposit_event(Event::<T>::Created(
                user.clone(),
                poll_start_timestamp,
                poll_end_timestamp,
            ));

            // Return a successful DispatchResult
            Ok(().into())
        }

        // Withdraw funds voter
        #[pallet::weight(10_000)]
        pub fn withdraw_funds_voter(
            origin: OriginFor<T>,
            poll_id: String,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            let current_timestamp = Timestamp::<T>::get();
            let hermes_poll_info =
                <HermesPollData<T>>::get(&poll_id).ok_or(Error::<T>::PollDoesNotExist)?;

            ensure!(
                current_timestamp > hermes_poll_info.poll_end_timestamp,
                Error::<T>::PollIsNotFinished
            );

            let mut hermes_voting_info = <HermesVotings<T>>::get(&poll_id, &user);

            ensure!(
                hermes_voting_info.hermes_withdrawn == false,
                Error::<T>::FundsAlreadyWithdrawn
            );

            // Withdraw Hermes
            Assets::<T>::transfer_from(
                &T::HermesAssetId::get().into(),
                &Self::account_id(),
                &user,
                hermes_voting_info.number_of_hermes,
            )?;

            hermes_voting_info.hermes_withdrawn = true;
            <HermesVotings<T>>::insert(&poll_id, &user, &hermes_voting_info);

            //Emit event
            Self::deposit_event(Event::<T>::VoterFundsWithdrawn(
                user,
                hermes_voting_info.number_of_hermes,
            ));

            // Return a successful DispatchResult
            Ok(().into())
        }

        // Withdraw funds creator
        #[pallet::weight(10_000)]
        pub fn withdraw_funds_creator(
            origin: OriginFor<T>,
            poll_id: String,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            let current_timestamp = Timestamp::<T>::get();
            let mut hermes_poll_info =
                <HermesPollData<T>>::get(&poll_id).ok_or(Error::<T>::PollDoesNotExist)?;

            ensure!(
                hermes_poll_info.creator == user,
                Error::<T>::YouAreNotCreator
            );

            ensure!(
                current_timestamp >= hermes_poll_info.poll_end_timestamp,
                Error::<T>::PollIsNotEnded
            );

            ensure!(
                hermes_poll_info.creator_hermes_withdrawn == false,
                Error::<T>::CreatorFundsAlreadyWithdrawn
            );

            // Withdraw Creator Hermes
            Assets::<T>::transfer_from(
                &T::HermesAssetId::get().into(),
                &Self::account_id(),
                &user,
                hermes_poll_info.hermes_locked,
            )?;

            hermes_poll_info.creator_hermes_withdrawn = true;
            <HermesPollData<T>>::insert(&poll_id, &hermes_poll_info);

            //Emit event
            Self::deposit_event(Event::<T>::CreatorFundsWithdrawn(
                user,
                hermes_poll_info.hermes_locked,
            ));

            // Return a successful DispatchResult
            Ok(().into())
        }

        // Change minimum Hermes for voting
        #[pallet::weight(10_000)]
        pub fn change_min_hermes_for_voting(
            origin: OriginFor<T>,
            hermes_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            if user != AuthorityAccount::<T>::get() {
                return Err(Error::<T>::Unauthorized.into());
            }

            MinimumHermesVotingAmount::<T>::put(hermes_amount);

            //Emit event
            Self::deposit_event(Event::MinimumHermesForVotingChanged(hermes_amount));

            // Return a successful DispatchResult
            Ok(().into())
        }

        // Change minimum Hermes for creating a poll
        #[pallet::weight(10_000)]
        pub fn change_min_hermes_for_creating_poll(
            origin: OriginFor<T>,
            hermes_amount: Balance,
        ) -> DispatchResultWithPostInfo {
            let user = ensure_signed(origin)?;

            if user != AuthorityAccount::<T>::get() {
                return Err(Error::<T>::Unauthorized.into());
            }

            MinimumHermesAmountForCreatingPoll::<T>::put(hermes_amount);

            //Emit event
            Self::deposit_event(Event::MinimumHermesForCreatingPollChanged(hermes_amount));

            // Return a successful DispatchResult
            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    impl<T: Config> Pallet<T> {
        /// The account ID of pallet
        fn account_id() -> T::AccountId {
            PALLET_ID.into_account_truncating()
        }
    }
}
