#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::asserts::{assert_fees, assert_max_slot, assert_max_slot_premium, assert_safe_ratio};
use crate::bid::{activate_bids, claim_liquidations, execute_liquidation, retract_bid, submit_bid};
use crate::querier::query_collateral_whitelist_info;
use crate::query::{
    query_bid, query_bid_pool, query_bid_pools, query_bids_by_user, query_collateral_info,
    query_config, query_liquidation_amount,
};
use crate::state::{
    read_collateral_info, read_config, store_collateral_info, store_config, CollateralInfo, Config, read_new_owner, store_new_owner, NewOwnerAddr,
};

use crate::error::ContractError;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Addr,
};
use cw20::Cw20ReceiveMsg;
use moneymarket::liquidation_queue::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    assert_fees(msg.liquidator_fee + msg.bid_fee)?;
    assert_safe_ratio(msg.safe_ratio)?;

    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(&msg.owner)?,
            oracle_contract: deps.api.addr_canonicalize(&msg.oracle_contract)?,
            stable_denom: msg.stable_denom,
            safe_ratio: msg.safe_ratio,
            bid_fee: msg.bid_fee,
            liquidator_fee: msg.liquidator_fee,
            liquidation_threshold: msg.liquidation_threshold,
            price_timeframe: msg.price_timeframe,
            waiting_period: msg.waiting_period,
            overseer: deps.api.addr_canonicalize(&msg.overseer)?,
        },
    )?;

    store_new_owner(deps.storage, &{
        NewOwnerAddr {
            new_owner_addr: deps.api.addr_canonicalize(&msg.owner)?,
        }
    })?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            oracle_contract,
            safe_ratio,
            bid_fee,
            liquidator_fee,
            liquidation_threshold,
            price_timeframe,
            waiting_period,
            overseer,
        } => update_config(
            deps,
            info,
            oracle_contract,
            safe_ratio,
            bid_fee,
            liquidator_fee,
            liquidation_threshold,
            price_timeframe,
            waiting_period,
            overseer,
        ),
        ExecuteMsg::SetOwner { new_owner_addr } => {
            let api = deps.api;
            set_new_owner(deps, info, api.addr_validate(&new_owner_addr)?)
        }
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info),
        ExecuteMsg::WhitelistCollateral {
            collateral_token,
            bid_threshold,
            max_slot,
            premium_rate_per_slot,
        } => whitelist_collateral(
            deps,
            info,
            collateral_token,
            bid_threshold,
            max_slot,
            premium_rate_per_slot,
        ),
        ExecuteMsg::UpdateCollateralInfo {
            collateral_token,
            bid_threshold,
            max_slot,
        } => update_collateral_info(deps, info, collateral_token, bid_threshold, max_slot),
        ExecuteMsg::SubmitBid {
            collateral_token,
            premium_slot,
        } => submit_bid(deps, env, info, collateral_token, premium_slot),
        ExecuteMsg::ActivateBids {
            collateral_token,
            bids_idx,
        } => activate_bids(deps, env, info, collateral_token, bids_idx),
        ExecuteMsg::RetractBid { bid_idx, amount } => retract_bid(deps, env, info, bid_idx, amount),
        ExecuteMsg::ClaimLiquidations {
            collateral_token,
            bids_idx,
        } => claim_liquidations(deps, env, info, collateral_token, bids_idx),
        ExecuteMsg::ExecuteBid {
            liquidator,
            repay_address,
            fee_address,
            collateral_denom,
            amount,
        } => {
            let sender = deps
                .api
                .addr_canonicalize(&info.sender.as_str())?
                .to_string();
            let collateral_token = collateral_denom;
            execute_liquidation(
                deps,
                env,
                sender,
                liquidator,
                repay_address,
                fee_address,
                collateral_token,
                amount,
            )
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender.clone();
    match from_json(&cw20_msg.msg) {
        //contract liquidation_queue received collatera token
        Ok(Cw20HookMsg::ExecuteBid {
            liquidator,
            repay_address,
            fee_address,
        }) => {
            let collateral_token = contract_addr.to_string();
            let repay_address = repay_address.unwrap_or_else(|| cw20_msg.sender.clone());
            let fee_address = fee_address.unwrap_or_else(|| cw20_msg.sender.clone());
            execute_liquidation(
                deps,
                env,
                cw20_msg.sender,
                liquidator,
                repay_address,
                fee_address,
                collateral_token,
                cw20_msg.amount.into(),
            )
        }
        _ => Err(ContractError::Std(StdError::generic_err(
            "Nofound Cw20HookMsg",
        ))),
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

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    oracle_contract: Option<String>,
    safe_ratio: Option<Decimal256>,
    bid_fee: Option<Decimal256>,
    liquidator_fee: Option<Decimal256>,
    liquidation_threshold: Option<Uint256>,
    price_timeframe: Option<u64>,
    waiting_period: Option<u64>,
    overseer: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(oracle_contract) = oracle_contract {
        config.oracle_contract = deps.api.addr_canonicalize(&oracle_contract)?;
    }

    if let Some(safe_ratio) = safe_ratio {
        assert_safe_ratio(safe_ratio)?;
        config.safe_ratio = safe_ratio;
    }

    if let Some(bid_fee) = bid_fee {
        config.bid_fee = bid_fee;
    }

    if let Some(liquidator_fee) = liquidator_fee {
        
        config.liquidator_fee = liquidator_fee;
    }
    assert_fees(config.bid_fee + config.liquidator_fee)?;

    if let Some(liquidation_threshold) = liquidation_threshold {
        config.liquidation_threshold = liquidation_threshold;
    }

    if let Some(price_timeframe) = price_timeframe {
        config.price_timeframe = price_timeframe;
    }

    if let Some(waiting_period) = waiting_period {
        config.waiting_period = waiting_period;
    }

    if let Some(overseer) = overseer {
        config.overseer = deps.api.addr_canonicalize(&overseer)?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new())
}

pub fn whitelist_collateral(
    deps: DepsMut,
    info: MessageInfo,
    collateral_token: String,
    bid_threshold: Uint256,
    max_slot: u8,
    premium_rate_per_slot: Decimal256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // fail if the collateral is already whitelisted
    if read_collateral_info(deps.storage, &collateral_token_raw).is_ok() {
        return Err(ContractError::Std(StdError::generic_err(
            "Collateral is already whitelisted",
        )));
    }

    // check if the colalteral is whitelisted in overseer
    let overseer = deps.api.addr_humanize(&config.overseer)?;
    query_collateral_whitelist_info(&deps.querier, overseer.to_string(), collateral_token)
        .map_err(|_| {
            ContractError::Std(StdError::generic_err(
                "This collateral is not whitelisted in kryptonite overseer",
            ))
        })?;

    // assert max slot does not exceed cap and max premium rate does not exceed 1
    assert_max_slot(max_slot)?;
    assert_max_slot_premium(max_slot, premium_rate_per_slot)?;

    // save collateral info
    store_collateral_info(
        deps.storage,
        &collateral_token_raw,
        &CollateralInfo {
            collateral_token: collateral_token_raw.clone(),
            max_slot,
            bid_threshold,
            premium_rate_per_slot,
        },
    )?;

    Ok(Response::new().add_attribute("action", "whitelist_collateral"))
}

pub fn update_collateral_info(
    deps: DepsMut,
    info: MessageInfo,
    collateral_token: String,
    bid_threshold: Option<Uint256>,
    max_slot: Option<u8>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let collateral_token_raw = deps.api.addr_canonicalize(&collateral_token)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // update collateral info
    let mut collateral_info: CollateralInfo =
        read_collateral_info(deps.storage, &collateral_token_raw)?;

    if let Some(bid_threshold) = bid_threshold {
        collateral_info.bid_threshold = bid_threshold;
    }

    if let Some(max_slot) = max_slot {
        // assert max slot does not exceed cap and max premium rate does not exceed 1
        assert_max_slot(max_slot)?;
        assert_max_slot_premium(max_slot, collateral_info.premium_rate_per_slot)?;
        collateral_info.max_slot = max_slot;
    }

    // save collateral info
    store_collateral_info(deps.storage, &collateral_token_raw, &collateral_info)?;

    Ok(Response::new().add_attribute("action", "update_collateral_info"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::LiquidationAmount {
            borrow_amount,
            borrow_limit,
            collaterals,
            collateral_prices,
        } => to_json_binary(&query_liquidation_amount(
            deps,
            borrow_amount,
            borrow_limit,
            collaterals,
            collateral_prices,
        )?),
        QueryMsg::CollateralInfo { collateral_token } => {
            to_json_binary(&query_collateral_info(deps, collateral_token)?)
        }
        QueryMsg::Bid { bid_idx } => to_json_binary(&query_bid(deps, bid_idx)?),
        QueryMsg::BidsByUser {
            collateral_token,
            bidder,
            start_after,
            limit,
        } => to_json_binary(&query_bids_by_user(
            deps,
            collateral_token,
            bidder,
            start_after,
            limit,
        )?),
        QueryMsg::BidPool {
            collateral_token,
            bid_slot,
        } => to_json_binary(&query_bid_pool(deps, collateral_token, bid_slot)?),
        QueryMsg::BidPoolsByCollateral {
            collateral_token,
            start_after,
            limit,
        } => to_json_binary(&query_bid_pools(
            deps,
            collateral_token,
            start_after,
            limit,
        )?),
    }
}
