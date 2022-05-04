use cosmwasm_std::{Binary, Uint128};
use cosmwasm_std::{Coin, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::Cw20ReceiveMsg;

use crate::state::ClubStakingDetails;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    /// Administrator privilege wallet address
    pub admin_address: String,
    /// Fury Token Miniting Contract address
    pub minting_contract_address: String,
    /// Proxy Contract for Astroport Liquidity Pool
    pub astro_proxy_address: String,
    /// Wallet where the Club Fee from Buy a Club is transferred
    pub club_fee_collector_wallet: String,
    /// Next timestamp when the next reward distribution shall take place
    pub club_reward_next_timestamp: Timestamp,
    /// Periodicity for reward distribution (in seconds)
    pub reward_periodicity: u64,
    /// Price in Fury for Buying a Club
    pub club_price: Uint128,
    /// Bonding duration (in seconds) applicable when Un-staking is initiated
    pub bonding_duration: u64,
    /// Duration (in seconds) for which a club is released by an Owner for potential Buy a Club by a new owner
    pub owner_release_locking_duration: u64,
    /// Wallet where Platform Fees (other than the fee used towards Swap) would transfered
    pub platform_fees_collector_wallet: String,
    /// Plastform Fee Specified in percentage multiplied by 100, i.e. 100% = 10000 and 0.01% = 1
    pub platform_fees: Uint128,
    /// Transaction Fee Specified in percentage multiplied by 100, i.e. 100% = 10000 and 0.01% = 1
    pub transaction_fees: Uint128,
    /// Control Fee Specified in percentage multiplied by 100, i.e. 100% = 10000 and 0.01% = 1
    pub control_fees: Uint128,
    pub max_bonding_limit_per_user: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// to Buy a Club , when some club is available for purchase by generic public
    BuyAClub {
        buyer: String,
        seller: Option<String>,
        club_name: String,
        auto_stake: bool,
    },
    /// Administrator Assigns Club Ownership
    AssignAClub {
        buyer: String,
        seller: Option<String>,
        club_name: String,
        auto_stake: bool,
    },
    /// to Stake Tokens on a Club by generic public
    StakeOnAClub {
        staker: String,
        club_name: String,
        amount: Uint128,
        auto_stake: bool,
    },
    /// to Stake Tokens on a Club on behalf of a Staker by Administrator
    AssignStakesToAClub {
        stake_list: Vec<ClubStakingDetails>,
        club_name: String,
    },
    /// to Release Ownership of a Club, for a potential new purchaser
    ReleaseClub {
        owner: String,
        club_name: String,
    },
    /// to Claim Rewards accumulated for a Club Owner
    ClaimOwnerRewards {
        owner: String,
        club_name: String,
    },
    /// to Claim Rewards accumulated for a wallet which was previously a Club Owner
    ClaimPreviousOwnerRewards {
        previous_owner: String,
    },
    /// to Un-stake Tokens , in two steps - 1) to a Bonded Stake and then 2) to Claim it after maturity
    StakeWithdrawFromAClub {
        staker: String,
        club_name: String,
        amount: Uint128,
        immediate_withdrawal: bool,
    },
    /// To Distribute Rewards to Stakers and Owners based on Club Ranking by Administrator in Batches
    CalculateAndDistributeRewards {
        staker_list: Vec<String>,
        club_name: String,
        is_first_batch: bool,
        is_final_batch: bool,
    },
    /// to Claim Rewards accumulated for a wallet of a Staker
    ClaimStakerRewards {
        staker: String,
        club_name: String,
    },
    IncreaseRewardAmount {
        reward_from: String,
        amount: Uint128,
    },

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ClubStakingDetails {
        club_name: String,
        user_list: Vec<String>,
    },
    /// Returns the current state of withdrawn tokens that are locked for
    /// BONDING_DURATION = 7 days (before being credited back) for the given address.
    /// Return type: BondingDetails.
    ClubOwnershipDetails {
        club_name: String,
    },
    ClubPreviousOwnershipDetails {
        previous_owner: String,
    },
    ClubOwnershipDetailsForOwner {
        owner_address: String,
    },
    AllClubOwnershipDetails {},
    AllPreviousClubOwnershipDetails {},
    AllStakes {
        user_address_list: Vec<String>,
    },
    AllStakesForUser {
        user_address: String,
    },
    AllBonds {
        user_address_list: Vec<String>,
    },
    ClubBondingDetailsForUser {
        club_name: String,
        user_address: String,
    },
    RewardAmount {},
    QueryPlatformFees {
        msg: Binary,
    },
    QueryStakerRewards {
        staker: String,
        club_name: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceivedMsg {
    /// Incoming Rewards for meant for distribution to Stakers and Owners
    IncreaseRewardAmount(IncreaseRewardAmountCommand),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IncreaseRewardAmountCommand {
    pub reward_from: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ProxyQueryMsgs {
    get_fury_equivalent_to_ust {
        ust_count: Uint128,
    },
    get_ust_equivalent_to_fury {
        fury_count: Uint128,
    },
}

