import {GamingContractPath, mint_wallet,} from './constants.js';
import {instantiateContract, storeCode} from "./utils.js";
import {fury_contract_address, proxy_contract_address} from "./const.js";


export function sleep(time) {
    return new Promise((resolve) => setTimeout(resolve, time));
}


let gaming_init = {
    "minting_contract_address": fury_contract_address,
    "admin_address": mint_wallet.key.accAddress,
    "platform_fee": "50",
    "transaction_fee": "30",
    "game_id": "Game001",
    "platform_fees_collector_wallet": mint_wallet.key.accAddress,
    "astro_proxy_address": proxy_contract_address,
}
let new_code_id = await storeCode(mint_wallet, GamingContractPath);
await sleep(15000)
let response = await instantiateContract(mint_wallet, new_code_id, gaming_init)
console.log(response)