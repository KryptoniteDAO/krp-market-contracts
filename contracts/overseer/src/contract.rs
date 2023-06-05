
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, Response, StdResult, Uint128, WasmMsg, SubMsg, Attribute,
};
use std::cmp::{max, min};

use crate::collateral::{
    liquidate_collateral, lock_collateral, query_all_collaterals, query_borrow_limit,
    query_collaterals, repay_stable_from_yield_reserve, unlock_collateral,
};
use crate::error::ContractError;
use crate::querier::query_market_epoch_state;
use crate::state::{
    read_all_marketlist, read_config, read_market_config, read_market_dynrate_config,
    read_market_dynrate_state, read_market_epoch_state, read_marketlist_elem, read_whitelist,
    read_whitelist_elem, store_config, store_market_config, store_market_dynrate_config,
    store_market_dynrate_state, store_market_epoch_state, store_marketlist_elem,
    store_whitelist_elem, Config, DynrateConfig, DynrateState, EpochState, MarketConfig,
    MarketlistElem, WhitelistElem,
};

use cosmwasm_bignumber::{Decimal256, Uint256};
use moneymarket::common::optional_addr_validate;
use moneymarket::market::EpochStateResponse;
use moneymarket::market::ExecuteMsg as MarketExecuteMsg;
use moneymarket::overseer::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MarketlistResponse, MarketlistResponseElem,
    MigrateMsg, QueryMsg, WhitelistResponse, WhitelistResponseElem,
};

use moneymarket::custody::ExecuteMsg as CustodyExecuteMsg;
use moneymarket::querier::{deduct_tax, query_balance};

pub const BLOCKS_PER_YEAR: u128 = 4656810;

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
            owner_addr: deps.api.addr_canonicalize(&msg.owner_addr)?,
            oracle_contract: deps.api.addr_canonicalize(&msg.oracle_contract)?,
            liquidation_contract: deps.api.addr_canonicalize(&msg.liquidation_contract)?,
            collector_contract: deps.api.addr_canonicalize(&msg.collector_contract)?,
            epoch_period: msg.epoch_period,
            krp_purchase_factor: msg.krp_purchase_factor,
            price_timeframe: msg.price_timeframe,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner_addr,
            oracle_contract,
            liquidation_contract,
            krp_purchase_factor,
            epoch_period,
            price_timeframe,
        } => {
            let api = deps.api;
            update_config(
                deps,
                info,
                optional_addr_validate(api, owner_addr)?,
                optional_addr_validate(api, oracle_contract)?,
                optional_addr_validate(api, liquidation_contract)?,
                krp_purchase_factor,
                epoch_period,
                price_timeframe,
            )
        }
        ExecuteMsg::Whitelist {
            name,
            symbol,
            collateral_token,
            custody_contract,
            max_ltv,
        } => {
            let api = deps.api;
            register_whitelist(
                deps,
                info,
                name,
                symbol,
                api.addr_validate(&collateral_token)?,
                api.addr_validate(&custody_contract)?,
                max_ltv,
            )
        }
        ExecuteMsg::UpdateWhitelist {
            collateral_token,
            custody_contract,
            max_ltv,
        } => {
            let api = deps.api;
            update_whitelist(
                deps,
                info,
                api.addr_validate(&collateral_token)?,
                optional_addr_validate(api, custody_contract)?,
                max_ltv,
            )
        }
        ExecuteMsg::ExecuteEpochOperations {} => execute_epoch_operations(deps, env),
        // ExecuteMsg::UpdateEpochState {
        //     market_contract,
        //     interest_buffer,
        //     distributed_interest,
        // } => {
        //     let api = deps.api;
        //     update_market_epoch_state(
        //         deps,
        //         env,
        //         info,
        //         &api.addr_canonicalize(market_contract.as_str())?,
        //         interest_buffer,
        //         distributed_interest,
        //     )
        // }
        ExecuteMsg::LockCollateral {
            borrower,
            collaterals,
        } => lock_collateral(deps, info, borrower, collaterals),
        ExecuteMsg::UnlockCollateral { collaterals } => {
            unlock_collateral(deps, env, info, collaterals)
        }
        ExecuteMsg::LiquidateCollateral { borrower } => {
            let api = deps.api;
            liquidate_collateral(deps, env, info, api.addr_validate(&borrower)?)
        }
        ExecuteMsg::FundReserve {} => fund_reserve(deps, info),
        ExecuteMsg::RepayStableFromYieldReserve {
            market_contract,
            borrower,
        } => {
            let api = deps.api;
            repay_stable_from_yield_reserve(
                deps,
                env,
                info,
                api.addr_validate(&market_contract)?,
                api.addr_validate(&borrower)?,
            )
        }
        ExecuteMsg::RegisterMarket {
            market_contract,
            stable_denom,
            stable_name,
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
            dyn_rate_epoch,
            dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation,
            dyn_rate_min,
            dyn_rate_max,
        } => {
            let api = deps.api;
            register_market(
                deps,
                info,
                env,
                api.addr_validate(&market_contract)?,
                stable_denom,
                stable_name,
                threshold_deposit_rate,
                target_deposit_rate,
                buffer_distribution_factor,
                dyn_rate_epoch,
                dyn_rate_maxchange,
                dyn_rate_yr_increase_expectation,
                dyn_rate_min,
                dyn_rate_max,
            )
        }

        ExecuteMsg::UpdateMarketConfig {
            market_contract,
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
        } => update_market_config(
            deps,
            info,
            market_contract,
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
        ),

        ExecuteMsg::UpdateMarketDynrateConfig {
            market_contract,
            dyn_rate_epoch,
            dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation,
            dyn_rate_min,
            dyn_rate_max,
        } => update_market_dyrate_config(
            deps,
            info,
            market_contract,
            dyn_rate_epoch,
            dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation,
            dyn_rate_min,
            dyn_rate_max,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner_addr: Option<Addr>,
    oracle_contract: Option<Addr>,
    liquidation_contract: Option<Addr>,
    krp_purchase_factor: Option<Decimal256>,
    epoch_period: Option<u64>,
    price_timeframe: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner_addr) = owner_addr {
        config.owner_addr = deps.api.addr_canonicalize(&owner_addr.to_string())?;
    }

    if let Some(oracle_contract) = oracle_contract {
        config.oracle_contract = deps.api.addr_canonicalize(&oracle_contract.to_string())?;
    }

    if let Some(liquidation_contract) = liquidation_contract {
        config.liquidation_contract = deps
            .api
            .addr_canonicalize(&liquidation_contract.to_string())?;
    }

    if let Some(krp_purchase_factor) = krp_purchase_factor {
        config.krp_purchase_factor = krp_purchase_factor;
    }

    if let Some(epoch_period) = epoch_period {
        config.epoch_period = epoch_period;
    }

    if let Some(price_timeframe) = price_timeframe {
        config.price_timeframe = price_timeframe;
    }
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn register_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    symbol: String,
    collateral_token: Addr,
    custody_contract: Addr,
    max_ltv: Decimal256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    if read_whitelist_elem(deps.storage, &collateral_token_raw).is_ok() {
        return Err(ContractError::TokenAlreadyRegistered {});
    }

    store_whitelist_elem(
        deps.storage,
        &collateral_token_raw,
        &WhitelistElem {
            name: name.to_string(),
            symbol: symbol.to_string(),
            custody_contract: deps.api.addr_canonicalize(custody_contract.as_str())?,
            max_ltv,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_whitelist"),
        attr("name", name),
        attr("symbol", symbol),
        attr("collateral_token", collateral_token),
        attr("custody_contract", custody_contract),
        attr("LTV", max_ltv.to_string()),
    ]))
}

pub fn update_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    collateral_token: Addr,
    custody_contract: Option<Addr>,
    max_ltv: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let collateral_token_raw = deps.api.addr_canonicalize(collateral_token.as_str())?;
    let mut whitelist_elem: WhitelistElem =
        read_whitelist_elem(deps.storage, &collateral_token_raw)?;

    if let Some(custody_contract) = custody_contract {
        whitelist_elem.custody_contract = deps.api.addr_canonicalize(custody_contract.as_str())?;
    }

    if let Some(max_ltv) = max_ltv {
        whitelist_elem.max_ltv = max_ltv;
    }

    store_whitelist_elem(deps.storage, &collateral_token_raw, &whitelist_elem)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_whitelist"),
        attr("collateral_token", collateral_token),
        attr(
            "custody_contract",
            deps.api.addr_humanize(&whitelist_elem.custody_contract)?,
        ),
        attr("LTV", whitelist_elem.max_ltv.to_string()),
    ]))
}

pub fn register_market(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    market_contract: Addr,
    stable_denom: String,
    stable_name: String,
    threshold_deposit_rate: Decimal256,
    target_deposit_rate: Decimal256,
    buffer_distribution_factor: Decimal256,
    dyn_rate_epoch: u64,
    dyn_rate_maxchange: Decimal256,
    dyn_rate_yr_increase_expectation: Decimal256,
    dyn_rate_min: Decimal256,
    dyn_rate_max: Decimal256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let market_contract_raw = deps.api.addr_canonicalize(market_contract.as_str())?;
    if read_marketlist_elem(deps.storage, &market_contract_raw).is_ok() {
        return Err(ContractError::MarketAreadyRegister {});
    }

    store_marketlist_elem(
        deps.storage,
        &market_contract_raw,
        &MarketlistElem {
            market_contract: deps.api.addr_canonicalize(market_contract.as_str())?,
            stable_denom: stable_denom.to_string(),
            stable_name: stable_name.to_string(),
        },
    )?;

    store_market_config(
        deps.storage,
        &market_contract_raw,
        &MarketConfig {
            threshold_deposit_rate,
            target_deposit_rate,
            buffer_distribution_factor,
        },
    )?;

    store_market_dynrate_config(
        deps.storage,
        &market_contract_raw,
        &DynrateConfig {
            dyn_rate_epoch,
            dyn_rate_maxchange,
            dyn_rate_yr_increase_expectation,
            dyn_rate_min,
            dyn_rate_max,
        },
    )?;

    store_market_epoch_state(
        deps.storage,
        &market_contract_raw,
        &EpochState {
            deposit_rate: Decimal256::zero(),
            prev_atoken_supply: Uint256::zero(),
            prev_interest_buffer: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
            last_executed_height: env.block.height,
        },
    )?;

    store_market_dynrate_state(
        deps.storage,
        &market_contract_raw,
        &DynrateState {
            last_executed_height: env.block.height,
            prev_yield_reserve: Decimal256::zero(),
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_market"),
        attr("stabel_name", stable_name),
        attr("stable_denom", stable_denom),
        attr("market_contract", market_contract.to_string()),
    ]))
}

pub fn update_market_config(
    deps: DepsMut,
    info: MessageInfo,
    market_contract: String,
    threshold_deposit_rate: Option<Decimal256>,
    target_deposit_rate: Option<Decimal256>,
    buffer_distribution_factor: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let market_contract_raw = deps.api.addr_canonicalize(market_contract.as_str())?;
    let mut market_config: MarketConfig = read_market_config(deps.storage, &market_contract_raw)?;

    if let Some(threshold_deposit_rate) = threshold_deposit_rate {
        market_config.threshold_deposit_rate = threshold_deposit_rate;
    }

    if let Some(buffer_distribution_factor) = buffer_distribution_factor {
        market_config.buffer_distribution_factor = buffer_distribution_factor;
    }

    if let Some(target_deposit_rate) = target_deposit_rate {
        market_config.target_deposit_rate = target_deposit_rate;
    }

    store_market_config(deps.storage, &market_contract_raw, &market_config)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_market_config"),
        attr("market_contract", market_contract.to_string()),
        attr(
            "threshold_deposit_rate",
            threshold_deposit_rate
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
        attr(
            "target_deposit_rate",
            target_deposit_rate
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
        attr(
            "buffer_distribution_factor",
            buffer_distribution_factor
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
    ]))
}

pub fn update_market_dyrate_config(
    deps: DepsMut,
    info: MessageInfo,
    market_contract: String,
    dyn_rate_epoch: Option<u64>,
    dyn_rate_maxchange: Option<Decimal256>,
    dyn_rate_yr_increase_expectation: Option<Decimal256>,
    dyn_rate_min: Option<Decimal256>,
    dyn_rate_max: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    let market_contract_raw = deps.api.addr_canonicalize(&market_contract.to_string())?;
    let mut dynrate_config: DynrateConfig =
        read_market_dynrate_config(deps.storage, &market_contract_raw)?;

    if let Some(dyn_rate_epoch) = dyn_rate_epoch {
        dynrate_config.dyn_rate_epoch = dyn_rate_epoch;
    }

    if let Some(dyn_rate_maxchange) = dyn_rate_maxchange {
        dynrate_config.dyn_rate_maxchange = dyn_rate_maxchange;
    }

    if let Some(dyn_rate_yr_increase_expectation) = dyn_rate_yr_increase_expectation {
        dynrate_config.dyn_rate_yr_increase_expectation = dyn_rate_yr_increase_expectation;
    }

    if let Some(dyn_rate_min) = dyn_rate_min {
        dynrate_config.dyn_rate_min = dyn_rate_min;
    }

    if let Some(dyn_rate_max) = dyn_rate_max {
        dynrate_config.dyn_rate_max = dyn_rate_max;
    }

    store_market_dynrate_config(deps.storage, &market_contract_raw, &dynrate_config)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_market_dyrate_config"),
        attr("market_contract", market_contract.to_string()),
        attr(
            "dyn_rate_epoch",
            dyn_rate_epoch
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
        attr(
            "dyn_rate_maxchange",
            dyn_rate_maxchange
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
        attr(
            "dyn_rate_yr_increase_expectation",
            dyn_rate_yr_increase_expectation
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
        attr(
            "dyn_rate_min",
            dyn_rate_min
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
        attr(
            "dyn_rate_max",
            dyn_rate_max
                .map(|rate| rate.to_string())
                .unwrap_or_else(|| "null".to_string()),
        ),
    ]))
}

fn update_deposit_rate(deps: &mut DepsMut, env:& Env, market_conctract: &CanonicalAddr) -> StdResult<()> {
    let market_dynrate_config: DynrateConfig =
        read_market_dynrate_config(deps.storage, &market_conctract)?;
    let market_dynrate_state: DynrateState =
        read_market_dynrate_state(deps.storage, &market_conctract)?;
    let mut market_config: MarketConfig = read_market_config(deps.storage, &market_conctract)?;
    let market_elem: MarketlistElem = read_marketlist_elem(deps.storage, &market_conctract)?;

    // check whether its time to re-evaluate rate
    if env.block.height
        >= market_dynrate_state.last_executed_height + market_dynrate_config.dyn_rate_epoch
    {
        // retrieve interest buffer
        let interest_buffer = query_balance(
            deps.as_ref(),
            env.contract.address.clone(),
            market_elem.stable_denom.to_string(),
        )?;
        // convert block rate into yearly rate
        let blocks_per_year = Decimal256::from_ratio(Uint256::from(BLOCKS_PER_YEAR), 1);
        let current_rate = market_config.threshold_deposit_rate * blocks_per_year;

        let yield_reserve = Decimal256::from_uint256(interest_buffer);
        let mut yr_went_up = yield_reserve > market_dynrate_state.prev_yield_reserve;

        // amount yield reserve changed in notional terms
        let yield_reserve_delta = if yr_went_up {
            yield_reserve - market_dynrate_state.prev_yield_reserve
        } else {
            market_dynrate_state.prev_yield_reserve - yield_reserve
        };

        // amount yield reserve changed in percentage terms
        // if the prev yield reserve was zero; assume either a 100% decrease
        // or a 100% increase, but this should be very rare
        let mut yield_reserve_change = if market_dynrate_state.prev_yield_reserve.is_zero() {
            Decimal256::one()
        } else {
            yield_reserve_delta / market_dynrate_state.prev_yield_reserve
        };

        // decreases the yield reserve change by dyn_rate_yr_increase_expectation
        // (assume (yr_went_up, yield_reserve_change) is one signed integer, this just subtracts
        // that integer by dynrate_config.dyn_rate_yr_increase_expectation)
        let increase_expectation = market_dynrate_config.dyn_rate_yr_increase_expectation;
        yield_reserve_change = if !yr_went_up {
            yield_reserve_change + increase_expectation
        } else if yield_reserve_change > increase_expectation {
            yield_reserve_change - increase_expectation
        } else {
            yr_went_up = !yr_went_up;
            increase_expectation - yield_reserve_change
        };

        yield_reserve_change = min(
            yield_reserve_change,
            market_dynrate_config.dyn_rate_maxchange,
        );

        let mut new_rate = if yr_went_up {
            current_rate + yield_reserve_change
        } else if current_rate > yield_reserve_change {
            current_rate - yield_reserve_change
        } else {
            Decimal256::zero()
        };

        // convert from yearly rate to block rate
        new_rate = new_rate / blocks_per_year;

        // clamp new rate
        new_rate = max(
            min(new_rate, market_dynrate_config.dyn_rate_max),
            market_dynrate_config.dyn_rate_min,
        );

        market_config.target_deposit_rate = new_rate;
        market_config.threshold_deposit_rate = new_rate;
        store_market_config(deps.storage, &market_conctract, &market_config)?;

        // store updated epoch state
        store_market_dynrate_state(
            deps.storage,
            &market_conctract,
            &DynrateState {
                last_executed_height: env.block.height,
                prev_yield_reserve: yield_reserve,
            },
        )?;
    };
    Ok(())
}

pub fn execute_epoch_operations(
    mut deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
    let marketlist: Vec<MarketlistResponseElem> = read_all_marketlist(deps.as_ref(), None, None)?;
    let whitelist: Vec<WhitelistResponseElem> = read_whitelist(deps.as_ref(), None, None)?;
    
    let mut messages: Vec<SubMsg> = Vec::new(); 
    let resp = Response::new();
    let mut attris: Vec<Attribute> =  Vec::new();

    //Update the deposit rate and epoch state of each stable market in turn
    for elem in marketlist {
        let market_contract = deps.as_ref().api.addr_canonicalize(elem.market_contract.as_str())?;
        let mut ret = execute_market_epoch_operations(&mut deps, &env, &market_contract, &mut messages)?;
        attris.append(&mut ret.attributes);
       
    }

    // Execute DistributeRewards
    for elem in whitelist.iter() {
        messages.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: elem.custody_contract.clone(),
            funds: vec![],
            msg: to_binary(&CustodyExecuteMsg::DistributeRewards {})?,
        })));
    }

    Ok(resp.add_submessages(messages).add_attributes(attris))

}

pub fn execute_market_epoch_operations(
    mut deps: &mut DepsMut,
    env: &Env,
    market_contract: &CanonicalAddr,
    messages: & mut Vec<SubMsg>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let state: EpochState = read_market_epoch_state(deps.storage, &market_contract)?;
    let market_config: MarketConfig = read_market_config(deps.storage, &market_contract)?;
    let market = read_marketlist_elem(deps.storage, &market_contract)?;

    if env.block.height < state.last_executed_height + config.epoch_period {
        return Err(ContractError::EpochNotPassed(state.last_executed_height));
    }

    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - state.last_executed_height);

    // Compute next epoch state
    let epoch_state: EpochStateResponse = query_market_epoch_state(
        deps.as_ref(),
        deps.api.addr_humanize(&market_contract)?,
        env.block.height,
        None,
    )?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate = epoch_state.exchange_rate / state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    let mut interest_buffer = query_balance(
        deps.as_ref(),
        env.contract.address.clone(),
        market.stable_denom.to_string(),
    )?;

    // Send accrued_buffer * config.krp_purchase_factor amount stable token to collector
    let accrued_buffer = interest_buffer - state.prev_interest_buffer;
    let krp_purchase_amount = accrued_buffer * config.krp_purchase_factor;
    if !krp_purchase_amount.is_zero() {
        messages.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: deps
                .api
                .addr_humanize(&config.collector_contract)?
                .to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: market.stable_denom.to_string(),
                    amount: krp_purchase_amount.into(),
                },
            )?],
        })));
    }

    // Deduct krp_purchase_amount from the interest_buffer
    interest_buffer = interest_buffer - krp_purchase_amount.into();

    // Distribute Interest Buffer to depositor
    // Only executed when deposit rate < threshold_deposit_rate
    let mut distributed_interest: Uint256 = Uint256::zero();
    if deposit_rate < market_config.threshold_deposit_rate {
        // missing_deposit_rate(_per_block)
        let missing_deposit_rate = market_config.threshold_deposit_rate - deposit_rate;
        let prev_deposits = state.prev_atoken_supply * state.prev_exchange_rate;

        // missing_deposits = prev_deposits * missing_deposit_rate(_per_block) * blocks
        let missing_deposits = prev_deposits * blocks * missing_deposit_rate;
        let distribution_buffer = interest_buffer * market_config.buffer_distribution_factor;

        // When there was not enough deposits happens,
        // distribute interest to market contract
        distributed_interest = std::cmp::min(missing_deposits, distribution_buffer);
        interest_buffer = interest_buffer - distributed_interest;

        if !distributed_interest.is_zero() {
            //deduct tax
            distributed_interest = Uint256::from(
                deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: market.stable_denom.clone(),
                        amount: distributed_interest.into(),
                    },
                )?
                .amount,
            );

            // Send some portion of interest buffer to Market contract
            messages.push(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: deps.api.addr_humanize(&market_contract)?.to_string(),
                amount: vec![Coin {
                    denom: market.stable_denom.clone(),
                    amount: distributed_interest.into(),
                }],
            })));
        }
    }
    
    let resp = update_market_epoch_state(
        &mut deps,
        &env,
        &market_contract,
        interest_buffer,
        distributed_interest,
        messages,
    )?;
    
    Ok(Response::new()
        .add_attributes(resp.attributes)
        .add_attributes(vec![
            attr("action", "epoch_market_operations"),
            attr("stable_name", market.stable_name.to_string()),
            attr("stable_denom", market.stable_denom.to_string()),
            attr("deposit_rate", deposit_rate.to_string()),
            attr("exchange_rate", epoch_state.exchange_rate.to_string()),
            attr("atoken_supply", epoch_state.atoken_supply),
            attr("distributed_interest", distributed_interest),
        ]))
}

pub fn update_market_epoch_state(
    deps: &mut DepsMut,
    env: &Env,
    market_contract: &CanonicalAddr,
    // To store interest buffer before receiving epoch staking rewards,
    // pass interest_buffer from execute_epoch_operations
    interest_buffer: Uint256,
    distributed_interest: Uint256,
    messages: &mut Vec<SubMsg>,
) -> Result<Response, ContractError> {
    let market_config: MarketConfig = read_market_config(deps.storage, &market_contract)?;
    let overseer_market_epoch_state: EpochState =
        read_market_epoch_state(deps.storage, &market_contract)?;
   
    let market_address = deps.api.addr_humanize(&market_contract)?;
    // # of blocks from the last executed height
    let blocks = Uint256::from(env.block.height - overseer_market_epoch_state.last_executed_height);

    // Compute next epoch state
    let market_epoch_state: EpochStateResponse = query_market_epoch_state(
        deps.as_ref(),
        deps.api.addr_humanize(&market_contract.clone())?,
        env.block.height,
        Some(distributed_interest),
    )?;

    // effective_deposit_rate = cur_exchange_rate / prev_exchange_rate
    // deposit_rate = (effective_deposit_rate - 1) / blocks
    let effective_deposit_rate =
        market_epoch_state.exchange_rate / overseer_market_epoch_state.prev_exchange_rate;
    let deposit_rate =
        (effective_deposit_rate - Decimal256::one()) / Decimal256::from_uint256(blocks);

    // store updated epoch state

    store_market_epoch_state(
        deps.storage,
        &market_contract,
        &EpochState {
            last_executed_height: env.block.height,
            prev_atoken_supply: market_epoch_state.atoken_supply,
            prev_exchange_rate: market_epoch_state.exchange_rate,
            prev_interest_buffer: interest_buffer,
            deposit_rate,
        },
    )?;

    // use unchanged rates to build msg
    let response_msg = to_binary(&MarketExecuteMsg::ExecuteEpochOperations {
        deposit_rate,
        target_deposit_rate: market_config.target_deposit_rate,
        threshold_deposit_rate: market_config.threshold_deposit_rate,
        distributed_interest,
    })?;

    // proceed with deposit rate update
    update_deposit_rate(deps, &env, &market_contract)?;
 

    messages.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: market_address.to_string(),
        funds: vec![],
        msg: response_msg,
    })));
    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "update_maket_epoch_state"),
            attr("market_contract", deps.api.addr_humanize(&market_contract)?.to_string()),
            attr("deposit_rate", deposit_rate.to_string()),
            attr("atoken_supply", market_epoch_state.atoken_supply),
            attr(
                "exchange_rate",
                market_epoch_state.exchange_rate.to_string(),
            ),
            attr("interest_buffer", interest_buffer),
        ]))
}

pub fn fund_reserve(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let sent_uusd = match info.funds.iter().find(|x| x.denom == "uusd") {
        Some(coin) => coin.amount,
        None => Uint128::zero(),
    };
    let market_list = read_all_marketlist(deps.as_ref(), None, None)?;
    for market in market_list {
        let market_contract_raw = deps
            .api
            .addr_canonicalize(market.market_contract.as_str())?;
        let mut overseer_market_epoch_state: EpochState =
            read_market_epoch_state(deps.storage, &market_contract_raw)?;
        overseer_market_epoch_state.prev_interest_buffer += Uint256::from(sent_uusd);
        store_market_epoch_state(
            deps.storage,
            &market_contract_raw,
            &overseer_market_epoch_state,
        )?;

        let mut market_dyn_rate_state: DynrateState =
            read_market_dynrate_state(deps.storage, &market_contract_raw)?;
        market_dyn_rate_state.prev_yield_reserve +=
            Decimal256::from_ratio(Uint256::from(sent_uusd), 1);
        store_market_dynrate_state(deps.storage, &market_contract_raw, &market_dyn_rate_state)?;
    }

    Ok(Response::new().add_attributes(vec![
        attr("action", "fund_reserve"),
        attr("funded_amount", sent_uusd.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::EpochState { market_contract } => to_binary(&query_state(deps, market_contract)?),
        QueryMsg::Whitelist {
            collateral_token,
            start_after,
            limit,
        } => to_binary(&query_whitelist(
            deps,
            optional_addr_validate(deps.api, collateral_token)?,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
        QueryMsg::Collaterals { borrower } => to_binary(&query_collaterals(
            deps,
            deps.api.addr_validate(&borrower)?,
        )?),
        QueryMsg::AllCollaterals { start_after, limit } => to_binary(&query_all_collaterals(
            deps,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
        QueryMsg::BorrowLimit {
            borrower,
            block_time,
        } => to_binary(&query_borrow_limit(
            deps,
            deps.api.addr_validate(&borrower)?,
            block_time,
        )?),
        QueryMsg::DynrateState { market_contract } => {
            to_binary(&query_dynrate_state(deps, market_contract)?)
        }
        QueryMsg::MarketConfig { market_contract } => {
            to_binary(&query_market_config(deps, market_contract)?)
        }
        QueryMsg::MarketList {
            market_contract,
            start_after,
            limit,
        } => to_binary(&query_market_list(
            deps,
            optional_addr_validate(deps.api, market_contract)?,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        owner_addr: deps.api.addr_humanize(&config.owner_addr)?.to_string(),
        oracle_contract: deps.api.addr_humanize(&config.oracle_contract)?.to_string(),
        liquidation_contract: deps
            .api
            .addr_humanize(&config.liquidation_contract)?
            .to_string(),
        collector_contract: deps
            .api
            .addr_humanize(&config.collector_contract)?
            .to_string(),
        epoch_period: config.epoch_period,
        krp_purchase_factor: config.krp_purchase_factor,
        price_timeframe: config.price_timeframe,
    })
}

pub fn query_state(deps: Deps, market_contract: String) -> StdResult<EpochState> {
    read_market_epoch_state(
        deps.storage,
        &deps.api.addr_canonicalize(&market_contract.as_str())?,
    )
}

pub fn query_dynrate_state(deps: Deps, market_contract: String) -> StdResult<DynrateState> {
    read_market_dynrate_state(
        deps.storage,
        &deps.api.addr_canonicalize(market_contract.as_str())?,
    )
}

pub fn query_market_config(deps: Deps, market_contract: String) -> StdResult<MarketConfig> {
    read_market_config(
        deps.storage,
        &deps.api.addr_canonicalize(market_contract.as_str())?,
    )
}

pub fn query_whitelist(
    deps: Deps,
    collateral_token: Option<Addr>,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<WhitelistResponse> {
    if let Some(collateral_token) = collateral_token {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(
            deps.storage,
            &deps.api.addr_canonicalize(collateral_token.as_str())?,
        )?;
        Ok(WhitelistResponse {
            elems: vec![WhitelistResponseElem {
                name: whitelist_elem.name,
                symbol: whitelist_elem.symbol,
                max_ltv: whitelist_elem.max_ltv,
                custody_contract: deps
                    .api
                    .addr_humanize(&whitelist_elem.custody_contract)?
                    .to_string(),
                collateral_token: collateral_token.to_string(),
            }],
        })
    } else {
        let start_after = if let Some(start_after) = start_after {
            Some(deps.api.addr_canonicalize(start_after.as_str())?)
        } else {
            None
        };

        let whitelist: Vec<WhitelistResponseElem> = read_whitelist(deps, start_after, limit)?;
        Ok(WhitelistResponse { elems: whitelist })
    }
}

pub fn query_market_list(
    deps: Deps,
    market_contract: Option<Addr>,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<MarketlistResponse> {
    if let Some(market_contract) = market_contract {
        let market_elem: MarketlistElem = read_marketlist_elem(
            deps.storage,
            &deps.api.addr_canonicalize(market_contract.as_str())?,
        )?;
        Ok(MarketlistResponse {
            elems: vec![MarketlistResponseElem {
                market_contract: market_contract.to_string(),
                stable_denom: market_elem.stable_denom.to_string(),
                stable_name: market_elem.stable_name,
            }],
        })
    } else {
        let start_after = if let Some(start_after) = start_after {
            Some(deps.api.addr_canonicalize(start_after.as_str())?)
        } else {
            None
        };

        let marketlist = read_all_marketlist(deps, start_after, limit)?;
        Ok(MarketlistResponse { elems: marketlist })
    }
}
