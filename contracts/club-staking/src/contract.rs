use cosmwasm_std::{
    BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, from_binary, MessageInfo, Order,
    Reply, Response, StdError, StdResult, Storage, SubMsg, to_binary, Uint128, WasmMsg,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Timestamp};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw2::set_contract_version;
use cw_storage_plus::Map;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, ProxyQueryMsgs, QueryMsg, ReceivedMsg};
use crate::state::{
    CLUB_BONDING_DETAILS, CLUB_OWNERSHIP_DETAILS, CLUB_PREVIOUS_OWNER_DETAILS, CLUB_REWARD_NEXT_TIMESTAMP, CLUB_STAKING_DETAILS,
    CLUB_STAKING_SNAPSHOT, ClubBondingDetails, ClubOwnershipDetails,
    ClubPreviousOwnerDetails, ClubStakingDetails, Config, CONFIG, REWARD, REWARD_GIVEN_IN_CURRENT_TIMESTAMP,
    WINNING_CLUB_DETAILS_SNAPSHOT, WinningClubDetails,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:club-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INCREASE_STAKE: bool = true;
const DECREASE_STAKE: bool = false;
const IMMEDIATE_WITHDRAWAL: bool = true;
const NO_IMMEDIATE_WITHDRAWAL: bool = false;
const DONT_CHANGE_AUTO_STAKE_SETTING: bool = false;
const SET_AUTO_STAKE: bool = true;
const MAX_UFURY_COUNT: i128 = 420000000000000;
// Reward to club owner for buying - 0 tokens
const CLUB_BUYING_REWARD_AMOUNT: u128 = 0u128;

// Reward to club staker for staking - 0 tokens
const CLUB_STAKING_REWARD_AMOUNT: u128 = 0u128;

// This is reduced to 0 day locking period in seconds, after buying a club, as no refund planned for Ownership Fee
const CLUB_LOCKING_DURATION: u64 = 0u64;

// This is locking period in seconds, after staking in club.
// No longer applicable so setting it to 0
const CLUB_STAKING_DURATION: u64 = 0u64;

// this is 7 day bonding period in seconds, after withdrawing a stake 
// TODO _ Revert after DEBUG : this is 1 hour for testing purposes only
// const CLUB_BONDING_DURATION: u64 = 3600u64;
// - now part of instantiation msg.bonding_duration

const HUNDRED_PERCENT: u128 = 10000u128;
const NINETY_NINE_NINE_PERCENT: u128 = 9990u128;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let mut next_reward_time = msg.club_reward_next_timestamp;
    if next_reward_time.seconds() == 0u64 {
        next_reward_time = _env.block.time.minus_seconds(1);
    }
    let config = Config {
        admin_address: deps.api.addr_validate(&msg.admin_address)?,
        minting_contract_address: deps.api.addr_validate(&msg.minting_contract_address)?,
        astro_proxy_address: deps.api.addr_validate(&msg.astro_proxy_address)?,
        club_fee_collector_wallet: deps.api.addr_validate(&msg.club_fee_collector_wallet)?,
        club_reward_next_timestamp: next_reward_time,
        reward_periodicity: msg.reward_periodicity,
        club_price: msg.club_price,
        bonding_duration: msg.bonding_duration,
        owner_release_locking_duration: msg.owner_release_locking_duration,
        platform_fees_collector_wallet: deps
            .api
            .addr_validate(&msg.platform_fees_collector_wallet)?,
        platform_fees: msg.platform_fees,
        transaction_fees: msg.transaction_fees,
        control_fees: msg.control_fees,
        max_bonding_limit_per_user: msg.max_bonding_limit_per_user,
    };
    CONFIG.save(deps.storage, &config)?;

    CLUB_REWARD_NEXT_TIMESTAMP.save(deps.storage, &config.club_reward_next_timestamp)?;
    println!(
        "now = {:?} next_timestamp = {:?} periodicity = {:?}",
        _env.block.time, config.club_reward_next_timestamp, config.reward_periodicity
    );
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::StakeOnAClub {
            staker,
            club_name,
            amount,
            auto_stake,
        } => {
            stake_on_a_club(deps, env, info, staker, club_name, amount, auto_stake)
        }
        ExecuteMsg::AssignStakesToAClub {
            stake_list,
            club_name
        } => {
            assign_stakes_to_a_club(deps, env, info, stake_list, club_name)
        }
        ExecuteMsg::BuyAClub {
            buyer,
            seller,
            club_name,
            auto_stake,
        } => {
            let config = CONFIG.load(deps.storage)?;
            let price = config.club_price;
            buy_a_club(deps, env, info, buyer, seller, club_name, price, auto_stake)
        }
        ExecuteMsg::AssignAClub {
            buyer,
            seller,
            club_name,
            auto_stake,
        } => {
            assign_a_club(deps, env, info, buyer, seller, club_name, auto_stake)
        }
        ExecuteMsg::ReleaseClub { owner, club_name } => {
            release_club(deps, env, info, owner, club_name)
        }
        ExecuteMsg::ClaimOwnerRewards { owner, club_name } => {
            claim_owner_rewards(deps, env, info, owner, club_name)
        }
        ExecuteMsg::ClaimPreviousOwnerRewards { previous_owner } => {
            claim_previous_owner_rewards(deps, info, previous_owner)
        }
        ExecuteMsg::StakeWithdrawFromAClub {
            staker,
            club_name,
            amount,
            immediate_withdrawal,
        } => withdraw_stake_from_a_club(
            deps,
            env,
            info,
            staker,
            club_name,
            amount,
            immediate_withdrawal,
        ),
        ExecuteMsg::CalculateAndDistributeRewards {
            staker_list,
            club_name,
            is_first_batch,
            is_final_batch,
        } => {
            calculate_and_distribute_rewards(deps, env, info, staker_list, club_name, is_first_batch, is_final_batch)
        }
        ExecuteMsg::ClaimStakerRewards { staker, club_name } => {
            claim_staker_rewards(deps, info, staker, club_name)
        }
        ExecuteMsg::IncreaseRewardAmount {
            reward_from,
            amount,
        } => {
            increase_reward_amount(deps, env, info, reward_from, amount)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}


fn received_message(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    message: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceivedMsg = from_binary(&message.msg)?;
    let amount = Uint128::from(message.amount);
    match msg {
        ReceivedMsg::IncreaseRewardAmount(irac) => {
            increase_reward_amount(deps, env, info, irac.reward_from, amount)
        }
    }
    // Err(ContractError::Std(StdError::GenericErr {
    //     msg: format!("received_message where msg = {:?}", msg),
    // }))
}

fn claim_previous_owner_rewards(
    deps: DepsMut,
    info: MessageInfo,
    previous_owner: String,
) -> Result<Response, ContractError> {
    let mut amount = Uint128::zero();
    let mut transfer_confirmed = false;
    let previous_owner_addr = deps.api.addr_validate(&previous_owner)?;
    //Check if withdrawer is same as invoker
    if previous_owner_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let previous_ownership_details;
    let previous_ownership_details_result =
        CLUB_PREVIOUS_OWNER_DETAILS.may_load(deps.storage, previous_owner.clone());
    match previous_ownership_details_result {
        Ok(od) => {
            previous_ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    if !(previous_ownership_details.is_none()) {
        for previous_owner_detail in previous_ownership_details {
            if previous_owner_detail.previous_owner_address == previous_owner.clone() {
                if Uint128::zero() == previous_owner_detail.reward_amount {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: String::from("No rewards for this previous owner"),
                    }));
                }

                amount = previous_owner_detail.reward_amount;

                // Now remove the previous ownership details
                CLUB_PREVIOUS_OWNER_DETAILS.remove(deps.storage, previous_owner.clone());

                // Add amount to the owners wallet
                transfer_confirmed = true;
            }
        }
    }
    if transfer_confirmed == false {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Not a valid previous owner for the club"),
        }));
    }
    transfer_from_contract_to_wallet(
        deps.storage,
        previous_owner.clone(),
        amount,
        "previous_owners_reward".to_string(),
    )
}

fn claim_owner_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    club_name: String,
) -> Result<Response, ContractError> {
    let mut amount = Uint128::zero();
    let mut transfer_confirmed = false;
    let owner_addr = deps.api.addr_validate(&owner)?;
    //Check if withdrawer is same as invoker
    if owner_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    if !(ownership_details.is_none()) {
        for owner_detail in ownership_details {
            if owner_detail.owner_address == owner.clone() {
                if Uint128::zero() == owner_detail.reward_amount {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: String::from("No rewards for this owner"),
                    }));
                }

                transfer_confirmed = true;

                amount = owner_detail.reward_amount;

                // Now save the ownership details
                CLUB_OWNERSHIP_DETAILS.save(
                    deps.storage,
                    club_name.clone(),
                    &ClubOwnershipDetails {
                        club_name: owner_detail.club_name,
                        start_timestamp: owner_detail.start_timestamp,
                        locking_period: owner_detail.locking_period,
                        owner_address: owner_detail.owner_address,
                        price_paid: owner_detail.price_paid,
                        reward_amount: Uint128::zero(),
                        owner_released: owner_detail.owner_released,
                        total_staked_amount: owner_detail.total_staked_amount,
                    },
                )?;
            }
        }
    }

    if transfer_confirmed == false {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Not a valid owner for the club"),
        }));
    }
    transfer_from_contract_to_wallet(
        deps.storage,
        owner.clone(),
        amount,
        "owner_reward".to_string(),
    )
}

fn periodically_refund_stakeouts(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    /*
        this is no longer used, 6 Apr 2022
    */
    return Ok(Response::default());
}

fn buy_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    buyer: String,
    seller_opt: Option<String>,
    club_name: String,
    price: Uint128,
    auto_stake: bool,
) -> Result<Response, ContractError> {
    if info.sender != buyer {
        return Err(ContractError::Unauthorized {});
    }

    println!("seller_opt = {:?}", seller_opt);
    let seller;
    match seller_opt.clone() {
        Some(s) => seller = s,
        None => seller = String::default(),
    }

    let config = CONFIG.load(deps.storage)?;

    let club_price = config.club_price;
    if price != club_price {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Club price is not matching"),
        }));
    }

    let required_ust_fees: Uint128;
    //To bypass calls from unit tests
    if info.sender.clone().into_string() == String::from("Owner001")
        || info.sender.clone().into_string() == String::from("Owner002")
        || info.sender.clone().into_string() == String::from("Owner003")
    {
        required_ust_fees = Uint128::zero();
    } else {
        required_ust_fees = query_platform_fees(
            deps.as_ref(),
            to_binary(&ExecuteMsg::BuyAClub {
                buyer: buyer.clone(),
                club_name: club_name.clone(),
                seller: seller_opt,
                auto_stake: auto_stake,
            })?,
        )?;
    }
    let mut fees = Uint128::zero();
    for fund in info.funds.clone() {
        if fund.denom == "uusd" {
            fees = fees.checked_add(fund.amount).unwrap();
        }
    }
    let adjusted_ust_fees = required_ust_fees
        * (Uint128::from(NINETY_NINE_NINE_PERCENT))
        / (Uint128::from(HUNDRED_PERCENT));
    if fees < adjusted_ust_fees {
        return Err(ContractError::InsufficientFees {
            required: required_ust_fees,
            received: fees,
        });
    }
    let buyer_addr = deps.api.addr_validate(&buyer)?;

    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();

    for one_club_name in all_clubs {
        let one_ownership_details =
            CLUB_OWNERSHIP_DETAILS.load(deps.storage, one_club_name.clone())?;
        if buyer == one_ownership_details.owner_address {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("buyer already owns this club"),
            }));
        }
    }

    let mut previous_owners_reward_amount = Uint128::from(0u128);
    let mut total_staked_amount = Uint128::from(0u128);

    if !(ownership_details.is_none()) {
        for owner in ownership_details {
            let mut current_time = env.block.time;
            let mut release_start_time = owner.start_timestamp;
            let mut release_locking_duration = owner.locking_period;
            println!(
                "release_start_time = {:?} locking_duration = {:?} current time = {:?}",
                release_start_time, release_locking_duration, current_time
            );
            if owner.owner_released == false {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Owner has not released the club"),
                }));
            } else if current_time > release_start_time.plus_seconds(release_locking_duration) {
                println!("Release time for the club has expired");
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Release time for the club has expired"),
                }));
            } else if owner.owner_address != String::default() && owner.owner_address != seller {
                println!(
                    "owner.owner_address = {:?} and seller = {:?}",
                    owner.owner_address, seller
                );
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Seller is not the owner for the club"),
                }));
            }

            total_staked_amount = owner.total_staked_amount;

            // Evaluate previous owner rewards
            previous_owners_reward_amount = owner.reward_amount;
            println!("prv own amount picked {:?}", previous_owners_reward_amount);
            let mut previous_reward = Uint128::zero();
            println!("prv own amount avl {:?}", previous_owners_reward_amount);
            if previous_owners_reward_amount != Uint128::zero() {
                let pod = CLUB_PREVIOUS_OWNER_DETAILS.may_load(deps.storage, seller.clone())?;
                match pod {
                    Some(pod) => {
                        previous_reward = pod.reward_amount;
                        println!("prv own existing reward {:?}", previous_reward);
                    }
                    None => {}
                }

                // Now save the previous ownership details
                CLUB_PREVIOUS_OWNER_DETAILS.save(
                    deps.storage,
                    seller.clone(),
                    &ClubPreviousOwnerDetails {
                        previous_owner_address: seller.clone(),
                        reward_amount: previous_reward + previous_owners_reward_amount,
                    },
                )?;
            }
        }
    }

    // Now save the ownership details
    CLUB_OWNERSHIP_DETAILS.save(
        deps.storage,
        club_name.clone(),
        &ClubOwnershipDetails {
            club_name: club_name.clone(),
            start_timestamp: env.block.time,
            locking_period: config.owner_release_locking_duration,
            owner_address: buyer_addr.to_string(),
            price_paid: price,
            reward_amount: Uint128::from(CLUB_BUYING_REWARD_AMOUNT),
            owner_released: false,
            total_staked_amount: total_staked_amount,
        },
    )?;

    let mut stakes = Vec::new();
    let mut user_stake_exists = false;
    let all_stakes = CLUB_STAKING_DETAILS.may_load(deps.storage, (&club_name.clone(), &buyer.clone()))?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }
    for stake in stakes {
        if buyer == stake.staker_address {
            user_stake_exists = true;
        }
    }
    if !user_stake_exists {
        // Now save the staking details for the owner - with 0 stake
        save_staking_details(
            deps.storage,
            env,
            buyer.clone(),
            club_name.clone(),
            Uint128::zero(),
            auto_stake,
            INCREASE_STAKE,
        )?;
    }

    let transfer_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.into_string(),
        recipient: config.club_fee_collector_wallet.to_string(),
        amount: price,
    };
    let exec = WasmMsg::Execute {
        contract_addr: config.minting_contract_address.to_string(),
        msg: to_binary(&transfer_msg).unwrap(),
        funds: vec![],
    };

    let send_wasm: CosmosMsg = CosmosMsg::Wasm(exec);
    let send_bank: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.platform_fees_collector_wallet.into_string(),
        amount: info.funds,
    });
    let data_msg = format!("Club fees {} received", price).into_bytes();
    return Ok(Response::new()
        .add_message(send_wasm)
        .add_message(send_bank)
        .add_attribute("action", "buy_a_club")
        .add_attribute("buyer", buyer)
        .add_attribute("club_name", club_name)
        .add_attribute("fees", price.to_string())
        .set_data(data_msg));
}

fn assign_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    buyer: String,
    seller_opt: Option<String>,
    club_name: String,
    auto_stake: bool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    println!("seller_opt = {:?}", seller_opt);
    let seller;
    match seller_opt.clone() {
        Some(s) => seller = s,
        None => seller = String::default(),
    }

    let buyer_addr = deps.api.addr_validate(&buyer)?;

    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();

    for one_club_name in all_clubs {
        let one_ownership_details =
            CLUB_OWNERSHIP_DETAILS.load(deps.storage, one_club_name.clone())?;
        if buyer == one_ownership_details.owner_address {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("buyer already owns this club"),
            }));
        }
    }

    let mut previous_owners_reward_amount = Uint128::from(0u128);
    let mut total_staked_amount = Uint128::from(0u128);

    if !(ownership_details.is_none()) {
        for owner in ownership_details {
            let mut current_time = env.block.time;
            let mut release_start_time = owner.start_timestamp;
            let mut release_locking_duration = owner.locking_period;
            println!(
                "release_start_time = {:?} locking_duration = {:?} current time = {:?}",
                release_start_time, release_locking_duration, current_time
            );
            if owner.owner_released == false {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Owner has not released the club"),
                }));
            } else if current_time > release_start_time.plus_seconds(release_locking_duration) {
                println!("Release time for the club has expired");
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Release time for the club has expired"),
                }));
            } else if owner.owner_address != String::default() && owner.owner_address != seller {
                println!(
                    "owner.owner_address = {:?} and seller = {:?}",
                    owner.owner_address, seller
                );
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Seller is not the owner for the club"),
                }));
            }

            total_staked_amount = owner.total_staked_amount;

            // Evaluate previous owner rewards
            previous_owners_reward_amount = owner.reward_amount;
            println!("prv own amount picked {:?}", previous_owners_reward_amount);
            let mut previous_reward = Uint128::zero();
            println!("prv own amount avl {:?}", previous_owners_reward_amount);
            if previous_owners_reward_amount != Uint128::zero() {
                let pod = CLUB_PREVIOUS_OWNER_DETAILS.may_load(deps.storage, seller.clone())?;
                match pod {
                    Some(pod) => {
                        previous_reward = pod.reward_amount;
                        println!("prv own existing reward {:?}", previous_reward);
                    }
                    None => {}
                }

                // Now save the previous ownership details
                CLUB_PREVIOUS_OWNER_DETAILS.save(
                    deps.storage,
                    seller.clone(),
                    &ClubPreviousOwnerDetails {
                        previous_owner_address: seller.clone(),
                        reward_amount: previous_reward + previous_owners_reward_amount,
                    },
                )?;
            }
        }
    }

    // Now save the ownership details
    CLUB_OWNERSHIP_DETAILS.save(
        deps.storage,
        club_name.clone(),
        &ClubOwnershipDetails {
            club_name: club_name.clone(),
            start_timestamp: env.block.time,
            locking_period: config.owner_release_locking_duration,
            owner_address: buyer_addr.to_string(),
            price_paid: Uint128::zero(),
            reward_amount: Uint128::from(CLUB_BUYING_REWARD_AMOUNT),
            owner_released: false,
            total_staked_amount: total_staked_amount,
        },
    )?;

    let mut stakes = Vec::new();
    let mut user_stake_exists = false;
    let all_stakes = CLUB_STAKING_DETAILS.may_load(deps.storage, (&club_name.clone(), &buyer.clone()))?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }
    for stake in stakes {
        if buyer == stake.staker_address {
            user_stake_exists = true;
        }
    }
    if !user_stake_exists {
        // Now save the staking details for the owner - with 0 stake
        save_staking_details(
            deps.storage,
            env,
            buyer.clone(),
            club_name.clone(),
            Uint128::zero(),
            auto_stake,
            INCREASE_STAKE,
        )?;
    }

    return Ok(Response::default());
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    return Err(ContractError::Std(StdError::GenericErr {
        msg: format!("the reply details are {:?}", reply),
    }));
}

fn release_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    seller: String,
    club_name: String,
) -> Result<Response, ContractError> {
    let seller_addr = deps.api.addr_validate(&seller)?;
    //Check if seller is same as invoker
    if seller_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    // check that the current ownership is with the seller
    if ownership_details.is_none() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Releaser is not the owner for the club"),
        }));
    }
    for owner in ownership_details {
        if owner.owner_address != seller_addr {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("Releaser is not the owner for the club"),
            }));
        } else {
            // Update the ownership details
            CLUB_OWNERSHIP_DETAILS.save(
                deps.storage,
                club_name.clone(),
                &ClubOwnershipDetails {
                    club_name: owner.club_name,
                    start_timestamp: env.block.time,
                    locking_period: owner.locking_period,
                    owner_address: owner.owner_address,
                    price_paid: owner.price_paid,
                    reward_amount: owner.reward_amount,
                    owner_released: true,
                    total_staked_amount: owner.total_staked_amount,
                },
            )?;
        }
    }
    return Ok(Response::default());
}

fn stake_on_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    club_name: String,
    amount: Uint128,
    auto_stake: bool,
) -> Result<Response, ContractError> {
    if info.sender != staker {
        return Err(ContractError::Unauthorized {});
    }

    let config = CONFIG.load(deps.storage)?;

    let staker_addr = deps.api.addr_validate(&staker)?;
    let contract_address = env.clone().contract.address.into_string();

    let required_ust_fees: Uint128;
    //To bypass calls from unit tests
    if info.sender.clone().into_string() == String::from("Staker001")
        || info.sender.clone().into_string() == String::from("Staker002")
        || info.sender.clone().into_string() == String::from("Staker003")
        || info.sender.clone().into_string() == String::from("Staker004")
        || info.sender.clone().into_string() == String::from("Staker005")
        || info.sender.clone().into_string() == String::from("Staker006")
    {
        required_ust_fees = Uint128::zero();
    } else {
        required_ust_fees = query_platform_fees(
            deps.as_ref(),
            to_binary(&ExecuteMsg::StakeOnAClub {
                staker: staker.clone(),
                club_name: club_name.clone(),
                amount: amount,
                auto_stake: auto_stake,
            })?,
        )?;
    }
    let mut fees = Uint128::zero();
    for fund in info.funds.clone() {
        if fund.denom == "uusd" {
            fees = fees.checked_add(fund.amount).unwrap();
        }
    }
    let adjusted_ust_fees = required_ust_fees
        * (Uint128::from(NINETY_NINE_NINE_PERCENT))
        / (Uint128::from(HUNDRED_PERCENT));
    if fees < adjusted_ust_fees {
        return Err(ContractError::InsufficientFees {
            required: required_ust_fees,
            received: fees,
        });
    }

    //check if the club_name is available for staking
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("Cannot find the club"),
            }));
        }
    }
    if ownership_details.is_some() {
        // Now save the staking details
        save_staking_details(
            deps.storage,
            env,
            staker.clone(),
            club_name.clone(),
            amount,
            auto_stake,
            INCREASE_STAKE,
        )?;
    } else {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("The club is not available for staking"),
        }));
    }

    let transfer_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.into_string(),
        recipient: contract_address,
        amount: amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: config.minting_contract_address.to_string(),
        msg: to_binary(&transfer_msg).unwrap(),
        funds: vec![],
    };

    let send_wasm: CosmosMsg = CosmosMsg::Wasm(exec);
    let send_bank: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.platform_fees_collector_wallet.into_string(),
        amount: info.funds,
    });
    let data_msg = format!("Club stake {} received", amount).into_bytes();
    return Ok(Response::new()
        .add_message(send_wasm)
        .add_message(send_bank)
        .add_attribute("action", "stake_on_a_club")
        .add_attribute("staker", staker)
        .add_attribute("club_name", club_name)
        .add_attribute("stake", amount.to_string())
        .set_data(data_msg));
}

fn assign_stakes_to_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stake_list: Vec<ClubStakingDetails>,
    club_name: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }
    let contract_address = env.clone().contract.address.into_string();

    for stake in stake_list.clone() {
        if stake.club_name != club_name {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("Passed club names do not match"),
            }));
        }
    }

    //check if the club_name is available for staking
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("Cannot find the club"),
            }));
        }
    }
    if !(ownership_details.is_some()) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("The club is not available for staking"),
        }));
    }
    let owner = ownership_details.unwrap();

    let mut total_amount = Uint128::zero();
    for stake in stake_list {
        let mut staker = stake.staker_address.clone();
        let mut amount = stake.staked_amount;
        let mut auto_stake = stake.auto_stake;
        total_amount += amount;

        // Now save the staking details
        save_staking_details(
            deps.storage,
            env.clone(),
            staker.clone(),
            club_name.clone(),
            amount,
            auto_stake,
            INCREASE_STAKE,
        )?;
    }

    let transfer_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.into_string(),
        recipient: contract_address,
        amount: total_amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: config.minting_contract_address.to_string(),
        msg: to_binary(&transfer_msg).unwrap(),
        funds: vec![],
    };

    let send_wasm: CosmosMsg = CosmosMsg::Wasm(exec);
    let data_msg = format!("Assign Stakes To Club {} received", total_amount).into_bytes();
    return Ok(Response::new()
        .add_message(send_wasm)
        .add_attribute("action", "assign_stakes_to_a_club")
        .add_attribute("club_name", club_name)
        .add_attribute("total_stake", total_amount.to_string())
        .set_data(data_msg));
}

fn withdraw_stake_from_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    club_name: String,
    withdrawal_amount: Uint128,
    immediate_withdrawal: bool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let staker_addr = deps.api.addr_validate(&staker)?;
    //Check if withdrawer is same as invoker
    if staker_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    //check if the club_name is available for staking
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    let required_ust_fees: Uint128;
    //To bypass calls from unit tests
    if info.sender.clone().into_string() == String::from("Staker001")
        || info.sender.clone().into_string() == String::from("Staker002")
        || info.sender.clone().into_string() == String::from("Staker003")
        || info.sender.clone().into_string() == String::from("Staker004")
        || info.sender.clone().into_string() == String::from("Staker005")
        || info.sender.clone().into_string() == String::from("Staker006")
    {
        required_ust_fees = Uint128::zero();
    } else {
        required_ust_fees = query_platform_fees(
            deps.as_ref(),
            to_binary(&ExecuteMsg::StakeWithdrawFromAClub {
                staker: staker.clone(),
                club_name: club_name.clone(),
                amount: withdrawal_amount,
                immediate_withdrawal,
            })?,
        )?;
    }
    let mut fees = Uint128::zero();
    for fund in info.funds.clone() {
        if fund.denom == "uusd" {
            fees = fees.checked_add(fund.amount).unwrap();
        }
    }
    let adjusted_ust_fees = required_ust_fees
        * (Uint128::from(NINETY_NINE_NINE_PERCENT))
        / (Uint128::from(HUNDRED_PERCENT));
    if fees < adjusted_ust_fees {
        return Err(ContractError::InsufficientFees {
            required: required_ust_fees,
            received: fees,
        });
    }

    let mut stakes = Vec::new();
    let all_stakes = CLUB_STAKING_DETAILS.may_load(deps.storage, (&club_name.clone(), &staker.clone()))?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("No stake found for this club"),
            }));
        }
    }
    let mut user_stake_exists = false;
    let mut withdrawal_amount_in_excess = false;
    for stake in stakes {
        if staker == stake.staker_address {
            user_stake_exists = true;
            if stake.staked_amount < withdrawal_amount {
                withdrawal_amount_in_excess = true;
            }
        }
    }
    if !user_stake_exists {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("User has not staked in this club"),
        }));
    }

    let mut transfer_confirmed = false;
    let mut action = "withdraw_stake".to_string();
    let mut burn_amount = Uint128::zero();
    if ownership_details.is_some() {
        let owner = ownership_details.unwrap();
        let mut unbonded_amount = Uint128::zero();
        let mut bonded_amount = Uint128::zero();
        let mut amount_remaining = withdrawal_amount.clone();

        if immediate_withdrawal == IMMEDIATE_WITHDRAWAL {
            // parse bonding to check maturity and sort with descending order of timestamp
            let mut bonds = Vec::new();
            let mut all_bonds = CLUB_BONDING_DETAILS.may_load(deps.storage, (&club_name.clone(), &staker.clone()))?;
            let mut s_bonds = Vec::new();
            match all_bonds {
                Some(some_bonds) => {
                    bonds = some_bonds;
                    for bond in bonds {
                        s_bonds.push((bond.bonding_start_timestamp.seconds(), bond.clone()));
                    }
                }
                None => {}
            }

            //  sort using first element, ie timestamp
            s_bonds.sort_by(|a, b| b.0.cmp(&a.0));

            let existing_bonds = s_bonds.clone();
            let mut updated_bonds = Vec::new();

            // PRE-MATURITY BOND are extracted here
            // let mut bonded_bonds = Vec::new();

            for bond in existing_bonds {
                let mut updated_bond = bond.1.clone();
                if staker_addr == bond.1.bonder_address {
                    println!(
                        "staker {:?} timestamp  {:?} amount {:?}",
                        staker_addr, bond.1.bonding_start_timestamp, bond.1.bonded_amount
                    );
                    if bond.1.bonding_start_timestamp
                        < env.block.time.minus_seconds(bond.1.bonding_duration)
                    {
                        if amount_remaining > Uint128::zero() {
                            if bond.1.bonded_amount > amount_remaining {
                                unbonded_amount += amount_remaining;
                                updated_bond.bonded_amount -= amount_remaining;
                                amount_remaining = Uint128::zero();
                                updated_bonds.push(updated_bond);
                            } else {
                                unbonded_amount += bond.1.bonded_amount;
                                amount_remaining -= bond.1.bonded_amount;
                            }
                        } else {
                            updated_bonds.push(updated_bond);
                        }
                    } else {
                        // PRE-MATURITY BOND ENCASH AT DISCOUNT - enable the following line
                        // bonded_bonds.push(updated_bond);
                        // PRE-MATURITY BOND ENCASH AT DISCOUNT - bypased or masked using this line
                        updated_bonds.push(updated_bond);
                    }
                } else {
                    updated_bonds.push(updated_bond);
                }
            }

            // // This section Checks the Pre-Maturity Bonds for possible encashment
            // for bond in bonded_bonds {
            //     let mut updated_bond = bond.clone();
            //     if amount_remaining > Uint128::zero() {
            //         if bond.bonded_amount > amount_remaining {
            //             bonded_amount = amount_remaining;
            //             updated_bond.bonded_amount -= amount_remaining;
            //             amount_remaining = Uint128::zero();
            //             updated_bonds.push(updated_bond);
            //         } else {
            //             bonded_amount += bond.bonded_amount;
            //             amount_remaining -= bond.bonded_amount;
            //         }
            //     } else {
            //         updated_bonds.push(updated_bond);
            //     }
            // }


            CLUB_BONDING_DETAILS.save(deps.storage, (&club_name.clone(), &staker.clone()), &updated_bonds)?;

            // update the staking details
            save_staking_details(
                deps.storage,
                env.clone(),
                staker.clone(),
                club_name.clone(),
                (withdrawal_amount - unbonded_amount) - bonded_amount,
                DONT_CHANGE_AUTO_STAKE_SETTING,
                DECREASE_STAKE,
            )?;

            // // PRE-MATURITY Withdrawal directly from Basic Stake , not even into Bonding - commented out to bypass 
            if withdrawal_amount > unbonded_amount {
                println!("Not Sufficient Matured Unstaked Bonds");
                // // Deduct 10% and burn it
                //     burn_amount = (withdrawal_amount - unbonded_amount)
                //         .checked_mul(Uint128::from(10u128))
                //         .unwrap_or_default()
                //         .checked_div(Uint128::from(100u128))
                //         .unwrap_or_default();
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Not Sufficient Matured Unstaked Bonds"),
                }));
            };

            // // Continue if reached here
            // Remaining 90% transfer to staker wallet
            transfer_confirmed = true;
        } else {
            if withdrawal_amount_in_excess {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Excess amount demanded for unstaking"),
                }));
            }

            let all_bonds = CLUB_BONDING_DETAILS.may_load(deps.storage, (&club_name.clone(), &staker.clone()))?.unwrap_or_default();
            let bonds_for_staker = all_bonds.len() as u64;
            if config.max_bonding_limit_per_user <= bonds_for_staker {
                println!("bonds for this staker = {:?}", bonds_for_staker);
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Too many bonded stakes for this staker"),
                }));
            }

            let action = "withdrawn_stake_bonded".to_string();
            // update the staking details
            save_staking_details(
                deps.storage,
                env.clone(),
                staker.clone(),
                club_name.clone(),
                withdrawal_amount,
                DONT_CHANGE_AUTO_STAKE_SETTING,
                DECREASE_STAKE,
            )?;

            // Move the withdrawn stakes to bonding list
            save_bonding_details(
                deps.storage,
                env.clone(),
                staker.clone(),
                club_name.clone(),
                withdrawal_amount,
                config.bonding_duration,
            )?;

            let mut rsp = Response::new();
            let send_bank: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
                to_address: config.platform_fees_collector_wallet.into_string(),
                amount: info.funds,
            });

            // early exit with only state change and platform fee transfer - no token exchange
            let data_msg = format!("Amount {} bonded", withdrawal_amount).into_bytes();
            rsp = rsp
                .add_message(send_bank)
                .add_attribute("action", action)
                .add_attribute("bonded", withdrawal_amount.clone().to_string())
                .set_data(data_msg);
            return Ok(rsp);
        }
    } else {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Invalid club"),
        }));
    }

    if transfer_confirmed == false {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Not a valid staker for the club"),
        }));
    }

    let mut rsp = Response::new();

    // transfer_with_burn(deps.storage, staker.clone(), withdrawal_amount, burn_amount, "staking_withdraw".to_string())
    if burn_amount > Uint128::zero() {
        let burn_msg = Cw20ExecuteMsg::Burn {
            amount: burn_amount.clone(),
        };
        let exec_burn = WasmMsg::Execute {
            contract_addr: config.minting_contract_address.to_string(),
            msg: to_binary(&burn_msg).unwrap(),
            funds: vec![],
        };
        let burn_wasm: CosmosMsg = CosmosMsg::Wasm(exec_burn);
        rsp = rsp
            .add_message(burn_wasm)
            .add_attribute("burnt", burn_amount.to_string());
    }
    let transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: staker,
        amount: withdrawal_amount - burn_amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: config.minting_contract_address.to_string(),
        msg: to_binary(&transfer_msg).unwrap(),
        funds: vec![],
    };
    let send_wasm: CosmosMsg = CosmosMsg::Wasm(exec);
    let send_bank: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.platform_fees_collector_wallet.into_string(),
        amount: info.funds,
    });

    let data_msg = format!("Amount {} transferred", withdrawal_amount).into_bytes();
    rsp = rsp
        .add_message(send_wasm)
        .add_message(send_bank)
        .add_attribute("action", action)
        .add_attribute("withdrawn", withdrawal_amount.clone().to_string())
        .set_data(data_msg);
    return Ok(rsp);
}

fn save_staking_details(
    storage: &mut dyn Storage,
    env: Env,
    staker: String,
    club_name: String,
    amount: Uint128,
    auto_stake: bool,
    increase_stake: bool,
) -> Result<Response, ContractError> {
    // Get the exising stakes for this club
    let mut stakes = Vec::new();
    let all_stakes = CLUB_STAKING_DETAILS.may_load(storage, (&club_name.clone(), &staker.clone()))?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }

    // if already staked for this club, then increase or decrease the staked_amount in existing stake
    let mut already_staked = false;
    let existing_stakes = stakes.clone();
    let mut updated_stakes = Vec::new();
    for stake in existing_stakes {
        let mut updated_stake = stake.clone();
        if staker == stake.staker_address {
            if increase_stake == INCREASE_STAKE {
                updated_stake.staked_amount += amount;
                updated_stake.auto_stake = auto_stake;
                if auto_stake == SET_AUTO_STAKE {
                    updated_stake.staked_amount += updated_stake.reward_amount;
                    updated_stake.reward_amount = Uint128::zero();
                }
            } else {
                if updated_stake.staked_amount >= amount {
                    updated_stake.staked_amount -= amount;
                } else {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: String::from("Excess amount demanded for withdrawal"),
                    }));
                }
            }
            already_staked = true;
        }
        updated_stakes.push(updated_stake);
    }
    if already_staked == true {
        // save the modified stakes - with updation or removal of existing stake
        CLUB_STAKING_DETAILS.save(storage, (&club_name.clone(), &staker.clone()), &updated_stakes)?;
    } else if increase_stake == INCREASE_STAKE {
        stakes.push(ClubStakingDetails {
            staker_address: staker.clone(),
            staking_start_timestamp: env.block.time,
            staked_amount: amount,
            staking_duration: CLUB_STAKING_DURATION,
            club_name: club_name.clone(),
            reward_amount: Uint128::from(CLUB_STAKING_REWARD_AMOUNT), // ensure that the first time reward amount is set to 0
            auto_stake: auto_stake,
        });
        CLUB_STAKING_DETAILS.save(storage, (&club_name.clone(), &staker.clone()), &stakes)?;
    }

    // Now update the total stake for this club
    let owner = CLUB_OWNERSHIP_DETAILS.load(storage, club_name.clone())?;
    let mut total_staked_amount = owner.total_staked_amount;
    if increase_stake == INCREASE_STAKE {
        total_staked_amount += amount;
    } else {
        total_staked_amount -= amount;
    }
    CLUB_OWNERSHIP_DETAILS.save(
        storage,
        club_name.clone(),
        &ClubOwnershipDetails {
            club_name: owner.club_name.clone(),
            start_timestamp: owner.start_timestamp,
            locking_period: owner.locking_period,
            owner_address: owner.owner_address,
            price_paid: owner.price_paid,
            reward_amount: owner.reward_amount,
            owner_released: owner.owner_released,
            total_staked_amount: total_staked_amount,
        },
    )?;

    return Ok(Response::default());
}

fn save_bonding_details(
    storage: &mut dyn Storage,
    env: Env,
    bonder: String,
    club_name: String,
    bonded_amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    // Get the exising bonds for this club
    let mut bonds = Vec::new();
    let all_bonds = CLUB_BONDING_DETAILS.may_load(storage, (&club_name.clone(), &bonder.clone()))?;
    match all_bonds {
        Some(some_bonds) => {
            bonds = some_bonds;
        }
        None => {}
    }
    bonds.push(ClubBondingDetails {
        bonder_address: bonder.clone(),
        bonding_start_timestamp: env.block.time,
        bonded_amount: bonded_amount,
        bonding_duration: duration,
        club_name: club_name.clone(),
    });
    CLUB_BONDING_DETAILS.save(storage, (&club_name.clone(), &bonder.clone()), &bonds)?;
    return Ok(Response::default());
}

fn increase_reward_amount(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    reward_from: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // For SECURITY This message MUST only come from the Admin
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }
    let existing_reward = REWARD.may_load(deps.storage)?.unwrap_or_default();
    let new_reward = existing_reward + amount;
    REWARD.save(deps.storage, &new_reward)?;

    let reward_given_in_current_timestamp = Uint128::zero();
    REWARD_GIVEN_IN_CURRENT_TIMESTAMP.save(deps.storage, &reward_given_in_current_timestamp)?;

    // get the actual transfer from the wallet containing funds
    // transfer_from_wallet_to_contract(deps.storage, config.admin_address.to_string(), amount);
    // NOTHING required to transfer anything staking fund has arrived in the staking contract

    return Ok(Response::default());
}

fn claim_staker_rewards(
    deps: DepsMut,
    info: MessageInfo,
    staker: String,
    club_name: String,
) -> Result<Response, ContractError> {
    let mut transfer_confirmed = false;
    let mut amount = Uint128::zero();
    let staker_addr = deps.api.addr_validate(&staker)?;
    //Check if withdrawer is same as invoker
    if staker_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let required_ust_fees = query_platform_fees(
        deps.as_ref(),
        to_binary(&ExecuteMsg::ClaimStakerRewards {
            staker: staker.clone(),
            club_name: club_name.clone(),
        })?,
    )?;
    let mut fees = Uint128::zero();
    for fund in info.funds.clone() {
        if fund.denom == "uusd" {
            fees = fees.checked_add(fund.amount).unwrap();
        }
    }
    let adjusted_ust_fees = required_ust_fees
        * (Uint128::from(NINETY_NINE_NINE_PERCENT))
        / (Uint128::from(HUNDRED_PERCENT));
    if fees < adjusted_ust_fees {
        return Err(ContractError::InsufficientFees {
            required: required_ust_fees,
            received: fees,
        });
    }

    // Get the exising stakes for this club
    let mut stakes = Vec::new();
    let all_stakes = CLUB_STAKING_DETAILS.may_load(deps.storage, (&club_name.clone(), &staker.clone()))?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }

    let existing_stakes = stakes.clone();
    let mut updated_stakes = Vec::new();
    for stake in existing_stakes {
        let mut updated_stake = stake.clone();
        if staker == stake.staker_address {
            amount += updated_stake.reward_amount;
            updated_stake.reward_amount = Uint128::zero();
            // confirm transfer to staker wallet
            transfer_confirmed = true;
        }
        updated_stakes.push(updated_stake);
    }
    CLUB_STAKING_DETAILS.save(deps.storage, (&club_name.clone(), &staker.clone()), &stakes)?;

    if transfer_confirmed == false {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Not a valid staker for the club"),
        }));
    }
    if amount == Uint128::zero() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("No rewards for this user"),
        }));
    }

    let config = CONFIG.load(deps.storage)?;

    let transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: staker.clone(),
        amount: amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: config.minting_contract_address.to_string(),
        msg: to_binary(&transfer_msg).unwrap(),
        funds: vec![],
    };
    let send_wasm: CosmosMsg = CosmosMsg::Wasm(exec);
    let send_bank: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.platform_fees_collector_wallet.into_string(),
        amount: info.funds,
    });
    let data_msg = format!("Amount {} transferred", amount).into_bytes();
    return Ok(Response::new()
        .add_message(send_wasm)
        .add_message(send_bank)
        .add_attribute("action", "staking_reward_claim")
        .add_attribute("staker", staker)
        .add_attribute("amount", amount.to_string())
        .set_data(data_msg));
}

fn calculate_and_distribute_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker_list: Vec<String>,
    club_name: String,
    is_first_batch: bool,
    is_final_batch: bool,
) -> Result<Response, ContractError> {
    // Check if this is executed by main/transaction wallet
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("not authorised"),
        }));
    }
    let total_reward = REWARD.may_load(deps.storage)?.unwrap_or_default();

    let mut next_reward_time = CLUB_REWARD_NEXT_TIMESTAMP
        .may_load(deps.storage)?
        .unwrap_or_default();
    println!(
        "now = {:?} next_reward_time = {:?} periodicity = {:?} is_first_batch = {:?} is_final_batch = {:?}",
        env.block.time, next_reward_time, config.reward_periodicity, is_first_batch, is_final_batch
    );

    let saved_next_reward_timestamp = next_reward_time;

    if env.block.time < next_reward_time {
        println!("Time for Reward not yet arrived");
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Time for Reward not yet arrived"),
        }));
    }
    if is_final_batch {
        if next_reward_time < env.block.time {
            next_reward_time = next_reward_time.plus_seconds(config.reward_periodicity);
        }
        println!("setting next_reward_time = {:?}", next_reward_time);
        CLUB_REWARD_NEXT_TIMESTAMP.save(deps.storage, &next_reward_time)?;
    }

    // No need to calculate if there is no reward amount
    if total_reward == Uint128::zero() {
        return Ok(Response::new().add_attribute("response", "no accumulated rewards")
            .add_attribute("next_timestamp", next_reward_time.to_string())
        );
    }
    distribute_reward_to_club_stakers(deps, env, config.reward_periodicity, saved_next_reward_timestamp,
                                      staker_list.clone(), club_name.clone(), total_reward, is_first_batch, is_final_batch)
}

fn distribute_reward_to_club_stakers(
    deps: DepsMut,
    env: Env,
    reward_periodicity: u64,
    saved_next_reward_timestamp: Timestamp,
    staker_list: Vec<String>,
    club_name: String,
    total_reward: Uint128,
    is_first_batch: bool,
    is_final_batch: bool,
) -> Result<Response, ContractError> {
    let mut winning_clubs_info: WinningClubDetails;
    if is_first_batch {
        let response = get_winning_clubs_details(deps.storage)?;
        winning_clubs_info = WinningClubDetails {
            total_number_of_clubs: response.0,
            total_stake_across_all_clubs: response.1,
            total_stake_in_winning_club: response.2,
            winner_list: response.3.clone(),
        };
        WINNING_CLUB_DETAILS_SNAPSHOT.save(deps.storage, &winning_clubs_info);
    } else {
        winning_clubs_info = WINNING_CLUB_DETAILS_SNAPSHOT.may_load(deps.storage)?.unwrap_or_default();
    }
    println!("winning_clubs_info = {:?}", winning_clubs_info);
    let total_number_of_clubs = winning_clubs_info.total_number_of_clubs;
    let total_stake_across_all_clubs = winning_clubs_info.total_stake_across_all_clubs;
    let total_stake_in_winning_club = winning_clubs_info.total_stake_in_winning_club;
    let winner_list = winning_clubs_info.winner_list.clone();
    let is_club_a_winner = is_winning_club(club_name.clone(), winner_list.clone());
    let club_details = query_club_ownership_details(deps.storage, club_name.clone())?;
    let club_owner_address = club_details.owner_address.clone();
    let num_of_winners = winner_list.len() as u64;
    let other_club_count = total_number_of_clubs - num_of_winners;

    if !is_club_a_winner && other_club_count <= 0 {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("computation error - should never happen"),
        }));
    }

    let mut owner_reward = Uint128::zero();
    let mut reward_for_all_stakers_in_winning_club = Uint128::zero();

    if is_club_a_winner {
        if other_club_count > 0 {
            // distribute 1% equally to owners in this winning club
            owner_reward = total_reward
                .checked_div(Uint128::from(100u128))
                .unwrap_or_default()
                .checked_div(Uint128::from(num_of_winners))
                .unwrap_or_default();
            println!("club_name {:?} owner reward for this winner is {:?}", club_name.clone(), owner_reward);
        } else {
            // there are only winning clubs
            // distribute 3% equally to owners in this winning club
            owner_reward = total_reward
                .checked_mul(Uint128::from(3u128))
                .unwrap_or_default()
                .checked_div(Uint128::from(100u128))
                .unwrap_or_default()
                .checked_div(Uint128::from(num_of_winners))
                .unwrap_or_default();
            println!("all clubs are winners club_name {:?} owner reward for this winner is {:?}", club_name.clone(), owner_reward);
        }
        // distribute 19% to stakers in winning club
        reward_for_all_stakers_in_winning_club = total_reward
            .checked_mul(Uint128::from(19u128))
            .unwrap_or_default()
            .checked_div(Uint128::from(100u128))
            .unwrap_or_default()
            .checked_div(Uint128::from(num_of_winners))
            .unwrap_or_default();
        println!("club_name {:?} stakers award in winner is {:?}", club_name.clone(), reward_for_all_stakers_in_winning_club);
    } else {
        // other_club_count must be greater than 0
        // distribute 2% equally to owner in this non winning club
        owner_reward = total_reward
            .checked_mul(Uint128::from(2u128))
            .unwrap_or_default()
            .checked_div(Uint128::from(100u128))
            .unwrap_or_default()
            .checked_div(Uint128::from(other_club_count))
            .unwrap_or_default();
        println!("club_name {:?} owner reward for non winner is {:?}", club_name.clone(), owner_reward);
    }

    // distribute the 78% to all stakers
    let all_stakers_reward = total_reward
        .checked_mul(Uint128::from(78u128))
        .unwrap_or_default()
        .checked_div(Uint128::from(100u128))
        .unwrap_or_default();

    let mut reward_given_so_far = Uint128::zero();
    let mut stake_to_add_for_club = Uint128::zero();
    for staker in staker_list {
        let mut updated_stakes_for_this_staker = Vec::new();
        let csd = CLUB_STAKING_DETAILS.may_load(deps.storage, (&club_name.clone(), &staker.clone()))?;
        let staking_details;
        match csd {
            None => {}
            Some(some_csd) => {
                staking_details = some_csd;

                for mut stake in staking_details {
                    let mut updated_stake = stake.clone();
                    println!("stake = {:?}", updated_stake);
                    if updated_stake.staking_start_timestamp > env.block.time {
                        continue;
                    }
                    if saved_next_reward_timestamp < env.block.time {
                        updated_stake.staking_start_timestamp = saved_next_reward_timestamp.plus_seconds(reward_periodicity);
                        println!("setting timestamp for stake = {:?}", updated_stake.staking_start_timestamp);
                    } else {
                        updated_stake.staking_start_timestamp = saved_next_reward_timestamp;
                        println!("setting timestamp for stake = {:?}", updated_stake.staking_start_timestamp);
                    }

                    let auto_stake = updated_stake.auto_stake;

                    // Calculate for All Staker - 78% proportional
                    let mut reward_for_this_stake = all_stakers_reward
                        .checked_mul(stake.staked_amount)
                        .unwrap_or_default()
                        .checked_div(total_stake_across_all_clubs)
                        .unwrap_or_default();

                    if is_club_a_winner {
                        // Calculate for Winning Club Staker - 19% proportional
                        reward_for_this_stake += reward_for_all_stakers_in_winning_club
                            .checked_mul(stake.staked_amount)
                            .unwrap_or_default()
                            .checked_div(total_stake_in_winning_club)
                            .unwrap_or_default();
                    }

                    if stake.staker_address == club_owner_address {
                        // Calculate for Club Owner - (1% or 3% for winner owner) or 2% proportional for non-winner owner
                        reward_for_this_stake += owner_reward;
                    }

                    reward_given_so_far += reward_for_this_stake;

                    if auto_stake == SET_AUTO_STAKE {
                        stake_to_add_for_club += reward_for_this_stake;
                        updated_stake.staked_amount += reward_for_this_stake;
                        updated_stake.staked_amount += updated_stake.reward_amount;
                        updated_stake.reward_amount = Uint128::zero();
                    } else {
                        updated_stake.reward_amount += reward_for_this_stake;
                    }
                    updated_stakes_for_this_staker.push(updated_stake);
                }
            }
        }
        CLUB_STAKING_DETAILS.save(deps.storage, (&club_name.clone(), &staker.clone()), &updated_stakes_for_this_staker)?;
    }

    // Now update the total stake for this club
    CLUB_OWNERSHIP_DETAILS.save(
        deps.storage,
        club_name.clone(),
        &ClubOwnershipDetails {
            club_name: club_details.club_name.clone(),
            start_timestamp: club_details.start_timestamp,
            locking_period: club_details.locking_period,
            owner_address: club_details.owner_address,
            price_paid: club_details.price_paid,
            reward_amount: club_details.reward_amount,
            owner_released: club_details.owner_released,
            total_staked_amount: club_details.total_staked_amount + stake_to_add_for_club,
        },
    )?;

    println!("club_name = {:?} total reward = {:?} reward so far = {:?} club stake increased by {:?}",
             club_name.clone(), total_reward, reward_given_so_far, stake_to_add_for_club);

    let mut reward_given_in_current_timestamp = REWARD_GIVEN_IN_CURRENT_TIMESTAMP.may_load(deps.storage)?.unwrap_or_default();
    reward_given_in_current_timestamp += reward_given_so_far;
    REWARD_GIVEN_IN_CURRENT_TIMESTAMP.save(deps.storage, &reward_given_in_current_timestamp)?;
    println!("reward_given_in_current_timestamp = {:?}", reward_given_in_current_timestamp);

    if is_final_batch {
        let mut new_reward = Uint128::zero();
        if total_reward > reward_given_in_current_timestamp {
            new_reward = total_reward - reward_given_in_current_timestamp;
        }
        REWARD.save(deps.storage, &new_reward)?;
        println!("new_reward = {:?} ", new_reward);
    }
    Ok(Response::default())
}

fn get_winning_clubs_details(
    storage: &mut dyn Storage,
) -> StdResult<(u64, Uint128, Uint128, Vec<String>)> {
    let mut max_incremental_stake_value = 0i128 - MAX_UFURY_COUNT;
    let mut max_total_stake_value = Uint128::zero();

    let mut total_number_of_clubs = 0u64;
    let mut total_stake_across_all_clubs = Uint128::zero();
    let mut total_stake_in_winning_club = Uint128::zero();
    let mut winners: Vec<String> = Vec::new();

    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club in all_clubs {
        let club_details = query_club_ownership_details(storage, club.clone())?;
        let stake_in_club = club_details.total_staked_amount;
        total_stake_across_all_clubs += stake_in_club;
        let staked_amount_u128: u128 = stake_in_club.into();
        let staked_amount_i128 = staked_amount_u128 as i128;
        let previous_amount = CLUB_STAKING_SNAPSHOT.may_load(storage, club.clone())?.unwrap_or_default();
        let previous_amount_u128: u128 = previous_amount.into();
        let previous_amount_i128 = previous_amount_u128 as i128;
        let difference_amount = staked_amount_i128 - previous_amount_i128;

        if difference_amount > max_incremental_stake_value {
            // found a new max_incremental_stake
            total_stake_in_winning_club = stake_in_club;
            max_incremental_stake_value = difference_amount;
            max_total_stake_value = stake_in_club;

            // pop all pre-existing winners
            let winner_len = winners.len();
            let mut i = 0;
            while i < winner_len {
                winners.swap_remove(0);
                i += 1;
            }

            // now add this stake to the winner list
            winners.push(club.clone());
        } else if difference_amount == max_incremental_stake_value {
            if stake_in_club >= max_total_stake_value {
                total_stake_in_winning_club = stake_in_club;
                max_incremental_stake_value = difference_amount;
                max_total_stake_value = stake_in_club;
                if stake_in_club > max_total_stake_value {
                    // found a new max_total_stake

                    // pop all pre-existing winners
                    let winner_len = winners.len();
                    let mut i = 0;
                    while i < winner_len {
                        winners.swap_remove(0);
                        i += 1;
                    }

                    // now add this stake to the winner list
                    winners.push(club.clone());
                } else if stake_in_club == max_total_stake_value {
                    // more than one winners have same total and incremental stake

                    // add this stake to the winner list
                    winners.push(club.clone());
                }
                // else skip this club
            }
        }
        // else skip this club

        total_number_of_clubs += 1;
        CLUB_STAKING_SNAPSHOT.save(storage, club.clone(), &stake_in_club)?;
    }

    println!("total_clubs = {:?}, total_stake = {:?}, winning_stake = {:?}, winners = {:?}",
             total_number_of_clubs,
             total_stake_across_all_clubs,
             total_stake_in_winning_club,
             winners);
    Ok((total_number_of_clubs,
        total_stake_across_all_clubs,
        total_stake_in_winning_club,
        winners))
}


fn is_winning_club(
    club_name: String,
    winner_list: Vec<String>,
) -> bool {
    for winner in winner_list {
        if club_name == winner {
            return true;
        }
    }
    return false;
}

fn transfer_from_contract_to_wallet(
    store: &dyn Storage,
    wallet_owner: String,
    amount: Uint128,
    action: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(store)?;

    let transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: wallet_owner,
        amount: amount,
    };
    let exec = WasmMsg::Execute {
        contract_addr: config.minting_contract_address.to_string(),
        msg: to_binary(&transfer_msg).unwrap(),
        funds: vec![
            // Coin {
            //     denom: token_info.name.to_string(),
            //     amount: price,
            // },
        ],
    };
    let send: SubMsg = SubMsg::new(exec);
    let data_msg = format!("Amount {} transferred", amount).into_bytes();
    return Ok(Response::new()
        .add_submessage(send)
        .add_attribute("action", action)
        .add_attribute("amount", amount.to_string())
        .set_data(data_msg));
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryPlatformFees { msg } => to_binary(&query_platform_fees(deps, msg)?),
        QueryMsg::ClubStakingDetails { club_name, user_list } => {
            to_binary(&query_club_staking_details(deps.storage, club_name, user_list)?)
        }
        QueryMsg::ClubOwnershipDetails { club_name } => {
            to_binary(&query_club_ownership_details(deps.storage, club_name)?)
        }
        QueryMsg::ClubPreviousOwnershipDetails { previous_owner } => to_binary(
            &query_club_previous_owner_details(deps.storage, previous_owner)?,
        ),
        QueryMsg::AllClubOwnershipDetails {} => {
            to_binary(&query_all_club_ownership_details(deps.storage)?)
        }
        QueryMsg::AllPreviousClubOwnershipDetails {} => {
            to_binary(&query_all_previous_club_ownership_details(deps.storage)?)
        }
        QueryMsg::ClubOwnershipDetailsForOwner { owner_address } => to_binary(
            &query_club_ownership_details_for_owner(deps.storage, owner_address)?,
        ),
        QueryMsg::AllStakes { user_address_list } => to_binary(&query_all_stakes(deps.storage, user_address_list)?),
        QueryMsg::AllStakesForUser { user_address } => {
            to_binary(&query_all_stakes_for_user(deps.storage, user_address)?)
        }
        QueryMsg::AllBonds { user_address_list } => to_binary(&query_all_bonds(deps.storage, user_address_list)?),
        QueryMsg::ClubBondingDetailsForUser {
            user_address,
            club_name,
        } => to_binary(&query_club_bonding_details_for_user(
            deps.storage,
            user_address,
            club_name,
        )?),
        QueryMsg::RewardAmount {} => to_binary(&query_reward_amount(deps.storage)?),
        QueryMsg::QueryStakerRewards {
            staker,
            club_name,
        } => to_binary(&query_staker_rewards(deps, staker, club_name)?),
    }
}

pub fn query_platform_fees(deps: Deps, msg: Binary) -> StdResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;
    let platform_fees_percentage: Uint128;
    let fury_amount_provided;
    match from_binary(&msg) {
        Ok(ExecuteMsg::IncreaseRewardAmount {
               reward_from: _,
               amount: _,
           }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::BuyAClub {
               buyer: _,
               seller: _,
               club_name: _,
               auto_stake: _,
           }) => {
            platform_fees_percentage = config.platform_fees + config.transaction_fees;
            fury_amount_provided = config.club_price;
        }
        Ok(ExecuteMsg::AssignAClub {
               buyer: _,
               seller: _,
               club_name: _,
               auto_stake: _,
           }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::StakeOnAClub {
               staker: _,
               club_name: _,
               amount,
               auto_stake: _,
           }) => {
            platform_fees_percentage = config.platform_fees + config.transaction_fees + config.control_fees;
            fury_amount_provided = amount;
        }
        Ok(ExecuteMsg::AssignStakesToAClub {
               stake_list: _,
               club_name: _,
           }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::ReleaseClub { owner: _, club_name: _ }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::ClaimOwnerRewards { owner: _, club_name: _ }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::ClaimPreviousOwnerRewards { previous_owner: _ }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::StakeWithdrawFromAClub {
               staker: _,
               club_name: _,
               amount,
               immediate_withdrawal: _,
           }) => {
            platform_fees_percentage = config.platform_fees + config.transaction_fees;
            fury_amount_provided = amount;
        }
        Ok(ExecuteMsg::CalculateAndDistributeRewards {
               staker_list: _,
               club_name: _,
               is_first_batch: _,
               is_final_batch: _,
           }) => {
            return Ok(Uint128::zero());
        }
        Ok(ExecuteMsg::ClaimStakerRewards { staker, club_name }) => {
            fury_amount_provided = query_staker_rewards(deps, staker, club_name)?;
            platform_fees_percentage = config.platform_fees + config.transaction_fees;
        }
        Err(err) => {
            return Err(StdError::generic_err(format!("{:?}", err)));
        }
    }
    let ust_equiv_for_fury: Uint128 = deps
        .querier
        .query_wasm_smart(config.astro_proxy_address, &ProxyQueryMsgs::get_ust_equivalent_to_fury {
            fury_count: fury_amount_provided,
        })?;

    return Ok(ust_equiv_for_fury
        .checked_mul(platform_fees_percentage)?
        .checked_div(Uint128::from(HUNDRED_PERCENT))?);
}

pub fn query_club_staking_details(
    storage: &dyn Storage,
    club_name: String,
    user_list: Vec<String>,
) -> StdResult<Vec<ClubStakingDetails>> {
    let mut all_stakes = Vec::new();
    for user in user_list {
        let csd = CLUB_STAKING_DETAILS.may_load(storage, (&club_name.clone(), &user.clone()))?;
        match csd {
            Some(staking_details) => {
                for stake in staking_details {
                    all_stakes.push(stake);
                }
            }
            None => {}
        }
    }
    return Ok(all_stakes);
}

fn query_all_stakes(storage: &dyn Storage, user_address_list: Vec<String>) -> StdResult<Vec<ClubStakingDetails>> {
    let mut all_stakes = Vec::new();
    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        for user_address in user_address_list.clone() {
            let csd = CLUB_STAKING_DETAILS.may_load(storage, (&club_name.clone(), &user_address.clone()))?;
            match csd {
                Some(staking_details) => {
                    for stake in staking_details {
                        all_stakes.push(stake);
                    }
                }
                None => {}
            }
        }
    }
    return Ok(all_stakes);
}

fn query_all_bonds(storage: &dyn Storage, user_address_list: Vec<String>) -> StdResult<Vec<ClubBondingDetails>> {
    let mut all_bonds = Vec::new();
    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        for user_address in user_address_list.clone() {
            let cbd = CLUB_BONDING_DETAILS.may_load(storage, (&club_name.clone(), &user_address.clone()))?;
            match cbd {
                Some(bonding_details) => {
                    for bond in bonding_details {
                        all_bonds.push(bond);
                    }
                }
                None => {}
            }
        }
    }
    return Ok(all_bonds);
}

fn query_reward_amount(storage: &dyn Storage) -> StdResult<Uint128> {
    let reward: Uint128 = REWARD.may_load(storage)?.unwrap_or_default();
    return Ok(reward);
}

fn query_staker_rewards(
    deps: Deps,
    staker: String,
    club_name: String,
) -> StdResult<Uint128> {
    // Get the exising stakes for this club
    let mut stakes = Vec::new();
    let all_stakes = CLUB_STAKING_DETAILS.may_load(deps.storage, (&club_name.clone(), &staker.clone()))?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }
    let mut amount = Uint128::zero();
    for stake in stakes {
        if staker == stake.staker_address {
            amount += stake.reward_amount;
        }
    }
    return Ok(amount);
}

fn query_club_ownership_details(
    storage: &dyn Storage,
    club_name: String,
) -> StdResult<ClubOwnershipDetails> {
    let cod = CLUB_OWNERSHIP_DETAILS.may_load(storage, club_name)?;
    match cod {
        Some(cod) => return Ok(cod),
        None => return Err(StdError::generic_err("No ownership details found")),
    };
}

pub fn query_club_previous_owner_details(
    storage: &dyn Storage,
    previous_owner: String,
) -> StdResult<ClubPreviousOwnerDetails> {
    let cod = CLUB_PREVIOUS_OWNER_DETAILS.may_load(storage, previous_owner)?;
    match cod {
        Some(cod) => return Ok(cod),
        None => return Err(StdError::generic_err("No previous ownership details found")),
    };
}

pub fn query_all_stakes_for_user(
    storage: &dyn Storage,
    user_address: String,
) -> StdResult<Vec<ClubStakingDetails>> {
    let mut all_stakes = Vec::new();
    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        let staking_details = CLUB_STAKING_DETAILS.load(storage, (&club_name.clone(), &user_address.clone()))?;
        for stake in staking_details {
            if stake.staker_address == user_address {
                all_stakes.push(stake);
            }
        }
    }
    return Ok(all_stakes);
}

pub fn query_club_bonding_details_for_user(
    storage: &dyn Storage,
    club_name: String,
    user_address: String,
) -> StdResult<Vec<ClubBondingDetails>> {
    let mut bonds: Vec<ClubBondingDetails> = Vec::new();
    let cbd = CLUB_BONDING_DETAILS.may_load(storage, (&club_name.clone(), &user_address.clone()))?;
    match cbd {
        Some(cbd) => {
            bonds = cbd;
        }
        None => return Err(StdError::generic_err("No bonding details found")),
    };
    let mut all_bonds = Vec::new();
    for bond in bonds {
        if bond.bonder_address == user_address {
            all_bonds.push(bond);
        }
    }
    return Ok(all_bonds);
}


pub fn query_all_club_ownership_details(
    storage: &dyn Storage,
) -> StdResult<Vec<ClubOwnershipDetails>> {
    let mut all_owners = Vec::new();
    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        let owner_details = CLUB_OWNERSHIP_DETAILS.load(storage, club_name)?;
        all_owners.push(owner_details);
    }
    return Ok(all_owners);
}

pub fn query_all_previous_club_ownership_details(
    storage: &dyn Storage,
) -> StdResult<Vec<ClubPreviousOwnerDetails>> {
    let mut pcod = Vec::new();
    let all_previous: Vec<String> = CLUB_PREVIOUS_OWNER_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for previous in all_previous {
        let previous_details = CLUB_PREVIOUS_OWNER_DETAILS.load(storage, previous)?;
        pcod.push(previous_details);
    }
    return Ok(pcod);
}

pub fn query_club_ownership_details_for_owner(
    storage: &dyn Storage,
    owner_address: String,
) -> StdResult<Vec<ClubOwnershipDetails>> {
    let mut all_owners = Vec::new();
    let all_clubs: Vec<String> = CLUB_OWNERSHIP_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        let owner_details = CLUB_OWNERSHIP_DETAILS.load(storage, club_name)?;
        if owner_details.owner_address == owner_address {
            all_owners.push(owner_details);
        }
    }
    return Ok(all_owners);
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, coins, CosmosMsg, from_binary, StdError, SubMsg, WasmMsg};
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn test_buying_of_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let query_res = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match query_res {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, false);
                assert_eq!(cod.reward_amount, Uint128::from(CLUB_BUYING_REWARD_AMOUNT));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_owner_claim_rewards() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        let result = buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );
        println!("result = {:?}", result);
        let query_res = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match query_res {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, false);
                assert_eq!(cod.reward_amount, Uint128::from(CLUB_BUYING_REWARD_AMOUNT));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        claim_owner_rewards(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            "CLUB001".to_string(),
        );

        let queryResAfter = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfter {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, false);
                assert_eq!(cod.reward_amount, Uint128::from(0u128));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_multiple_buying_of_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let owner2_info = mock_info("Owner002", &[coin(1000, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner2_info.clone(),
            "Owner002".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let query_res = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match query_res {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_assign_a_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(1000, "stake")]);
        let owner2_info = mock_info("Owner002", &[coin(1000, "stake")]);

        println!("Now assigning the club to Owner001");
        assign_a_club(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            SET_AUTO_STAKE,
        );

        let queryRes0 = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes0 {
            Ok(mut cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(0u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        println!("Now releasing the club from Owner001");
        release_club(deps.as_mut(), mock_env(), owner1_info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let queryRes1 = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes1 {
            Ok(mut cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(0u128));
                assert_eq!(cod.owner_released, true);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        println!("Now assigning the club to Owner002");
        assign_a_club(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            "Owner002".to_string(),
            Some("Owner001".to_string()),
            "CLUB001".to_string(),
            SET_AUTO_STAKE,
        );

        println!("Now releasing the club from Owner002");
        release_club(deps.as_mut(), mock_env(), owner2_info.clone(), "Owner002".to_string(), "CLUB001".to_string());

        let queryRes2 = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes2 {
            Ok(mut cod) => {
                assert_eq!(cod.owner_address, "Owner002".to_string());
                assert_eq!(cod.price_paid, Uint128::from(0u128));
                assert_eq!(cod.owner_released, true);
                cod.start_timestamp = now.minus_seconds(22 * 24 * 60 * 60);
                CLUB_OWNERSHIP_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &cod);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        println!("Now trying to assign the club to Owner003 - should fail");
        assign_a_club(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            "Owner003".to_string(),
            Some("Owner002".to_string()),
            "CLUB001".to_string(),
            SET_AUTO_STAKE,
        );

        let queryRes3 = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes3 {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner002".to_string());
                assert_eq!(cod.price_paid, Uint128::from(0u128));
                assert_eq!(cod.owner_released, true);
                assert_eq!(cod.start_timestamp, now.minus_seconds(22 * 24 * 60 * 60));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_assign_stakes_to_a_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(1000, "stake")]);

        println!("Now assigning the club to Owner001");
        assign_a_club(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            SET_AUTO_STAKE,
        );

        let queryRes0 = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes0 {
            Ok(mut cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(0u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let mut stake_list: Vec<ClubStakingDetails> = Vec::new();
        let mut user_address_list = Vec::new();
        user_address_list.push("Owner001".to_string());
        for i in 1..7 {
            let mut staker = String::default();
            match i {
                1 => { staker = "Staker001".to_string(); }
                2 => { staker = "Staker002".to_string(); }
                3 => { staker = "Staker003".to_string(); }
                4 => { staker = "Staker004".to_string(); }
                5 => { staker = "Staker005".to_string(); }
                6 => { staker = "Staker006".to_string(); }
                _ => {}
            }
            user_address_list.push(staker.clone());
            println!("staker is {}", staker);
            stake_list.push(ClubStakingDetails {
                staker_address: staker,
                staking_start_timestamp: now,
                staked_amount: Uint128::from(330000u128),
                staking_duration: CLUB_STAKING_DURATION,
                club_name: "CLUB001".to_string(),
                reward_amount: Uint128::from(CLUB_STAKING_REWARD_AMOUNT),
                auto_stake: SET_AUTO_STAKE,
            });
        };

        let staker6Info = mock_info("Staker006", &[coin(10, "stake")]);
        assign_stakes_to_a_club(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            stake_list,
            "CLUB001".to_string(),
        );

        let queryRes1 = query_all_stakes(&mut deps.storage, user_address_list);
        match queryRes1 {
            Ok(all_stakes) => {
                println!("all stakes : {:?}", all_stakes);
                assert_eq!(all_stakes.len(), 7);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_buying_of_club_after_releasing_by_prev_owner() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        let mut resp = buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );
        println!("{:?}", resp);
        resp = release_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            "CLUB001".to_string(),
        );
        println!("{:?}", resp);

        let now = mock_env().block.time; // today

        let query_res = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match query_res {
            Ok(mut cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                cod.start_timestamp = now.minus_seconds(22 * 24 * 60 * 60);
                CLUB_OWNERSHIP_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &cod);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        release_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            "CLUB001".to_string(),
        );

        let queryResAfterReleasing =
            query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterReleasing {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, true);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let owner2_info = mock_info("Owner002", &[coin(0, "uusd")]);
        let resp = buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner2_info.clone(),
            "Owner002".to_string(),
            Some("Owner001".to_string()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );
        println!("{:?}", resp);
        let queryResAfterSellingByPrevOwner =
            query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterSellingByPrevOwner {
            Ok(cod) => {
                assert_eq!(cod.owner_address, "Owner002".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_claim_previous_owner_rewards() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        release_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            "CLUB001".to_string(),
        );

        let now = mock_env().block.time; // today

        let query_res = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match query_res {
            Ok(mut cod) => {
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                cod.start_timestamp = now.minus_seconds(22 * 24 * 60 * 60);
                CLUB_OWNERSHIP_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &cod);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(33u128),
            SET_AUTO_STAKE,
        );

        increase_reward_amount(
            deps.as_mut(),
            mock_env(),
            mintingContractInfo.clone(),
            "reward_from abc".to_string(),
            Uint128::from(1000000u128),
        );

        let mut staker_list1 = Vec::new();
        staker_list1.push("Staker001".to_string());
        staker_list1.push("Owner001".to_string());
        let club_name1 = "CLUB001".to_string();
        calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), staker_list1.clone(), club_name1, true, true);

        println!("releasing club");
        release_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            "CLUB001".to_string(),
        );

        let queryResAfterReleasing =
            query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterReleasing {
            Ok(cod) => {
                println!(
                    "before - owner:{:?}, reward {:?}",
                    cod.owner_address, cod.reward_amount
                );
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, true);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        println!(
            "pod:\n {:?}",
            query_all_previous_club_ownership_details(&mut deps.storage)
        );

        println!("buy a club with new owner");
        let owner2_info = mock_info("Owner002", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner2_info.clone(),
            "Owner002".to_string(),
            Some("Owner001".to_string()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let queryResAfterSellingByPrevOwner =
            query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterSellingByPrevOwner {
            Ok(cod) => {
                println!(
                    "after - owner:{:?}, reward {:?}",
                    cod.owner_address, cod.reward_amount
                );
                assert_eq!(cod.owner_address, "Owner002".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
        println!("checking previous owner details now");

        /*
        27 Feb 2022, Commenting this out - because the reward is now moved to stake
                     so there will be no previous owner details

        let queryPrevOwnerDetailsBeforeRewardClaim =
            query_club_previous_owner_details(&mut deps.storage, "Owner001".to_string());
        match queryPrevOwnerDetailsBeforeRewardClaim {
            Ok(pod) => {
                println!(
                    "before - owner:{:?}, reward {:?}",
                    pod.previous_owner_address, pod.reward_amount
                );
                assert_eq!(pod.previous_owner_address, "Owner001".to_string());
                assert_eq!(pod.reward_amount, Uint128::from(10000u128));
            }
            Err(e) => {
                println!("error parsing cpod header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
        */

        println!(
            "pod:\n {:?}",
            query_all_previous_club_ownership_details(&mut deps.storage)
        );

        claim_previous_owner_rewards(deps.as_mut(), owner1_info.clone(), "Owner001".to_string());
        let queryPrevOwnerDetailsAfterRewardClaim =
            query_club_previous_owner_details(&mut deps.storage, "Owner001".to_string())
                .unwrap_err();
        assert_eq!(
            queryPrevOwnerDetailsAfterRewardClaim,
            (StdError::GenericErr {
                msg: String::from("No previous ownership details found")
            })
        );

        println!(
            "pod:\n {:?}",
            query_all_previous_club_ownership_details(&mut deps.storage)
        );
    }

    #[test]
    fn test_claim_rewards_with_no_auto_stake() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            false, // NO AUTO STAKE
        );


        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(33000u128),
            false, // NO AUTO STAKE
        );

        increase_reward_amount(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            "reward_from abc".to_string(),
            Uint128::from(1000000u128),
        );

        let mut staker_list1 = Vec::new();
        staker_list1.push("Staker001".to_string());
        staker_list1.push("Owner001".to_string());
        let club_name1 = "CLUB001".to_string();
        calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), staker_list1.clone(), club_name1, true, true);

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let queryRes = query_all_stakes(&mut deps.storage, user_address_list);
        match queryRes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
                for stake in all_stakes {
                    let staker_address = stake.staker_address;
                    let reward_amount = stake.reward_amount;
                    let staked_amount = stake.staked_amount;
                    println!("staker : {:?} reward_amount : {:?} staked_amount : {:?}", staker_address.clone(), reward_amount, staked_amount);
                    if staker_address == "Staker001" {
                        assert_eq!(reward_amount, Uint128::from(970000u128));
                        assert_eq!(staked_amount, Uint128::from(33000u128));
                    }
                    if staker_address == "Owner001" {
                        assert_eq!(staked_amount, Uint128::from(0u128));
                        assert_eq!(reward_amount, Uint128::from(30000u128));
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_multiple_staking_on_club_by_same_address() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(33u128),
            SET_AUTO_STAKE,
        );
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            SET_AUTO_STAKE,
        );
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(42u128),
            SET_AUTO_STAKE,
        );

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let query_stakes = query_all_stakes(&mut deps.storage, user_address_list);
        match query_stakes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
                for stake in all_stakes {
                    if stake.staker_address == "Staker001".to_string() {
                        assert_eq!(stake.staked_amount, Uint128::from(86u128));
                    } else {
                        assert_eq!(stake.staked_amount, Uint128::from(0u128));
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_immediate_partial_withdrawals_from_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(99u128),
            SET_AUTO_STAKE,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(12u128),
            IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(13u128),
            IMMEDIATE_WITHDRAWAL,
        );

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let query_stakes = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match query_stakes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
                for stake in all_stakes {
                    if stake.staker_address == "Staker001".to_string() {
                        assert_eq!(stake.staked_amount, Uint128::from(63u128));
                    } else {
                        assert_eq!(stake.staked_amount, Uint128::from(0u128));
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage, user_address_list.clone());
        match queryBonds {
            Ok(all_bonds) => {
                assert_eq!(all_bonds.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_immediate_complete_withdrawals_from_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1Info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(99u128),
            SET_AUTO_STAKE,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(12u128),
            IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(13u128),
            IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(63u128),
            IMMEDIATE_WITHDRAWAL,
        );

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let query_stakes = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match query_stakes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage, user_address_list.clone());
        match queryBonds {
            Ok(all_bonds) => {
                assert_eq!(all_bonds.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_non_immediate_complete_withdrawals_from_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let admin_info = mock_info("admin11111", &[]);
        let minting_contract_info = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            admin_info.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(99u128),
            SET_AUTO_STAKE,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(12u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(13u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(63u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let query_stakes = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match query_stakes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage, user_address_list.clone());
        match queryBonds {
            Ok(all_bonds) => {
                assert_eq!(all_bonds.len(), 4);
                for bond in all_bonds {
                    if bond.bonded_amount != Uint128::from(11u128)
                        && bond.bonded_amount != Uint128::from(12u128)
                        && bond.bonded_amount != Uint128::from(13u128)
                        && bond.bonded_amount != Uint128::from(63u128)
                    {
                        println!("bond is {:?} ", bond);
                        assert_eq!(1, 2);
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
        let stakerInfo = mock_info("Staker002", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker002".to_string(),
            "CLUB001".to_string(),
            Uint128::from(99u128),
            SET_AUTO_STAKE,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker002".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );

        let queryBonds = query_club_bonding_details_for_user(
            &mut deps.storage,
            "CLUB001".to_string(),
            "Staker002".to_string(),
        );
        match queryBonds {
            Ok(all_bonds) => {
                assert_eq!(all_bonds.len(), 1);
                for bond in all_bonds {
                    if bond.bonded_amount != Uint128::from(11u128) {
                        println!("bond is {:?} ", bond);
                        assert_eq!(1, 2);
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_non_immediate_complete_withdrawals_from_club_with_scheduled_refunds() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "feecollector11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        );

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let stakerInfo = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(99u128),
            SET_AUTO_STAKE,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(12u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(13u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(63u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let query_stakes = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match query_stakes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let now = mock_env().block.time; // today

        let query_bonds = query_all_bonds(&mut deps.storage, user_address_list.clone());
        let club_name = "CLUB001".to_string();
        match query_bonds {
            Ok(all_bonds) => {
                let existing_bonds = all_bonds.clone();
                let mut updated_bonds = Vec::new();
                assert_eq!(existing_bonds.len(), 4);
                for user_addr in user_address_list.clone() {
                    for bond in existing_bonds.clone() {
                        let mut updated_bond = bond.clone();
                        if updated_bond.bonded_amount != Uint128::from(11u128)
                            && updated_bond.bonded_amount != Uint128::from(12u128)
                            && updated_bond.bonded_amount != Uint128::from(13u128)
                            && updated_bond.bonded_amount != Uint128::from(63u128)
                        {
                            println!("updated_bond is {:?} ", updated_bond);
                            assert_eq!(1, 2);
                        }
                        if updated_bond.bonded_amount == Uint128::from(63u128) {
                            updated_bond.bonding_start_timestamp = now.minus_seconds(8 * 24 * 60 * 60);
                        }
                        updated_bonds.push(updated_bond);
                    }
                    CLUB_BONDING_DETAILS.save(&mut deps.storage, (&club_name.clone(), &user_addr.clone()), &updated_bonds);
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        /*
                Commenting out as this is no longer used, 6 Apr 2022

                periodically_refund_stakeouts(deps.as_mut(), mock_env(), adminInfo);

                let queryBondsAfterPeriodicRefund = query_all_bonds(&mut deps.storage, user_address_list.clone());
                match queryBondsAfterPeriodicRefund {
                    Ok(all_bonds) => {
                        assert_eq!(all_bonds.len(), 3);
                        for bond in all_bonds {
                            if bond.bonded_amount != Uint128::from(11u128)
                                && bond.bonded_amount != Uint128::from(12u128)
                                && bond.bonded_amount != Uint128::from(13u128)
                            {
                                println!("bond is {:?} ", bond);
                                assert_eq!(1, 2);
                            }
                        }
                    }
                    Err(e) => {
                        println!("error parsing header: {:?}", e);
                        assert_eq!(1, 2);
                    }
                }
        */
    }

    #[test]
    fn test_non_immediate_partial_withdrawals_from_club() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 24 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1_info = mock_info("Owner001", &[coin(0, "uusd")]);
        let result = buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1_info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );
        println!("buy_a_club result = {:?}", result);
        let stakerInfo = mock_info("Staker001", &[coin(10, "uusd")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(99u128),
            SET_AUTO_STAKE,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(11u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(12u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        let result = withdraw_stake_from_a_club(
            deps.as_mut(),
            mock_env(),
            stakerInfo.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(13u128),
            NO_IMMEDIATE_WITHDRAWAL,
        );
        println!("result = {:?}", result);
        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Owner001".to_string());
        let query_stakes = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match query_stakes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 2);
                for stake in all_stakes {
                    if stake.staker_address == "Staker001".to_string() {
                        assert_eq!(stake.staked_amount, Uint128::from(63u128));
                    } else {
                        assert_eq!(stake.staked_amount, Uint128::from(0u128));
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage, user_address_list.clone());
        match queryBonds {
            Ok(all_bonds) => {
                assert_eq!(all_bonds.len(), 3);
                for bond in all_bonds {
                    if bond.bonded_amount != Uint128::from(11u128)
                        && bond.bonded_amount != Uint128::from(12u128)
                        && bond.bonded_amount != Uint128::from(13u128)
                    {
                        println!("bond is {:?} ", bond);
                        assert_eq!(1, 2);
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_distribute_rewards() {
        let mut deps = mock_dependencies(&[]);
        let now = mock_env().block.time; // today

        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            //club_reward_next_timestamp: now.minus_seconds(8 * 60 * 60),
            club_reward_next_timestamp: now.minus_seconds(1),
            reward_periodicity: 5 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        let adminInfo = mock_info("admin11111", &[]);
        let mintingContractInfo = mock_info("minting_admin11111", &[]);

        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
            .unwrap();

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner1Info.clone(),
            "Owner001".to_string(),
            Some(String::default()),
            "CLUB001".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );
        let owner2Info = mock_info("Owner002", &[coin(1000, "stake")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner2Info.clone(),
            "Owner002".to_string(),
            Some(String::default()),
            "CLUB002".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );
        let owner3Info = mock_info("Owner003", &[coin(1000, "stake")]);
        buy_a_club(
            deps.as_mut(),
            mock_env(),
            owner3Info.clone(),
            "Owner003".to_string(),
            Some(String::default()),
            "CLUB003".to_string(),
            Uint128::from(1000000u128),
            SET_AUTO_STAKE,
        );

        let staker1Info = mock_info("Staker001", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker1Info.clone(),
            "Staker001".to_string(),
            "CLUB001".to_string(),
            Uint128::from(330000u128),
            SET_AUTO_STAKE,
        );

        let staker2Info = mock_info("Staker002", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker2Info.clone(),
            "Staker002".to_string(),
            "CLUB001".to_string(),
            Uint128::from(110000u128),
            SET_AUTO_STAKE,
        );

        let staker3Info = mock_info("Staker003", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker3Info.clone(),
            "Staker003".to_string(),
            "CLUB002".to_string(),
            Uint128::from(420000u128),
            SET_AUTO_STAKE,
        );

        let staker4Info = mock_info("Staker004", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker4Info.clone(),
            "Staker004".to_string(),
            "CLUB002".to_string(),
            Uint128::from(100000u128),
            SET_AUTO_STAKE,
        );

        let staker5Info = mock_info("Staker005", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker5Info.clone(),
            "Staker005".to_string(),
            "CLUB003".to_string(),
            Uint128::from(820000u128),
            SET_AUTO_STAKE,
        );

        let staker6Info = mock_info("Staker006", &[coin(10, "stake")]);
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker6Info.clone(),
            "Staker006".to_string(),
            "CLUB003".to_string(),
            Uint128::from(50000u128),
            SET_AUTO_STAKE,
        );

        let mut user_address_list = Vec::new();
        user_address_list.push("Staker001".to_string());
        user_address_list.push("Staker002".to_string());
        user_address_list.push("Staker003".to_string());
        user_address_list.push("Staker004".to_string());
        user_address_list.push("Staker005".to_string());
        user_address_list.push("Staker006".to_string());
        user_address_list.push("Owner001".to_string());
        user_address_list.push("Owner002".to_string());
        user_address_list.push("Owner003".to_string());
        let queryRes0 = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match queryRes0 {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 9);
                println!("all stakes : {:?}", all_stakes);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        increase_reward_amount(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            "reward_from abc".to_string(),
            Uint128::from(1000000u128),
        );
        println!("stakes before distribution");
        let queryRes00 = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match queryRes00 {
            Ok(all_stakes) => {
                println!("all stakes : {:?}", all_stakes);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let mut queryReward = query_reward_amount(&mut deps.storage);
        println!("reward amount before distribution: {:?}", queryReward);
        let club_name1 = "CLUB001".to_string();
        calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), user_address_list.clone(), club_name1, true, false);
        println!("stakes after first distribution");
        let queryRes01 = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match queryRes01 {
            Ok(all_stakes) => {
                println!("all stakes : {:?}", all_stakes);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        queryReward = query_reward_amount(&mut deps.storage);
        println!("reward amount after first distribution: {:?}", queryReward);
        let club_name2 = "CLUB002".to_string();
        calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), user_address_list.clone(), club_name2, false, false);
        println!("stakes after second distribution");
        let queryRes01 = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match queryRes01 {
            Ok(all_stakes) => {
                println!("all stakes : {:?}", all_stakes);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        queryReward = query_reward_amount(&mut deps.storage);
        println!("reward amount after second distribution: {:?}", queryReward);
        let club_name3 = "CLUB003".to_string();
        calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), user_address_list.clone(), club_name3, false, true);

        queryReward = query_reward_amount(&mut deps.storage);
        println!("reward amount after third distribution: {:?}", queryReward);
        println!("stakes after third distribution");
        let queryRes = query_all_stakes(&mut deps.storage, user_address_list.clone());
        match queryRes {
            Ok(all_stakes) => {
                assert_eq!(all_stakes.len(), 9);
                println!("all stakes : {:?}", all_stakes);
                for stake in all_stakes {
                    let staker_address = stake.staker_address;
                    let staked_amount = stake.staked_amount;
                    println!("staker : {:?} staked_amount : {:?}", staker_address.clone(), staked_amount);
                    if staker_address == "Staker001" {
                        assert_eq!(staked_amount, Uint128::from(470655u128));
                    }
                    if staker_address == "Staker002" {
                        assert_eq!(staked_amount, Uint128::from(156885u128));
                    }
                    if staker_address == "Staker003" {
                        assert_eq!(staked_amount, Uint128::from(599016u128));
                    }
                    if staker_address == "Staker004" {
                        assert_eq!(staked_amount, Uint128::from(142622u128));
                    }
                    if staker_address == "Staker005" {
                        assert_eq!(staked_amount, Uint128::from(1348588u128));
                    }
                    if staker_address == "Staker006" {
                        assert_eq!(staked_amount, Uint128::from(82230u128));
                    }
                    if staker_address == "Owner001" {
                        assert_eq!(staked_amount, Uint128::from(10000u128));
                    }
                    if staker_address == "Owner002" {
                        assert_eq!(staked_amount, Uint128::from(10000u128));
                    }
                    if staker_address == "Owner003" {
                        assert_eq!(staked_amount, Uint128::from(10000u128));
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        // test another attempt to calculate and distribute at the same time

        increase_reward_amount(
            deps.as_mut(),
            mock_env(),
            mintingContractInfo.clone(),
            "reward_from abc".to_string(),
            Uint128::from(1000000u128),
        );

        /*
                let club_name2 = "CLUB001".to_string();
                let err = execute(
                    deps.as_mut(),
                    mock_env(),
                    adminInfo.clone(),
                    ExecuteMsg::CalculateAndDistributeRewards {},
                )
                .unwrap_err();

                assert_eq!(
                    err,
                    (ContractError::Std(StdError::GenericErr {
                        msg: String::from("Time for Reward not yet arrived")
                    }))
                );

        // test by preponing club_reward_next_timestamp
        let instantiate_msg = InstantiateMsg {
            admin_address: "admin11111".to_string(),
            minting_contract_address: "minting_admin11111".to_string(),
            astro_proxy_address: "astro_proxy_address1111".to_string(),
            club_fee_collector_wallet: "club_fee_collector_wallet11111".to_string(),
            club_reward_next_timestamp: now.minus_seconds(1 * 60 * 60),
            reward_periodicity: 5 * 60 * 60u64,
            club_price: Uint128::from(1000000u128),
            bonding_duration: 5 * 60u64,
            owner_release_locking_duration: 24 * 60 * 60u64,
            platform_fees_collector_wallet: "platform_fee_collector_wallet_1111".to_string(),
            platform_fees: Uint128::from(100u128),
            transaction_fees: Uint128::from(30u128),
            control_fees: Uint128::from(50u128),
            max_bonding_limit_per_user: 10u64,
        };
        instantiate(
            deps.as_mut(),
            mock_env(),
            adminInfo.clone(),
            instantiate_msg,
        )
        .unwrap();

        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker4Info.clone(),
            "Staker004".to_string(),
            "CLUB002".to_string(),
            Uint128::from(100000u128),
            SET_AUTO_STAKE,
        );
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker4Info.clone(),
            "Staker004".to_string(),
            "CLUB001".to_string(),
            Uint128::from(500000u128),
            SET_AUTO_STAKE,
        );
        stake_on_a_club(
            deps.as_mut(),
            mock_env(),
            staker4Info.clone(),
            "Staker004".to_string(),
            "CLUB003".to_string(),
            Uint128::from(126718u128),
            SET_AUTO_STAKE,
        );
        */

        increase_reward_amount(
            deps.as_mut(),
            mock_env(),
            mintingContractInfo.clone(),
            "reward_from def".to_string(),
            Uint128::from(1000000u128),
        );

        let queryReward = query_reward_amount(&mut deps.storage);
        println!("reward amount is {:?}", queryReward);
        let club_name1 = "CLUB001".to_string();
        let res1 = calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), user_address_list.clone(), 
            club_name1, true, false).unwrap_err();
        assert_eq!(res1, (ContractError::Std(StdError::GenericErr {msg: String::from("Time for Reward not yet arrived")})));
        println!("");
        println!("");
        let club_name2 = "CLUB002".to_string();
        let res2 = calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), user_address_list.clone(), 
            club_name2, false, false).unwrap_err();
        assert_eq!(res2, (ContractError::Std(StdError::GenericErr {msg: String::from("Time for Reward not yet arrived")})));
        println!("");
        println!("");
        let club_name3 = "CLUB003".to_string();
        let res3 = calculate_and_distribute_rewards(deps.as_mut(), mock_env(), adminInfo.clone(), user_address_list.clone(), 
            club_name3, false, true).unwrap_err();
        assert_eq!(res3, (ContractError::Std(StdError::GenericErr {msg: String::from("Time for Reward not yet arrived")})));
    }
}
