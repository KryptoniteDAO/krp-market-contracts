#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::state::{read_config, store_config, Config, read_new_owner, store_new_owner};

use cosmwasm_bignumber::Decimal256;
use moneymarket::distribution_model::{
    KptEmissionRateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
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
            emission_cap: msg.emission_cap,
            emission_floor: msg.emission_floor,
            increment_multiplier: msg.increment_multiplier,
            decrement_multiplier: msg.decrement_multiplier,
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
            emission_cap,
            emission_floor,
            increment_multiplier,
            decrement_multiplier,
        } => {
            update_config(
                deps,
                info,
                emission_cap,
                emission_floor,
                increment_multiplier,
                decrement_multiplier,
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
        return Err(ContractError::Unauthorized {});
    }

    config.owner = new_owner.new_owner_addr;
    store_config(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    emission_cap: Option<Decimal256>,
    emission_floor: Option<Decimal256>,
    increment_multiplier: Option<Decimal256>,
    decrement_multiplier: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(emission_cap) = emission_cap {
        config.emission_cap = emission_cap;
    }

    if let Some(emission_floor) = emission_floor {
        config.emission_floor = emission_floor
    }

    if let Some(increment_multiplier) = increment_multiplier {
        config.increment_multiplier = increment_multiplier;
    }

    if let Some(decrement_multiplier) = decrement_multiplier {
        config.decrement_multiplier = decrement_multiplier;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::KptEmissionRate {
            deposit_rate,
            target_deposit_rate,
            threshold_deposit_rate,
            current_emission_rate,
        } => to_binary(&query_kpt_emission_rate(
            deps,
            deposit_rate,
            target_deposit_rate,
            threshold_deposit_rate,
            current_emission_rate,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        emission_cap: state.emission_cap,
        emission_floor: state.emission_floor,
        increment_multiplier: state.increment_multiplier,
        decrement_multiplier: state.decrement_multiplier,
    };

    Ok(resp)
}

fn query_kpt_emission_rate(
    deps: Deps,
    deposit_rate: Decimal256,
    target_deposit_rate: Decimal256,
    threshold_deposit_rate: Decimal256,
    current_emission_rate: Decimal256,
) -> StdResult<KptEmissionRateResponse> {
    let config: Config = read_config(deps.storage)?;

    let half_dec = Decimal256::one() + Decimal256::one();
    let mid_rate = (threshold_deposit_rate + target_deposit_rate) / half_dec;
    let high_trigger = (mid_rate + target_deposit_rate) / half_dec;
    let low_trigger = (mid_rate + threshold_deposit_rate) / half_dec;

    let emission_rate = if deposit_rate < low_trigger {
        current_emission_rate * config.increment_multiplier
    } else if deposit_rate > high_trigger {
        current_emission_rate * config.decrement_multiplier
    } else {
        current_emission_rate
    };

    let emission_rate = if emission_rate > config.emission_cap {
        config.emission_cap
    } else if emission_rate < config.emission_floor {
        config.emission_floor
    } else {
        emission_rate
    };

    Ok(KptEmissionRateResponse { emission_rate })
}
