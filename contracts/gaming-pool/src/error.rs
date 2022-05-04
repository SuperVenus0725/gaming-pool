use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {
        invoker: String
    },

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("No allowance for this account")]
    NoAllowance {},

    // #[error("No vesting details for this account")]
    // NoVestingDetails (String),

    #[error("Minting cannot exceed the cap")]
    CannotExceedCap {},

    #[error("Logo binary data exceeds 5KB limit")]
    LogoTooBig {},

    #[error("Invalid xml preamble for SVG")]
    InvalidXmlPreamble {},

    #[error("Invalid png header")]
    InvalidPngHeader {},

    #[error("Insufficient fees in UST sent")]
    InsufficientFeesUst {},

    #[error("Number Of Coins Sent Is Invalid")]
    InvalidNumberOfCoinsSent {},

    #[error("Refund already claimed")]
    RefundAlreadyClaimed {},
    #[error("Reward already claimed")]
    RewardAlreadyClaimed {},
    #[error("Error Calcualting Plarform fee")]
    ErrorCalculatingPlatformFee {},

    #[error("Error Processing Batch For Reward Distribute, Both reward and refund cannot be zero ")]
    ErrorProcessingBatch {},

    #[error("Invalid Reply ID ")]
    InvalidReplyId {},

    #[error("Invalid Ammount to swap")]
    InvalidSwap {
        total_collection_in_pool: Uint128,
        amount_to_swap: Uint128,
    },
    #[error("Value Mismatch ")]
    ValueMismatch {
        reward_in_fury: Uint128,
        reward_in_total: Uint128,
    },

    #[error("Swap Info Not Found for Pool ")]
    SwapInfoNotFound {},

    #[error("User Is Restricted..!")]
    UserIsRestricted {},

}
