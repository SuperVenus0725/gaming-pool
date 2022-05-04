import {
     mint_wallet,
     nitin_wallet,
     sameer_wallet,
} from './constants.js';
import {
    storeCode,
    migrateContract,
    executeContract,
    queryContract,
    queryContractInfo,
    queryCodeInfo,
    transferToken,
    get_server_epoch_seconds
} from "./utils.js";

let current_minting_address = "terra18vd8fpwxzck93qlwghaj6arh4p7c5n896xzem5"
let MintingContractPath = "../whitelisting_artifacts/cw20_base.wasm"
let club_staking_address = "terra1nc5qec6n7wnx3v29eu0mutahlq8lrer0w54y39"
let proxy_contract_address ="terra19zpyd046u4swqpksr3n44cej4j8pg6ah2y6dcg"

async function main() {
    try {
        let response = await alltests()
    } catch (e) {
        try {
            await console.log(`${JSON.stringify(e.response.headers)}`)
            await console.log(`${JSON.stringify(e.response.config.data.tx_bytes)}`)
            await console.log(`${JSON.stringify(e.response.data.message)}`)
        } catch (e1) {
            await console.log(e)
        }
    }

}

async function alltests() {
    let response
    let msg

    response =  await queryContractInfo(current_minting_address)
    await console.log(`Previous Contract Info: ${JSON.stringify(response.code_id)}`)

    response =  await queryCodeInfo(response.code_id)
    await console.log(`Previous Code Info: ${JSON.stringify(response)}`)

    let new_code_id = await storeCode(mint_wallet, MintingContractPath);
    await console.log(`Code Id: ${new_code_id.toString()}`)

    response = await migrateContract(mint_wallet, current_minting_address, new_code_id, {})
    await console.log(`Migrate xtn hash : ${response.txhash}`)
    await console.log(`Response Timestamp : "${response.timestamp}"`)

    let dateNow
    if (response.timestamp == "") {
        dateNow = await get_server_epoch_seconds()
        await console.log(`get_server_epoch_seconds: ${dateNow}`)
    } else {
        dateNow = new Date(response.timestamp).getTime() / 1000
    }


    response =  await queryContractInfo(current_minting_address)
    await console.log(`New Contract Info: ${JSON.stringify(response.code_id)}`)

    response =  await queryCodeInfo(response.code_id)
    await console.log(`New Code Info: ${JSON.stringify(response)}`)


    let delaySec = 300
    let restrictDate = (dateNow + delaySec) * 1000000000
    await console.log(`Restricted by sec ${delaySec}`)
    await console.log(`Time Now sec ${dateNow} Restricted upto ${restrictDate}`)


    response = await executeContract(mint_wallet, current_minting_address, 
                        { set_white_list_expiration_timestamp : { timestamp : restrictDate.toString()}})

    await console.log(`Set time xtn hash : ${response.txhash}`)

    response = await executeContract(mint_wallet, current_minting_address, 
                        {
                          restricted_wallet_list_update: {
                            add_list: [
                              nitin_wallet.key.accAddress
                            ],
                            remove_list: []
                          }
                        })
    await console.log(`Add Wallet txhash : ${response.txhash}`)

    response = await executeContract(mint_wallet, current_minting_address, 
                        {
                          restricted_contract_list_update: {
                            add_list: [
                                club_staking_address
                            ],
                            remove_list: []
                          }
                        })
    await console.log(`Add Contract txhash : ${response.txhash}`)


    await console.log(`QUERIES`)

    response = await queryContract(current_minting_address, {restricted_list_timestamp:{}})
    await console.log(`Check Timestamp ${JSON.stringify(response)}`)

    response = await queryContract(current_minting_address, {restricted_wallet_list:{}})
    await console.log(`Check Restricted Wallets ${JSON.stringify(response)}`)

    response = await queryContract(current_minting_address, {restricted_contract_list:{}})
    await console.log(`Check Restricted Contracts ${JSON.stringify(response)}`)

    response = await queryContract(current_minting_address, 
                                    {is_restricted_wallet:{address:nitin_wallet.key.accAddress}})
    await console.log(`Check Restricted Wallet ${JSON.stringify(response)}`)

    response = await queryContract(current_minting_address, 
                                    {is_restricted_wallet:{address:sameer_wallet.key.accAddress}})
    await console.log(`Check non-restricted Wallet ${JSON.stringify(response)}`)

    response = await queryContract(current_minting_address, 
                                    {is_restricted_contract:{address:club_staking_address}})
    await console.log(`Check Included Contract ${JSON.stringify(response)}`)

    response = await queryContract(current_minting_address, 
                                    {is_restricted_contract:{address:proxy_contract_address}})
    await console.log(`Check not-included Contract ${JSON.stringify(response)}`)

    await console.log(`TEST WHITELIST FEATURE`)

    msg = {transfer:{recipient:nitin_wallet.key.accAddress,amount:"100011"}}
    await console.log(`${sameer_wallet.key.accAddress}, ${current_minting_address} ${JSON.stringify(msg)}`)
    response = await executeContract(sameer_wallet, current_minting_address, msg)
    await console.log(`Unrestricted Wallet Transfer txhash : ${response.txhash}`)


    response = await executeContract(sameer_wallet, current_minting_address,
                                    {increase_allowance:{ spender:proxy_contract_address,amount:"100022"}})
    await console.log(`Unrestricted Wallet Increase Allowance txhash : ${response.txhash}`)

    //let sendFuryForStakeReward = jsonToBinary({"increase_reward_amount":{"reward_from":"minter"}})
    // echo -n '{"increase_reward_amount":{"reward_from":"minter"}}'|base64
    let sendFuryForStakeReward = 'eyJpbmNyZWFzZV9yZXdhcmRfYW1vdW50Ijp7InJld2FyZF9mcm9tIjoibWludGVyIn19'
    

    response = await executeContract(mint_wallet, current_minting_address,
                    {send:{contract:club_staking_address,amount:"100033",msg:sendFuryForStakeReward}})
    await console.log(`Unrestricted Wallet Send txhash : ${response.txhash}`)

    try {
        response = await executeContract(nitin_wallet, current_minting_address,
                                        {increase_allowance:{ spender:proxy_contract_address,amount:"100022"}})
        await console.log(`Error :: Restricted Transfer txhash : ${response.txhash}`)
    } catch (e) {
        await console.log(`Restricted Wallet Failure in increase_allowance as expected`)
        await console.log(`error message : ${JSON.stringify(e.response.data.message)}`)
    }

    try {
        response = await executeContract(nitin_wallet, current_minting_address,
                                        {transfer:{recipient:sameer_wallet.key.accAddress,amount:"100011"}})
        await console.log(`Error :: Restricted Transfer txhash : ${response.txhash}`)
    } catch (e) {
        await console.log(`Restricted Wallet Failure in transfer as expected`)
        await console.log(`error message : ${JSON.stringify(e.response.data.message)}`)
    }

    response = await executeContract(nitin_wallet, current_minting_address,
                    {increase_allowance:{ spender:club_staking_address,amount:"100022"}})
    await console.log(`Restricted Wallet Increase Allowance to Permitted Contract txhash : ${response.txhash}`)

    response = await executeContract(nitin_wallet, current_minting_address,
                    {send:{contract:club_staking_address,amount:"100033",msg:sendFuryForStakeReward}})
    await console.log(`Restricted Wallet Send to Permitted Contract txhash : ${response.txhash}`)

    await console.log("sleeping for delaySec, before testing Xtns after Restriction period elapsed")
    await new Promise(resolve => setTimeout(resolve, delaySec * 1000));

    try {
        response = await executeContract(nitin_wallet, current_minting_address,
                                        {increase_allowance:{ spender:proxy_contract_address,amount:"100022"}})
        await console.log(`Now successful - Restricted increase_allowance txhash : ${response.txhash}`)
    } catch (e) {
        await console.log(`Error :: Restricted Wallet Failure in increase_allowance `)
        await console.log(`error message : ${JSON.stringify(e.response.data.message)}`)
    }

    try {
        response = await executeContract(nitin_wallet, current_minting_address,
                                        {transfer:{recipient:sameer_wallet.key.accAddress,amount:"100011"}})
        await console.log(`Now successful - Restricted Transfer txhash : ${response.txhash}`)
    } catch (e) {
        await console.log(`Error :: Restricted Wallet Failure in Transfer `)
        await console.log(`error message : ${JSON.stringify(e.response.data.message)}`)
    }


}

main()