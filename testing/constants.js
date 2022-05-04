import {LCDClient, LocalTerra, MnemonicKey} from "@terra-money/terra.js";
// Contracts
export const MintingContractPath = "../artifacts/cw20_base.wasm"
export const ClubStakingContractPath = "../artifacts/club_staking.wasm"
export const GamingContractPath = "../artifacts/gaming_pool.wasm"

const contracts_folder_path = "../artifacts/"
export const debug = false //  Turn This False to Change to TestNet
// Sleep Time
export const sleep_time = (debug) ? 0 : 31000; // Sleep Time For All Test Processes

// Terra Clients
export const terraTestnetClient = new LCDClient({
    URL: "https://lcd.terra.dev",
    chainID: "columbus-5"
});

terraTestnetClient.chainID = "columbus-5";
export const localTerraClient = new LocalTerra();
localTerraClient.chainID = "localterra";
console.log(`Debug ${debug}`)
// export const terraClient =
export const terraClient = (debug) ? localTerraClient : terraTestnetClient; //  Deployer

console.log("terraClient.chainID = " + terraClient.chainID);


// Accounts
const mk1 = new MnemonicKey({mnemonic: "awesome festival volume rifle diagram suffer rhythm knock unlock reveal marine transfer lumber faint walnut love hover beach amazing robust oppose moon west will",});
export const mint_wallet = terraClient.wallet(mk1);

const mk2 = new MnemonicKey({mnemonic: "kiwi habit donor choice control fruit fame hamster trip aerobic juice lens lawn popular fossil taste venture furnace october income advice window opera helmet",});
export const treasury_wallet = terraClient.wallet(mk2);

const mk3 = new MnemonicKey({mnemonic: "job dilemma fold hurry solar strong solar priority lawsuit pass demise senior purpose useless outdoor jaguar identify enhance dirt vehicle fun nasty dragon still",});
export const liquidity_wallet = terraClient.wallet(mk3);

const mk4 = new MnemonicKey({mnemonic: "snap merit day trash key reopen stamp normal diagram vacant economy donate winner sister aerobic artist cheese bright palace athlete mind snack crawl bridge",});
export const marketing_wallet = terraClient.wallet(mk4);

const mk5 = new MnemonicKey({mnemonic: "element final maximum lake rain jewel never typical bunker detect gold earn fancy grace heart surge auction debris embody lazy edit worry expose soon"});
export const team_wallet = terraClient.wallet(mk5);

const mkNitin = new MnemonicKey({mnemonic: "garden celery myth discover isolate dilemma width sugar enemy grief case kingdom boring guess next huge indoor cargo crime letter useful essay gold view"});
export const nitin_wallet = terraClient.wallet(mkNitin);

const mkAjay = new MnemonicKey({mnemonic: "purse blur pitch skirt upset master relief feel pole enroll coffee change tooth live bunker federal work dry struggle little design eyebrow hope essence"});
export const ajay_wallet = terraClient.wallet(mkAjay);

const mkSameer = new MnemonicKey({mnemonic: "term salon nothing matrix flower click annual bomb anxiety glide castle okay payment degree umbrella clap cancel lock broom use ritual thrive price flavor"});
export const sameer_wallet = terraClient.wallet(mkSameer);

export const deployer = sameer_wallet


// Wallet Congfig
export const walletTest1 = (debug) ? terraClient.wallets.test1 : mint_wallet; //  Deployer
export const walletTest2 = (debug) ? terraClient.wallets.test2 : treasury_wallet;
export const walletTest3 = (debug) ? terraClient.wallets.test3 : liquidity_wallet;
export const walletTest4 = (debug) ? terraClient.wallets.test4 : marketing_wallet;
export const walletTest5 = (debug) ? terraClient.wallets.test5 : team_wallet;
export const walletTest6 = (debug) ? terraClient.wallets.test6 : nitin_wallet;
export const walletTest7 = (debug) ? terraClient.wallets.test10 : ajay_wallet;

export const mintInitMessage = {
    name: "Fury",
    symbol: "FURY",
    decimals: 6,
    initial_balances: [
        {address: "terra1ttjw6nscdmkrx3zhxqx3md37phldgwhggm345k", amount: "410000000000000"},
        {address: "terra1m46vy0jk9wck6r9mg2n8jnxw0y4g4xgl3csh9h", amount: "0"},
        {address: "terra1k20rlfj3ea47zjr2sp672qqscck5k5mf3uersq", amount: "0"},
        {address: "terra1wjq02nwcv6rq4zutq9rpsyq9k08rj30rhzgvt4", amount: "0"},
        {address: "terra19rgzfvlvq0f82zyy4k7whrur8x9wnpfcj5j9g7", amount: "0"},
        {address: "terra12g4sj6euv68kgx40k7mxu5xlm5sfat806umek7", amount: "0"},
        {address: deployer.key.accAddress, amount: "010000000000000"},
    ],
    mint: {
        minter: "terra1ttjw6nscdmkrx3zhxqx3md37phldgwhggm345k",
        cap: "420000000000000"
    },
    marketing: {
        project: "crypto11.me",
        description: "This token in meant to be used for playing gamesin crypto11 world",
        marketing: "terra1wjq02nwcv6rq4zutq9rpsyq9k08rj30rhzgvt4"
    },
}
