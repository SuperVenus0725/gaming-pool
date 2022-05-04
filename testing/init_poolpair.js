import {GamingContractPath, mint_wallet,} from './constants.js';
import {instantiateContract, storeCode} from "./utils.js";
import {
    auth_liquidity_provider,
    fury_contract_address,
    liquidity_wallet,
    pool_pair_address,
    treasury_wallet
} from "./const";
import {bonded_lp_reward_wallet} from "../test-staking/constants";


export function sleep(time) {
    return new Promise((resolve) => setTimeout(resolve, time));
}


let proxyInitMessage = {
    /// admin address for configuration activities
    admin_address: mint_wallet.key.accAddress,
    /// contract address of Fury token
    custom_token_address: fury_contract_address,

    /// discount_rate when fury and UST are both provided
    pair_discount_rate: 700,
    /// bonding period when fury and UST are both provided TODO 7*24*60*60
    pair_bonding_period_in_sec: 5 * 24 * 60 * 60,
    /// Fury tokens for balanced investment will be fetched from this wallet
    pair_fury_reward_wallet: liquidity_wallet,
    /// The LP tokens for all liquidity providers except
    /// authorised_liquidity_provider will be stored to this address
    /// The LPTokens for balanced investment are delivered to this wallet
    pair_lp_tokens_holder: liquidity_wallet,

    /// discount_rate when only UST are both provided
    native_discount_rate: 500,
    /// bonding period when only UST provided TODO 5*24*60*60
    native_bonding_period_in_sec: 7 * 24 * 60 * 60,
    /// Fury tokens for native(UST only) investment will be fetched from this wallet
    //TODO: Change to Bonded Rewards Wallet == (old name)community/LP incentives Wallet
    native_investment_reward_wallet: bonded_lp_reward_wallet,
    /// The native(UST only) investment will be stored into this wallet
    native_investment_receive_wallet: treasury_wallet,

    /// This address has the authority to pump in liquidity
    /// The LP tokens for this address will be returned to this address
    authorized_liquidity_provider: auth_liquidity_provider,
    ///Time in nano seconds since EPOC when the swapping will be enabled
    swap_opening_date: "16509147437286239",

    /// Pool pair contract address of astroport
    pool_pair_address: pool_pair_address,

    platform_fees_collector_wallet: mint_wallet.key.accAddress,
    ///Specified in percentage multiplied by 100, i.e. 100% = 10000 and 0.01% = 1
    platform_fees: "50",
    ///Specified in percentage multiplied by 100, i.e. 100% = 10000 and 0.01% = 1
    transaction_fees: "30",
    ///Specified in percentage multiplied by 100, i.e. 100% = 10000 and 0.01% = 1
    swap_fees: "0",
};
let new_code_id = await storeCode(mint_wallet, GamingContractPath);
await sleep(15000)
let response = await instantiateContract(mint_wallet, new_code_id, proxyInitMessage)
console.log(response)