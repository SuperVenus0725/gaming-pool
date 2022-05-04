use astroport::asset::Asset;
use astroport::factory::PairType;
use cosmwasm_std::{Addr, Binary, Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::{Cw20ReceiveMsg, Logo};

use crate::ContractError;
use crate::state::{GameResult, SwapBalanceDetails, WalletPercentage};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub admin_address: String,
    pub minting_contract_address: String,
    pub platform_fees_collector_wallet: String,
    pub transaction_fee: Uint128,
    pub astro_proxy_address: String,
    pub platform_fee: Uint128,
    pub game_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetPlatformFeeWallets {
        wallet_percentages: Vec<WalletPercentage>
    },
    SetPoolTypeParams {
        pool_type: String,
        pool_fee: Uint128,
        min_teams_for_pool: u32,
        max_teams_for_pool: u32,
        max_teams_for_gamer: u32,
        wallet_percentages: Vec<WalletPercentage>,
    },
    CancelGame {},
    LockGame {},
    CreatePool {
        pool_type: String
    },
    ClaimReward {
        gamer: String
    },
    ClaimRefund {
        gamer: String,
        max_spread: Option<Decimal>,
    },
    GamePoolRewardDistribute {
        pool_id: String,
        game_winners: Vec<GameResult>,
        is_final_batch: bool,
        ust_for_rake: Uint128,
    },
    GamePoolBidSubmitCommand {
        gamer: String,
        pool_type: String,
        pool_id: String,
        team_id: String,
        amount: Uint128,
        max_spread: Option<Decimal>,

    },
    Sweep { funds: Vec<Coin> },
    Swap {
        amount: Uint128,
        pool_id: String,
        max_spread: Option<Decimal>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    PoolTeamDetails {
        pool_id: String,
        user: String,
    },
    PoolDetails {
        pool_id: String,
    },
    PoolTypeDetails {
        pool_type: String,
    },
    AllPoolTypeDetails {},
    AllTeams { users: Vec<String> },
    QueryReward {
        gamer: String
    },
    QueryRefund {
        gamer: String,
    },
    QueryGameResult {
        gamer: String,
        pool_id: String,
        team_id: String,
    },
    GameDetails {},
    PoolTeamDetailsWithTeamId {
        pool_id: String,
        team_id: String,
        gamer: String,
    },
    AllPoolsInGame {},
    PoolCollection {
        pool_id: String,
    },
    GetTeamCountForUserInPoolType {
        gamer: String,
        game_id: String,
        pool_type: String,
    },
    SwapInfo {
        pool_id: String
    },
    GetTotalFees {
        amount: Uint128
    },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceivedMsg {
    GamePoolBidSubmit(GamePoolBidSubmitCommand),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GamePoolBidSubmitCommand {
    pub gamer: String,
    pub pool_type: String,
    pub pool_id: String,
    pub team_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ProxyQueryMsgs {
    get_fury_equivalent_to_ust {
        ust_count: Uint128,
    },
    get_ust_equivalent_to_fury {
        fury_count: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyExecuteMsg {
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    /// Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsgSimulation {
    /// Returns information about a swap simulation in a [`SimulationResponse`] object.
    Simulation { offer_asset: Asset },
    ReverseSimulation { ask_asset: Asset },
    FeeInfo {
        pair_type: PairType,
    },
    QueryPlatformFees {
        msg: Binary,
    },
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}
