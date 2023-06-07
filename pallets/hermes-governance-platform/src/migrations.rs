use crate::alloc::string::ToString;
use crate::{AccountIdOf, Config, HermesPollData, HermesPollInfo, HermesVotingInfo, HermesVotings};
use alloc::string::String;
use codec::{Decode, Encode};
use common::Balance;
use frame_support::dispatch::Weight;
use frame_support::log;
use frame_support::traits::Get;
use frame_support::RuntimeDebug;
use sp_std::vec::Vec;

#[derive(Encode, Decode, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo, Clone, Copy)]
pub enum VotingOption {
    Yes,
    No,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, scale_info::TypeInfo)]
pub struct OldHermesPollInfo<AccountId, Moment> {
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

pub fn migrate<T: Config>() -> Weight {
    sp_runtime::runtime_logger::RuntimeLogger::init();
    migrate_voting_and_poll_data::<T>()
}

pub fn migrate_voting_and_poll_data<T: Config>() -> Weight {
    let mut weight: u64 = 0;

    HermesVotings::<T>::translate_values::<(VotingOption, Balance, bool), _>(
        |(voting_option, number_of_hermes, hermes_withdrawn)| {
            weight += 1;

            let new_voting_option: String;

            if voting_option == VotingOption::Yes {
                new_voting_option = "Yes".to_string();
            } else {
                new_voting_option = "No".to_string();
            }

            Some(HermesVotingInfo {
                voting_option: new_voting_option,
                number_of_hermes,
                hermes_withdrawn,
            })
        },
    );

    HermesPollData::<T>::translate_values::<OldHermesPollInfo<AccountIdOf<T>, T::Moment>, _>(|v| {
        weight += 1;

        let mut options = Vec::new();
        options.push("Yes".to_string());
        options.push("No".to_string());

        Some(HermesPollInfo {
            creator: v.creator,
            hermes_locked: v.hermes_locked,
            poll_start_timestamp: v.poll_start_timestamp,
            poll_end_timestamp: v.poll_end_timestamp,
            title: v.title,
            description: v.description,
            creator_hermes_withdrawn: v.creator_hermes_withdrawn,
            options,
        })
    });

    log::info!(
        target: "runtime",
        "HermesVotingInfo migrated to new version with voting_option as a 'String' and HermesPollInfo migrated to new version with 'options' field"
    );

    T::DbWeight::get().reads_writes(weight, weight)
}
