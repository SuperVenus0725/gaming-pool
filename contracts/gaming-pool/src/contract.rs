use std::convert::TryFrom;
use std::ops::{Div, Mul};
use std::str::FromStr;

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, to_binary, Uint128};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use schemars::_serde_json::ser::State;

use cw20::Cw20QueryMsg;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute::{cancel_game, claim_refund, claim_reward, create_pool, execute_sweep,
                     game_pool_bid_submit, game_pool_reward_distribute, lock_game,
                     save_team_details, set_platform_fee_wallets,
                     set_pool_type_params, swap};
use crate::msg::{BalanceResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::{get_team_count_for_user_in_pool_type, query_all_pool_type_details, query_all_pools_in_game, query_all_teams, query_game_details, query_game_result, query_pool_collection, query_pool_details, query_pool_team_details, query_pool_type_details, query_refund, query_reward, query_swap_data_for_pool, query_team_details, query_total_fees};
use crate::state::{Config, CONFIG, GAME_DETAILS, GAME_RESULT_DUMMY, GameDetails, GameResult, SWAP_BALANCE_INFO};

// This is a comment
// version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:gaming-pool";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DUMMY_WALLET: &str = "terra1t3czdl5h4w4qwgkzs80fdstj0z7rfv9v2j6uh3";

// Initial reward amount to gamer for joining a pool
pub const INITIAL_REWARD_AMOUNT: u128 = 0u128;
// Initial refund amount to gamer for joining a pool
pub const INITIAL_REFUND_AMOUNT: u128 = 0u128;

// Initial value of team points
pub const INITIAL_TEAM_POINTS: u64 = 0u64;

// Initial rank of team - set to a low rank more than max pool size
pub const INITIAL_TEAM_RANK: u64 = 100000u64;

pub const UNCLAIMED_REWARD: bool = false;
pub const CLAIMED_REWARD: bool = true;
pub const UNCLAIMED_REFUND: bool = false;
pub const CLAIMED_REFUND: bool = true;
pub const REWARDS_DISTRIBUTED: bool = true;
pub const REWARDS_NOT_DISTRIBUTED: bool = false;

pub const GAME_POOL_OPEN: u64 = 1u64;
pub const GAME_POOL_CLOSED: u64 = 2u64;
pub const GAME_CANCELLED: u64 = 3u64;
pub const GAME_COMPLETED: u64 = 4u64;
pub const HUNDRED_PERCENT: u128 = 10000u128;
pub const NINETY_NINE_NINE_PERCENT: u128 = 9990u128;

pub const DUMMY_TEAM_ID: &str = "DUMMY_TEAM_ID";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        admin_address: deps.api.addr_validate(&msg.admin_address)?,
        minting_contract_address: deps.api.addr_validate(&msg.minting_contract_address)?,
        platform_fees_collector_wallet: deps
            .api
            .addr_validate(&msg.platform_fees_collector_wallet)?,
        astro_proxy_address: deps.api.addr_validate(&msg.astro_proxy_address)?,
        platform_fee: msg.platform_fee,
        transaction_fee: msg.transaction_fee,
        game_id: msg.game_id.clone(),
    };
    CONFIG.save(deps.storage, &config)?;

    let dummy_wallet = String::from(DUMMY_WALLET);
    let main_address = deps.api.addr_validate(dummy_wallet.clone().as_str())?;
    GAME_RESULT_DUMMY.save(
        deps.storage,
        &main_address,
        &GameResult {
            gamer_address: DUMMY_WALLET.to_string(),
            game_id: msg.game_id.clone(),
            team_id: DUMMY_TEAM_ID.to_string(),
            team_rank: INITIAL_TEAM_RANK,
            team_points: INITIAL_TEAM_POINTS,
            reward_amount: Uint128::from(INITIAL_REWARD_AMOUNT),
            refund_amount: Uint128::from(INITIAL_REFUND_AMOUNT),
        },
    )?;

    GAME_DETAILS.save(
        deps.storage,
        msg.game_id.clone(),
        &GameDetails {
            game_id: msg.game_id.clone(),
            game_status: GAME_POOL_OPEN,
        },
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Query Cw20 Check list
    check_and_confirm_whitelist_status(&deps, &info, &env)?;
    match msg {
        ExecuteMsg::SetPlatformFeeWallets { wallet_percentages } => {
            set_platform_fee_wallets(deps, info, wallet_percentages)
        }
        ExecuteMsg::SetPoolTypeParams {
            pool_type,
            pool_fee,
            min_teams_for_pool,
            max_teams_for_pool,
            max_teams_for_gamer,
            wallet_percentages,
        } => set_pool_type_params(
            deps,
            env,
            info,
            pool_type,
            pool_fee,
            min_teams_for_pool,
            max_teams_for_pool,
            max_teams_for_gamer,
            wallet_percentages,
        ),
        ExecuteMsg::CancelGame {} => cancel_game(deps, env, info),
        ExecuteMsg::LockGame {} => lock_game(deps, env, info),
        ExecuteMsg::CreatePool { pool_type } => create_pool(deps, env, info, pool_type),
        ExecuteMsg::ClaimReward { gamer } => claim_reward(deps, info, gamer, env),
        ExecuteMsg::ClaimRefund { gamer, max_spread } => claim_refund(deps, info, gamer, env, None, max_spread),
        ExecuteMsg::GamePoolRewardDistribute {
            pool_id,
            game_winners,
            is_final_batch,
            ust_for_rake,
        } => game_pool_reward_distribute(deps, env, info, pool_id, game_winners, is_final_batch, false, ust_for_rake),
        ExecuteMsg::GamePoolBidSubmitCommand {
            gamer,
            pool_type,
            pool_id,
            team_id,
            amount,
            max_spread
        } => game_pool_bid_submit(
            deps, env, info, gamer, pool_type, pool_id, team_id, amount, false, max_spread),
        ExecuteMsg::Sweep { funds } => execute_sweep(deps, info, funds),
        ExecuteMsg::Swap {
            amount,
            pool_id, max_spread
        } => swap(deps, env, info, amount, pool_id, max_spread),
    }
}

pub fn check_and_confirm_whitelist_status(
    deps: &DepsMut,
    info: &MessageInfo,
    env: &Env,
) -> Result<Response, ContractError> {
    let query = cw20_base::msg::QueryMsg::WhitelistRestriction {
        wallet_address: info.sender.to_string(),
        contract_address: env.contract.address.to_string(),
        contract_check_needed: true,
    };
    let state = CONFIG.load(deps.storage)?;
    let response_is_restricted: bool = deps.querier.query_wasm_smart(
        state.minting_contract_address,
        &query,
    )?;
    if response_is_restricted {
        return Err(ContractError::UserIsRestricted {})
    }
    return Ok(Response::default())
}


// This is the safe way of contract migration
// We can add expose specific state properties to
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PoolTeamDetails { pool_id, user } => {
            to_binary(&query_pool_team_details(deps.storage, pool_id, user)?)
        }
        QueryMsg::PoolDetails { pool_id } => to_binary(&query_pool_details(deps.storage, pool_id)?),
        QueryMsg::PoolTypeDetails { pool_type } => {
            to_binary(&query_pool_type_details(deps.storage, pool_type)?)
        }
        QueryMsg::AllPoolTypeDetails {} => to_binary(&query_all_pool_type_details(deps.storage)?),
        QueryMsg::AllTeams { users } => to_binary(&query_all_teams(deps.storage, users)?),
        QueryMsg::QueryReward { gamer } => to_binary(&query_reward(deps.storage, gamer)?),
        QueryMsg::QueryRefund { gamer } => to_binary(&query_refund(deps.storage, gamer)?),
        QueryMsg::QueryGameResult {
            gamer,
            pool_id,
            team_id,
        } => to_binary(&query_game_result(deps, gamer, pool_id, team_id)?),
        QueryMsg::GameDetails {} => to_binary(&query_game_details(deps.storage)?),
        QueryMsg::PoolTeamDetailsWithTeamId { pool_id, team_id, gamer } => {
            to_binary(&query_team_details(deps.storage, pool_id, team_id, gamer)?)
        }
        QueryMsg::AllPoolsInGame {} => to_binary(&query_all_pools_in_game(deps.storage)?),
        QueryMsg::PoolCollection { pool_id } => {
            to_binary(&query_pool_collection(deps.storage, pool_id)?)
        }
        QueryMsg::GetTeamCountForUserInPoolType {
            game_id,
            gamer,
            pool_type,
        } => to_binary(&get_team_count_for_user_in_pool_type(
            deps.storage,
            gamer,
            game_id,
            pool_type,
        )?),
        QueryMsg::SwapInfo {
            pool_id
        } => to_binary(&query_swap_data_for_pool(
            deps.storage,
            pool_id,
        )?),
        QueryMsg::GetTotalFees {
            amount
        } => to_binary(&query_total_fees(
            deps,
            amount,
        )?)
    }
}


#[allow(dead_code)]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let pool_id = msg.id.to_string();
    let current_fury_balance: BalanceResponse = deps.querier.query_wasm_smart(
        config.clone().minting_contract_address,
        &Cw20QueryMsg::Balance {
            address: _env.contract.address.clone().to_string()
        },
    )?;
    let mut balance_info = SWAP_BALANCE_INFO.load(deps.storage, pool_id.clone())?;
    balance_info.balance_post_swap = current_fury_balance.balance;
    let balance_gained = balance_info.balance_post_swap - balance_info.balance_pre_swap;
    // ((Balance gained * 10_000) / Amount In UST Swapped)
    // (poolcollection * exchange rate)/10_000 at time of use
    balance_info.exchange_rate = balance_gained.checked_mul(Uint128::from(10000u128)).unwrap().checked_div(balance_info.ust_amount_swapped).unwrap();
    SWAP_BALANCE_INFO.save(deps.storage, pool_id.clone(), &balance_info)?;
    return Ok(Response::default()
        .add_attribute("fury_balance_gained", balance_gained.to_string())
        .add_attribute("exchange_rate_recieved", balance_info.exchange_rate.to_string())
        .add_attribute("pool_id", pool_id)
    );
}
