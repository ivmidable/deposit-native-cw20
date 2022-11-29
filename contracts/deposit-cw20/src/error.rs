use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("Invalid Owner")]
    InvalidOwner {},

    #[error("Invalid Coin")]
    InvalidCoin {},

    #[error("Stake duration has not passed")]
    StakeDurationNotPassed {},

    #[error("User does not have coins from this cw20 to withdraw")]
    NoCw20ToWithdraw {},

    #[error("Contract does not possess token_id from this cw721 to withdraw")]
    NoCw721ToWithdraw {},

}
