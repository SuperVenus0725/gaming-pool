import * as readline from 'node:readline';
import { promisify } from 'util';
import { ajay_wallet, ClubStakingContractPath, liquidity_wallet, marketing_wallet, 
    MintingContractPath, mintInitMessage, mint_wallet, nitin_wallet, sameer_wallet, 
    team_wallet, terraClient, treasury_wallet } from './constants.js';
//import { primeAccountsWithFunds } from "./primeCustomAccounts.js";
import { executeContract, getGasUsed, instantiateContract, queryContract, readArtifact, 
    storeCode, writeArtifact, queryContractInfo, queryTokenBalance, queryBankUusd, bankTransferFund} from './utils.js';

// define your own wallet for this script as gamer_wallet
// const mk1 = new MnemonicKey({mnemonic: "",});
// export const gamer_wallet = terraClient.wallet(mk1);

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});
const question = promisify(rl.question).bind(rl);

let sleep_time = 5000;
function sleep(time) {
    return new Promise((resolve) => setTimeout(resolve, time));
}

import * as chai from 'chai';

const assert = chai.assert;

let game_contract = "terra18wpjn83dayu4meu6wnn29khfkwdxs7ky8rt3c4"
let game_code_id = 10
let team_id_number = 1
let pool_id = "1"
let fury_contract_address = "terra18vd8fpwxzck93qlwghaj6arh4p7c5n896xzem5"
let proxy_contract_address = "terra19zpyd046u4swqpksr3n44cej4j8pg6ah2y6dcg"
let estimate_gas_uusd = 351263


const gaming_init = {

    "minting_contract_address": fury_contract_address, //  This should be a contract But We passed wallet so it wont raise error on addr validate
    "admin_address": sameer_wallet.key.accAddress,
    "platform_fee": "1",
    "transaction_fee": "1",
    "game_id": "Game001",
    "platform_fees_collector_wallet": sameer_wallet.key.accAddress,
    "astro_proxy_address": proxy_contract_address,
}

const createPoolTypeMP1 = {
    set_pool_type_params: {
        pool_type: "MP1",
        pool_fee: "1000",
        min_teams_for_pool: 10000,
        max_teams_for_pool: 10000,
        max_teams_for_gamer: 10000,
        wallet_percentages: [
            {
                wallet_address: "terra1uyuy363rzun7e6txdjdemqj9zua9j7wxl2lp0m",
                wallet_name: "rake_1",
                percentage: 100
            }
        ]
    }
}


terraClient.chainID = "localterra";
// export const terraClient = new LCDClient({
//   URL: 'https://bombay-lcd.terra.dev',
//   chainID: 'bombay-12',
// });

async function main() {
    try {
        let skipSetup = await question('Do Skip Setup Operations? (y/N) ');
        if (skipSetup === 'Y' || skipSetup === 'y') {
            
        } else {
            await test_create_and_query_game();
            await set_pool_headers_for_MP1_pool_type();
            await test_create_and_query_pool();
        }
        let ufury = await queryTokenBalance(fury_contract_address,nitin_wallet.key.accAddress)
        let uusd = await queryBankUusd(nitin_wallet.key.accAddress)
        console.log(`nitin fury: ${ufury} uusd ${uusd}`)
        console.log(`game fee uusd: ${createPoolTypeMP1.set_pool_type_params.pool_fee} `)
        let poolResponse = await queryContract(proxy_contract_address,{pool:{}})
        let rate = Number(poolResponse.assets[0].amount)/Number(poolResponse.assets[1].amount);
        let game_fee_ufury = Math.ceil(Number(createPoolTypeMP1.set_pool_type_params.pool_fee)*rate)
        let game_uusd_fee = Math.ceil(estimate_gas_uusd + Number(createPoolTypeMP1.set_pool_type_params.pool_fee*0.01301))
        console.log(`pool : ufury ${poolResponse.assets[0].amount} uusd ${poolResponse.assets[1].amount} rate ${rate}`)
        console.log(`iterations ufury use ${Math.floor(ufury/game_fee_ufury)} uusd use ${Math.floor(uusd/game_uusd_fee)}`)
        if (Math.ceil(ufury/game_fee_ufury) < createPoolTypeMP1.set_pool_type_params.max_teams_for_pool) {
            console.log(`more ufury reqd for game fees than avl in wallet`)
            let diff = Math.ceil(game_fee_ufury * createPoolTypeMP1.set_pool_type_params.max_teams_for_pool - ufury)
            console.log(`transferring ufury from mint_wallet : ${diff} to ${nitin_wallet.key.accAddress}`)
            await executeContract(mint_wallet, fury_contract_address, {transfer:{recipient:nitin_wallet.key.accAddress,amount:diff.toString()}})
        }   
        if (Math.ceil(uusd/estimate_gas_uusd) < createPoolTypeMP1.set_pool_type_params.max_teams_for_pool) {
            console.log(`more uusd reqd for gas and platform fee than avl in wallet`)
            let diff = Math.ceil(game_uusd_fee * createPoolTypeMP1.set_pool_type_params.max_teams_for_pool - uusd)
            console.log(`transferring uusd from mint_wallet : ${diff} to ${nitin_wallet.key.accAddress}`)
            await bankTransferFund(mint_wallet, nitin_wallet, 0, diff)
        }
        skipSetup = await question('Continue Operations? (y/N) ');
        let loopTry = 0
        if (skipSetup === 'Y' || skipSetup === 'y') {
            while (loopTry < createPoolTypeMP1.set_pool_type_params.max_teams_for_pool) {
                await test_game_pool_bid_submit(game_contract,pool_id,team_id_number.toString(),nitin_wallet)
                loopTry += 1
            }
        }
        
    } catch (error) {
        console.log(error);
    } finally {
        rl.close();
        console.log(`Total gas used = ${getGasUsed()}`);
    }
}

let test_create_and_query_game = async function () {
    console.log(`Uploading Gaming Contract ../artifacts/gaming_pool.wasm`)
    game_code_id = await storeCode(sameer_wallet, "../artifacts/gaming_pool.wasm")
    console.log(`Instantiating Gaming Contract ${game_code_id}`)
    let result = await instantiateContract(sameer_wallet, game_code_id, gaming_init)
    game_contract = result.logs[0].events[0].attributes.filter(element => element.key == 'contract_address').map(x => x.value);
    console.log(`Gaming Address: ${game_contract}`)
    game_contract = game_contract.toString()
    console.log("Query For Contract Details");
    let query_response = await queryContract(game_contract, {
        game_details: {}
    })
    assert.isTrue(gaming_init['game_id'] === query_response['game_id'])
    assert.isTrue(1 === query_response['game_status'])
    await console.log("Assert Success")
    //await sleep(sleep_time);
}

let test_create_and_query_pool = async function () {
    await console.log("Testing Create and Query Pool")
    await console.log("Create Pool")
    let createPoolMsg = { create_pool : { pool_type : "MP1" } }
    let response = await executeContract(sameer_wallet, game_contract, createPoolMsg)
    await console.log(`Pool Create TX : ${response.txhash}`)
    //await sleep(sleep_time)
    let new_pool_id = response.logs[0].events[1].attributes[1].value
    console.log(`New Pool ID  ${new_pool_id}`)
    pool_id = new_pool_id;
    response = await queryContract(game_contract, {
        pool_details: {
            pool_id: new_pool_id
        }
    })
    assert.isTrue(response['pool_id'] === new_pool_id)
    assert.isTrue(response['game_id'] === "Game001")
    assert.isTrue(response['pool_type'] === "MP1")
    assert.isTrue(response['current_teams_count'] === 0)
    assert.isTrue(response['rewards_distributed'] === false)
    await console.log("Assert Success")
}


const set_pool_headers_for_MP1_pool_type = async function () {
    await console.log("Testing Create Pool Type Header")
    const response = await executeContract(sameer_wallet, game_contract, createPoolTypeMP1)
    await console.log(response.txhash)
    //await sleep(sleep_time)
    await console.log("Assert Success")
}


let test_game_pool_bid_submit = async function (game_contract,pool_id,team_id,gamer_wallet) {
    console.log("Placing a bid")
    let pool_details = await queryContract(game_contract,
        {
            pool_details: {
                pool_id: pool_id
            }
        })
    let game_id = pool_details.game_id
    let current_teams_count = pool_details.current_teams_count
    let pool_type_details = await queryContract(game_contract,
        {
            pool_type_details: {
                pool_type: pool_details.pool_type
            }
        })
    let pool_type = pool_details.pool_type
    let pool_fee = pool_type_details.pool_fee
    let max_teams_for_pool = pool_type_details.max_teams_for_pool

    if (current_teams_count == max_teams_for_pool) {
        console.log(`pool full : ${current_teams_count} teams`)
        return
    }
    team_id_number = current_teams_count + 10
    team_id = team_id_number.toString()
    const gameInfo = await queryContractInfo(game_contract)
    let astro_proxy_address = gameInfo.init_msg.astro_proxy_address
    let fury_contract_address = gameInfo.init_msg.minting_contract_address
    let gaming_code_id = gameInfo.code_id
    let pool_fee_fury = await queryContract(astro_proxy_address,
        { get_fury_equivalent_to_ust : {
            ust_count: pool_fee
            }
        })

    // hardcoding without query to the contract
    let platform_fees = Math.ceil(pool_fee * 1301 / 100000);

    await console.log(`game ${game_contract} poolid ${pool_id} teamid ${team_id} gamer ${nitin_wallet.key.accAddress}`)
    let increaseAllowanceMsg = {
        increase_allowance: {
            spender: game_contract,
            amount: `${pool_fee_fury}`
        }
    };
    //await console.log(increaseAllowanceMsg);
    //await console.log("Increasing Allowance For the Gaming Pool Contract ")
    let incrAllowResp = await executeContract(gamer_wallet, fury_contract_address, increaseAllowanceMsg)
    //await sleep(sleep_time)
    await console.log(`Increase Allowance txhash : ${incrAllowResp.txhash}`)
    await console.log("Submitting Game Pool Bid")
    let response = await executeContract(gamer_wallet, game_contract, {
        game_pool_bid_submit_command: {
            gamer: gamer_wallet.key.accAddress,
            pool_type: pool_type,
            pool_id: pool_id,
            team_id: team_id,
            amount: `${pool_fee_fury}`
        }
    }, {'uusd': platform_fees})
    //await sleep(sleep_time);
    console.log(`Bid Submit txhash ${response.txhash}`);
    let ufury = await queryTokenBalance(fury_contract_address,nitin_wallet.key.accAddress)
    let uusd = await queryBankUusd(nitin_wallet.key.accAddress)
    console.log(`nitin balances fury: ${ufury} uusd ${uusd}`)
}

main ()