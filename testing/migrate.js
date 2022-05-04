import {ClubStakingContractPath, MintingContractPath, terraClient} from './constants.js';
import {storeCode} from "./utils.js";
import {MnemonicKey} from "@terra-money/terra.js";


const admin = new MnemonicKey({mnemonic: "gadget bottom broom illegal magnet narrow giant sausage foil ugly remind about orchard pelican involve civil army bulk alone acoustic phone disagree opinion one"});
export const admin_wallet = terraClient.wallet(admin);
console.log(ClubStakingContractPath)
let new_code_id = await storeCode(admin_wallet, ClubStakingContractPath);
console.log(new_code_id)
