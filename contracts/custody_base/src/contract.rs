#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult,
};

use crate::collateral::{
    deposit_collateral, liquidate_collateral, lock_collateral, query_borrower, query_borrowers,
    unlock_collateral, withdraw_collateral,
};
use crate::error::ContractError;
use crate::state::{
    read_config, read_new_owner, store_config, store_new_owner, Config, NewOwnerAddr,
};

use cw20::Cw20ReceiveMsg;
use moneymarket::common::optional_addr_validate;
use moneymarket::custody_base::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

pub const CLAIM_REWARDS_OPERATION: u64 = 1u64;
pub const SWAP_TO_STABLE_OPERATION: u64 = 2u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_canonicalize(&msg.owner)?,
        overseer_contract: deps.api.addr_canonicalize(&msg.overseer_contract)?,
        collateral_token: deps.api.addr_canonicalize(&msg.collateral_token)?,
        market_contract: deps.api.addr_canonicalize(&msg.market_contract)?,
        reward_contract: deps.api.addr_canonicalize(&msg.reward_contract)?,
        liquidation_contract: deps.api.addr_canonicalize(&msg.liquidation_contract)?,
        stable_denom: msg.stable_denom,
        basset_info: msg.basset_info,
    };

    store_config(deps.storage, &config)?;

    store_new_owner(deps.storage, &{
        NewOwnerAddr {
            new_owner_addr: config.owner.clone(),
        }
    })?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::UpdateConfig {
            liquidation_contract,
        } => {
            let api = deps.api;
            update_config(
                deps,
                info,
                optional_addr_validate(api, liquidation_contract)?,
            )
        }
        ExecuteMsg::SetOwner { new_owner_addr } => {
            let api = deps.api;
            set_new_owner(deps, info, api.addr_validate(&new_owner_addr)?)
        }
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info),
        ExecuteMsg::LockCollateral { borrower, amount } => {
            let borrower_addr = deps.api.addr_validate(&borrower)?;
            lock_collateral(deps, info, borrower_addr, amount)
        }
        ExecuteMsg::UnlockCollateral { borrower, amount } => {
            let borrower_addr = deps.api.addr_validate(&borrower)?;
            unlock_collateral(deps, info, borrower_addr, amount)
        }
        ExecuteMsg::DistributeRewards {} => Ok(Response::new()),
        ExecuteMsg::WithdrawCollateral { borrower, amount } => {
            withdraw_collateral(deps, info, borrower, amount)
        }
        ExecuteMsg::LiquidateCollateral {
            liquidator,
            borrower,
            amount,
        } => {
            let liquidator_addr = deps.api.addr_validate(&liquidator)?;
            let borrower_addr = deps.api.addr_validate(&borrower)?;
            liquidate_collateral(deps, info, liquidator_addr, borrower_addr, amount)
        }
        ExecuteMsg::UpdateSwapContract { swap_contract: _ } => Ok(Response::new()),
        ExecuteMsg::UpdateSwapDenom {
            swap_denom: _,
            is_add: _,
        } => Ok(Response::new()),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender;
    match from_json(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCollateral {}) => {
            // only asset contract can execute this message
            let config: Config = read_config(deps.storage)?;
            if deps.api.addr_canonicalize(contract_addr.as_str())? != config.collateral_token {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "receive cw20 and depoist collateral Unauthorized {}",
                    contract_addr.to_string()
                ))));
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            deposit_collateral(deps, cw20_sender_addr, cw20_msg.amount.into())
        }
        _ => Err(ContractError::MissingDepositCollateralHook {}),
    }
}

pub fn set_new_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner_addr: Addr,
) -> Result<Response, ContractError> {
    let config = read_config(deps.as_ref().storage)?;
    let mut new_owner = read_new_owner(deps.as_ref().storage)?;
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    if sender_raw != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    new_owner.new_owner_addr = deps.api.addr_canonicalize(&new_owner_addr.to_string())?;
    store_new_owner(deps.storage, &new_owner)?;

    Ok(Response::default())
}

pub fn accept_ownership(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let new_owner = read_new_owner(deps.as_ref().storage)?;
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    let mut config = read_config(deps.as_ref().storage)?;
    if sender_raw != new_owner.new_owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    config.owner = new_owner.new_owner_addr;
    store_config(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    liquidation_contract: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(liquidation_contract) = liquidation_contract {
        config.liquidation_contract = deps.api.addr_canonicalize(liquidation_contract.as_str())?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Borrower { address } => {
            let addr = deps.api.addr_validate(&address)?;
            to_json_binary(&query_borrower(deps, addr)?)
        }
        QueryMsg::Borrowers { start_after, limit } => to_json_binary(&query_borrowers(
            deps,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        collateral_token: deps
            .api
            .addr_humanize(&config.collateral_token)?
            .to_string(),
        overseer_contract: deps
            .api
            .addr_humanize(&config.overseer_contract)?
            .to_string(),
        market_contract: deps.api.addr_humanize(&config.market_contract)?.to_string(),
        reward_contract: deps.api.addr_humanize(&config.reward_contract)?.to_string(),
        liquidation_contract: deps
            .api
            .addr_humanize(&config.liquidation_contract)?
            .to_string(),
        stable_denom: config.stable_denom,
        basset_info: config.basset_info,
        swap_contract: None,
        swap_denoms: None,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
