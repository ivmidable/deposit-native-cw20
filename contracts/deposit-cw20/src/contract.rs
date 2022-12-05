#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, from_binary, to_binary, BankMsg, Binary, CustomMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, Response, StdResult, Uint128, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, Expiration};
use cw20_base;
use cw721::Cw721ReceiveMsg;
use cw_utils;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    Cw20DepositResponse, Cw20HookMsg, Cw721DepositResponse, Cw721HookMsg, DepositResponse,
    ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg,
};
use crate::state::{Cw20Deposits, Cw721Deposits, Deposit, Deposits};
use crate::traits::{DepositExecute, DepositQuery};

const CONTRACT_NAME: &str = "deposit-cw20-example";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

impl<'a, C> DepositExecute<C> for Deposit<'a, C>
where
    C: CustomMsg,
{
    type Err = ContractError;
    fn execute_deposit(
        &self,
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response<C>, ContractError> {
        let sender = info.sender.clone().into_string();

        let d_coins = info.funds[0].clone();

        //check to see if deposit exists
        match self
            .deposits
            .load(deps.storage, (&sender, d_coins.denom.as_str()))
        {
            Ok(mut deposit) => {
                //add coins to their account
                deposit.coins.amount += d_coins.amount;
                deposit.coins.amount = deposit.coins.amount.checked_add(d_coins.amount).unwrap();
                deposit.count = deposit.count.checked_add(1).unwrap();
                self.deposits
                    .save(deps.storage, (&sender, d_coins.denom.as_str()), &deposit)
                    .unwrap();
            }
            Err(_) => {
                //user does not exist, add them.
                let deposit = Deposits {
                    count: 1,
                    owner: info.sender,
                    coins: d_coins.clone(),
                };
                self.deposits
                    .save(deps.storage, (&sender, d_coins.denom.as_str()), &deposit)
                    .unwrap();
            }
        }
        Ok(Response::new()
            .add_attribute("execute", "deposit")
            .add_attribute("denom", d_coins.denom)
            .add_attribute("amount", d_coins.amount))
    }

    fn execute_withdraw(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        amount: u128,
        denom: String,
    ) -> Result<Response<C>, ContractError> {
        let sender = info.sender.clone().into_string();

        let mut deposit = self
            .deposits
            .load(deps.storage, (&sender, denom.as_str()))
            .unwrap();
        deposit.coins.amount = deposit
            .coins
            .amount
            .checked_sub(Uint128::from(amount))
            .unwrap();
        deposit.count = deposit.count.checked_sub(1).unwrap();
        self.deposits
            .save(deps.storage, (&sender, denom.as_str()), &deposit)
            .unwrap();

        let msg = BankMsg::Send {
            to_address: sender.clone(),
            amount: vec![coin(amount, denom.clone())],
        };

        Ok(Response::new()
            .add_attribute("execute", "withdraw")
            .add_attribute("denom", denom)
            .add_attribute("amount", amount.to_string())
            .add_message(msg))
    }

    fn execute_cw20_deposit(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        owner: String,
        amount: Uint128,
    ) -> Result<Response<C>, ContractError> {
        let cw20_contract_address = info.sender.clone().into_string();
        let expiration = Expiration::AtHeight(env.block.height + 20);
        match self
            .cw20_deposits
            .load(deps.storage, (&owner, &cw20_contract_address))
        {
            Ok(mut deposit) => {
                //add coins to their account
                deposit.amount = deposit.amount.checked_add(amount).unwrap();
                deposit.count = deposit.count.checked_add(1).unwrap();
                deposit.stake_time = expiration;
                self.cw20_deposits
                    .save(deps.storage, (&owner, &cw20_contract_address), &deposit)
                    .unwrap();
            }
            Err(_) => {
                //user does not exist, add them.
                let deposit = Cw20Deposits {
                    count: 1,
                    owner: owner.clone(),
                    contract: info.sender.into_string(),
                    amount,
                    stake_time: expiration,
                };
                self.cw20_deposits
                    .save(deps.storage, (&owner, &cw20_contract_address), &deposit)
                    .unwrap();
            }
        }

        self.total_cw20_deposits.update(
            deps.storage,
            env.block.height,
            |total| -> StdResult<u64> { Ok(total.unwrap_or_default().checked_add(1u64).unwrap()) },
        )?;

        Ok(Response::new()
            .add_attribute("execute", "cw20_deposit")
            .add_attribute("owner", owner)
            .add_attribute("contract", cw20_contract_address.to_string())
            .add_attribute("amount", amount.to_string()))
    }

    fn execute_cw20_withdraw(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        amount: Uint128,
    ) -> Result<Response<C>, ContractError> {
        let sender = info.sender.clone().into_string();
        match self.cw20_deposits.load(deps.storage, (&sender, &contract)) {
            Ok(mut deposit) => {
                if deposit.stake_time.is_expired(&env.block) == false {
                    return Err(ContractError::StakeDurationNotPassed {});
                }

                deposit.amount = deposit.amount.checked_sub(amount).unwrap();
                deposit.count = deposit.count.checked_sub(1).unwrap();
                self.cw20_deposits
                    .save(deps.storage, (&sender, &contract), &deposit)
                    .unwrap();

                let exe_msg = cw20_base::msg::ExecuteMsg::Transfer {
                    recipient: sender,
                    amount: Uint128::from(amount),
                };
                let msg = WasmMsg::Execute {
                    contract_addr: contract,
                    msg: to_binary(&exe_msg)?,
                    funds: vec![],
                };

                self.total_cw20_deposits.update(
                    deps.storage,
                    env.block.height,
                    |total| -> StdResult<u64> { Ok(total.unwrap_or_default().checked_sub(1u64).unwrap()) },
                )?;

                Ok(Response::new()
                    .add_attribute("execute", "withdraw")
                    .add_message(msg))
            }
            Err(_) => {
                return Err(ContractError::NoCw20ToWithdraw {});
            }
        }
    }

    fn execute_cw721_deposit(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        owner: String,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        let cw721_contract_address = info.sender.clone().into_string();

        let data = Cw721Deposits {
            owner: owner.clone(),
            contract: info.sender.into_string(),
            token_id: token_id.clone(),
        };
        self.cw721_deposits
            .save(
                deps.storage,
                (&cw721_contract_address, &token_id),
                &data,
                env.block.height,
            )
            .unwrap();

        Ok(Response::new()
            .add_attribute("execute", "cw721_deposit")
            .add_attribute("owner", owner)
            .add_attribute("contract", cw721_contract_address.to_string()))
    }

    fn execute_cw721_withdraw(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        let owner = info.sender.clone().into_string();

        let _deposit = self
            .cw721_deposits
            .load(deps.storage, (&contract, &token_id))
            .unwrap();

        self.cw721_deposits
            .remove(deps.storage, (&contract, &token_id), env.block.height)
            .unwrap();

        let exe_msg = nft::contract::ExecuteMsg::TransferNft {
            recipient: owner,
            token_id: token_id,
        };
        let msg = WasmMsg::Execute {
            contract_addr: contract,
            msg: to_binary(&exe_msg)?,
            funds: vec![],
        };

        Ok(Response::new()
            .add_attribute("execute", "cw721_withdraw")
            .add_message(msg))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = Deposit::<Empty>::default();
    match msg {
        ExecuteMsg::Deposit {} => contract.execute_deposit(deps, info),
        ExecuteMsg::Withdraw { amount, denom } => {
            contract.execute_withdraw(deps, info, amount, denom)
        }
        ExecuteMsg::Receive(cw20_msg) => receive_cw20(deps, env, info, &contract, cw20_msg),
        ExecuteMsg::ReceiveNft(cw721_msg) => receive_cw721(deps, env, info, &contract, cw721_msg),
        ExecuteMsg::WithdrawCw20 { address, amount } => {
            contract.execute_cw20_withdraw(deps, env, info, address, amount)
        }
        ExecuteMsg::WithdrawNft {
            contract_addr,
            token_id,
        } => contract.execute_cw721_withdraw(deps, env, info, contract_addr, token_id),
    }
}

impl<'a, C> DepositQuery for Deposit<'a, C>
where
    C: CustomMsg,
{
    fn query_deposits(&self, deps: Deps, address: String) -> StdResult<DepositResponse> {
        let res: StdResult<Vec<_>> = self
            .deposits
            .prefix(&address)
            .range(deps.storage, None, None, Order::Ascending)
            .collect();
        let deposits = res?;
        Ok(DepositResponse { deposits })
    }

    fn query_cw20_deposits(&self, deps: Deps, address: String) -> StdResult<Cw20DepositResponse> {
        let res: StdResult<Vec<_>> = self
            .cw20_deposits
            .prefix(&address)
            .range(deps.storage, None, None, Order::Ascending)
            .collect();
        let deposits = res?;
        Ok(Cw20DepositResponse { deposits })
    }

    fn query_cw721_by_contract(
        &self,
        deps: Deps,
        contract: String,
    ) -> StdResult<Cw721DepositResponse> {
        let res: StdResult<Vec<_>> = self
            .cw721_deposits
            .prefix(&contract)
            .range(deps.storage, None, None, Order::Ascending)
            .collect();
        let deposits = res?;
        Ok(Cw721DepositResponse { deposits })
    }

    fn query_cw721_by_owner(&self, deps: Deps, address: String) -> StdResult<Cw721DepositResponse> {
        let res: StdResult<Vec<_>> = self
            .cw721_deposits
            .idx
            .owner
            .prefix(address)
            .range(deps.storage, None, None, Order::Ascending)
            .collect();
        let deposits = res?;
        Ok(Cw721DepositResponse { deposits })
    }

    fn query_total_cw20_deposits_changelog(&self, deps: Deps) -> StdResult<Vec<(u64, Option<u64>)>>{
        let res: StdResult<Vec<_>> = self
            .total_cw20_deposits.changelog().range(deps.storage, None, None, Order::Ascending).collect();
        let changelog = res?;
        Ok( changelog.into_iter().map(|(k, v)| (k, v.old)).collect() )
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let contract = Deposit::<Empty>::default();
    match msg {
        QueryMsg::Deposits { address } => to_binary(&contract.query_deposits(deps, address)?),
        QueryMsg::Cw20Deposits { address } => {
            to_binary(&contract.query_cw20_deposits(deps, address)?)
        }
        QueryMsg::Cw721DepositsByContract {
            contract_addr,
        } => to_binary(&contract.query_cw721_by_contract(deps, contract_addr)?),
        QueryMsg::Cw721DepositsByOwner { address } => {
            to_binary(&contract.query_cw721_by_owner(deps, address)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    unimplemented!()
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: &Deposit<Empty>,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Deposit {}) => {
            contract.execute_cw20_deposit(deps, env, info, cw20_msg.sender, cw20_msg.amount)
        }
        _ => Err(ContractError::CustomError {
            val: "Invalid Cw20HookMsg".to_string(),
        }),
    }
}

pub fn receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: &Deposit<Empty>,
    cw721_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw721_msg.msg) {
        Ok(Cw721HookMsg::Deposit {}) => {
            contract.execute_cw721_deposit(deps, env, info, cw721_msg.sender, cw721_msg.token_id)
        }
        _ => Err(ContractError::CustomError {
            val: "Invalid Cw721HookMsg".to_string(),
        }),
    }
}
