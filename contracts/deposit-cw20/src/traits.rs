use cosmwasm_std::{DepsMut, MessageInfo, Response, StdResult, Deps, CustomMsg, Env, Uint128};

use crate::msg::{DepositResponse, Cw20DepositResponse, Cw721DepositResponse};

pub trait Deposit<C>: DepositExecute<C> + DepositQuery
where
    C: CustomMsg,
{
}

pub trait DepositExecute<C>
where
    C: CustomMsg,
{
    type Err: ToString;
    fn execute_deposit(&self, deps: DepsMut, info: MessageInfo) -> Result<Response<C>, Self::Err>;
    fn execute_withdraw(&self, deps: DepsMut, info: MessageInfo, amount:u128, denom:String) -> Result<Response<C>, Self::Err>;
    fn execute_cw20_deposit(&self, deps: DepsMut, env:Env, info: MessageInfo, owner:String, amount:Uint128) -> Result<Response<C>, Self::Err>;
    fn execute_cw20_withdraw(&self, deps: DepsMut, env: Env, info: MessageInfo, contract:String, amount: Uint128) -> Result<Response<C>, Self::Err>;
    fn execute_cw721_deposit(&self, deps: DepsMut, env:Env, info: MessageInfo, owner:String, token_id:String) -> Result<Response<C>, Self::Err>;
    fn execute_cw721_withdraw(&self, deps: DepsMut, env:Env, info: MessageInfo, contract:String, token_id: String) -> Result<Response<C>, Self::Err>;
}

pub trait DepositQuery {
    fn query_deposits(&self, deps: Deps, address: String) -> StdResult<DepositResponse>;
    fn query_cw20_deposits(&self, deps: Deps, address: String) -> StdResult<Cw20DepositResponse>;
    fn query_cw721_by_contract(&self, deps: Deps, contract_addr:String) -> StdResult<Cw721DepositResponse>;
    fn query_cw721_by_owner(&self, deps: Deps, address: String) -> StdResult<Cw721DepositResponse>;
}