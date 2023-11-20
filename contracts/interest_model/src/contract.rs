use crate::error::ContractError;
use crate::state::{read_config, store_config, Config, read_new_owner, store_new_owner};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_bignumber::Decimal256;
use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use moneymarket::interest_model::{
    BorrowRateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(&msg.owner)?,
            base_rate: msg.base_rate,
            interest_multiplier: msg.interest_multiplier,
        },
    )?;

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
        ExecuteMsg::UpdateConfig {
            base_rate,
            interest_multiplier,
        } => {
            update_config(
                deps,
                info,
                base_rate,
                interest_multiplier,
            )
        }
        ExecuteMsg::SetOwner { new_owner_addr } => {
            let api = deps.api;
            set_new_owner(deps, info, api.addr_validate(&new_owner_addr)?)
        }
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info),
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
        return Err(ContractError::Unauthorized{});
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
        return Err(ContractError::Unauthorized{});
    }

    config.owner = new_owner.new_owner_addr;
    store_config(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    base_rate: Option<Decimal256>,
    interest_multiplier: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(base_rate) = base_rate {
        config.base_rate = base_rate;
    }

    if let Some(interest_multiplier) = interest_multiplier {
        config.interest_multiplier = interest_multiplier;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::BorrowRate {
            market_balance,
            total_liabilities,
            total_reserves,
        } => to_binary(&query_borrow_rate(
            deps,
            market_balance,
            total_liabilities,
            total_reserves,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        base_rate: state.base_rate,
        interest_multiplier: state.interest_multiplier,
    };

    Ok(resp)
}

fn query_borrow_rate(
    deps: Deps,
    market_balance: Uint256,
    total_liabilities: Decimal256,
    total_reserves: Decimal256,
) -> StdResult<BorrowRateResponse> {
    let config: Config = read_config(deps.storage)?;

    // ignore decimal parts
    let total_value_in_market =
        Decimal256::from_uint256(market_balance) + total_liabilities - total_reserves;

    let utilization_ratio = if total_value_in_market.is_zero() {
        Decimal256::zero()
    } else {
        total_liabilities / total_value_in_market
    };

    Ok(BorrowRateResponse {
        rate: utilization_ratio * config.interest_multiplier + config.base_rate,
    })
}
