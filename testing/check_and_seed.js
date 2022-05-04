import {executeContract} from "./utils.js";

const {terraClient} = require("./constants.js");
const fury_contract_address = ""

function bankTransferFund(wallet_from, wallet_to, uluna_amount, uusd_amount) {
    console.log(`Funding ${wallet_to.key.accAddress}`);
    return new Promise(resolve => {
        // create a simple message that moves coin balances
        const send1 = new MsgSend(
            wallet_from.key.accAddress,
            wallet_to.key.accAddress,
            {uluna: uluna_amount, uusd: uusd_amount}
        );

        wallet_from
            .createAndSignTx({
                msgs: [send1],
                memo: 'Seeding Contract',
            })
            .then(tx => terraClient.tx.broadcast(tx))
            .then(result => {
                console.log(result.txhash);
                resolve(result.txhash);
            });
    })
}

async function transferFuryTokens(walletFrom, toAddress, amount) {
    let transferFuryToTreasuryMsg = {
        transfer: {
            recipient: toAddress,
            amount: amount
        }
    };
    console.log(`Transfer Message = ${JSON.stringify(transferFuryToTreasuryMsg)}`);
    let response = await executeContract(walletFrom, fury_contract_address, transferFuryToTreasuryMsg);
    console.log(`Response - ${response['txhash']}`);
}

async function check_and_seed(minimum_uusd_balance, minimum_fury_balance, contract_address) {
    //This will load the UST balance
    const [balance] = await terraClient.bank.balance(contract_address);
    console.log(balance.toData());
    let ust_balance = 0
    // This will load the FURY balance
    const response = await terraClient.wasm.contractQuery(fury_contract_address, {balance: {address: contract_address}});
    let fury_balance = response['balance']
    if (ust_balance < minimum_uusd_balance) {
        await bankTransferFund(walletTest1, mint_wallet, 500000000, 1000000000)
    }

}