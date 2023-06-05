use crate::contract::query_market_list;
use crate::error::ContractError;
use crate::querier::{query_liquidation_amount};
use crate::state::{
    read_all_collaterals, read_collaterals, read_config, read_marketlist_elem, read_whitelist,
    read_whitelist_elem, store_collaterals, Config, WhitelistElem,
};
use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};

use moneymarket::custody::ExecuteMsg as CustodyExecuteMsg;
use moneymarket::liquidation::LiquidationAmountResponse;
use moneymarket::market::{BorrowerInfoResponse, ExecuteMsg as MarketExecuteMsg};
use moneymarket::oracle::PriceResponse;
use moneymarket::overseer::{
    AllCollateralsResponse, BorrowLimitResponse, CollateralsResponse, WhitelistResponseElem,
};
use moneymarket::querier::{query_balance, query_price, query_borrower_info,};
use moneymarket::tokens::{Tokens, TokensHuman, TokensMath, TokensToHuman, TokensToRaw};

pub fn lock_collateral(
    deps: DepsMut,
    info: MessageInfo,
    borrower: String,
    collaterals_human: TokensHuman,
) -> Result<Response, ContractError> {
    let whitelist: Vec<WhitelistResponseElem> = read_whitelist(deps.as_ref(), None, None)?;

    let mut is_white_custody = false;
    for elem in whitelist.iter() {
        if info.sender.to_string() == elem.custody_contract.clone() {
            is_white_custody = true;
            break;
        }
    }
    if false == is_white_custody {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "sender {} lock collateral Unauthorized",
            info.sender.to_string()
        ))));
    }

    let borrower_raw = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut cur_collaterals: Tokens = read_collaterals(deps.storage, &borrower_raw);

    let collaterals: Tokens = collaterals_human.to_raw(deps.as_ref())?;

    cur_collaterals.add(collaterals.clone());
    store_collaterals(deps.storage, &borrower_raw, &cur_collaterals)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(deps.storage, &collateral.0)?;
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&whitelist_elem.custody_contract)?
                .to_string(),
            funds: vec![],
            msg: to_binary(&CustodyExecuteMsg::LockCollateral {
                borrower: borrower.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Logging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals_human
        .iter()
        .map(|c| format!("{}{}", c.1, c.0))
        .collect();

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "lock_collateral"),
        attr("borrower", info.sender),
        attr("collaterals", collateral_logs.join(",")),
    ]))
}

pub fn unlock_collateral(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collaterals_human: TokensHuman,
) -> Result<Response, ContractError> {
    let market_list = query_market_list(deps.as_ref(), None, None, None)?;

    let borrower = info.sender;
    let borrower_raw = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut cur_collaterals: Tokens = read_collaterals(deps.storage, &borrower_raw);
    let collaterals: Tokens = collaterals_human.to_raw(deps.as_ref())?;

    // Underflow check is done in sub_collateral
    if cur_collaterals.sub(collaterals.clone()).is_err() {
        return Err(ContractError::UnlockExceedsLocked {});
    }

    // Compute borrow limit with collaterals except unlock target collaterals
    let (borrow_limit, _) = compute_borrow_limit(
        deps.as_ref(),
        &cur_collaterals,
        Some(env.block.time.seconds()),
    )?;

    let mut total_borrow_value = Uint256::zero();
    for market in market_list.elems {
        let borrow_amount_res: BorrowerInfoResponse = query_borrower_info(
            deps.as_ref(),
            deps.api.addr_validate(market.market_contract.as_str())?,
            borrower.clone(),
            true,
            env.block.height,
        )?;
        if let Some(loan_value) = borrow_amount_res.loan_value {
            total_borrow_value += loan_value;
        }
    }

    if borrow_limit < total_borrow_value {
        return Err(ContractError::UnlockTooLarge(borrow_limit.into()));
    }

    store_collaterals(deps.storage, &borrower_raw, &cur_collaterals)?;

    let mut messages: Vec<SubMsg> = vec![];
    for collateral in collaterals {
        let whitelist_elem: WhitelistElem = read_whitelist_elem(deps.storage, &collateral.0)?;
        messages.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&whitelist_elem.custody_contract)?
                .to_string(),
            funds: vec![],
            msg: to_binary(&CustodyExecuteMsg::UnlockCollateral {
                borrower: borrower.to_string(),
                amount: collateral.1,
            })?,
        })));

        messages.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps
                .api
                .addr_humanize(&whitelist_elem.custody_contract)?
                .to_string(),
            funds: vec![],
            msg: to_binary(&CustodyExecuteMsg::WithdrawCollateral {
                borrower: borrower.to_string(),
                amount: Some(collateral.1),
            })?,
        })));
    }

    // Logging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals_human
        .iter()
        .map(|c| format!("{}{}", c.1, c.0))
        .collect();

    Ok(Response::new()
        .add_submessages(messages)
        .add_attributes(vec![
            attr("action", "unlock_collateral"),
            attr("borrower", borrower),
            attr("collaterals", collateral_logs.join(",")),
        ]))
}

pub fn liquidate_collateral(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    borrower: Addr,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let market_list = query_market_list(deps.as_ref(), None, None, None)?;

    let borrower_raw = deps.api.addr_canonicalize(borrower.as_str())?;
    let mut cur_collaterals: Tokens = read_collaterals(deps.storage, &borrower_raw);

    // Compute borrow limit with collaterals except unlock target collaterals
    let (borrow_limit, collateral_prices) = compute_borrow_limit(
        deps.as_ref(),
        &cur_collaterals,
        Some(env.block.time.seconds()),
    )?;

    let mut market_prev_balance_array: Vec<(String, Uint256)> = Vec::new();

    let mut total_borrow_value = Uint256::zero();
    let mut market_loan_array: Vec<(String, Uint256)> = Vec::new();
    for market in market_list.elems {
        let borrow_amount_res: BorrowerInfoResponse = query_borrower_info(
            deps.as_ref(),
            deps.api.addr_validate(market.market_contract.as_str())?,
            borrower.clone(),
            true,
            env.block.height,
        )?;

        if let Some(loan_value) = borrow_amount_res.loan_value {
            total_borrow_value += loan_value;
            market_loan_array.push((
                market.market_contract.to_string(),
                loan_value,
            ));
        }

        let prev_balance: Uint256 =
            query_balance(deps.as_ref(), deps.api.addr_validate(market.market_contract.as_str())?, market.stable_denom)?;
        market_prev_balance_array.push((market.market_contract.to_string(), prev_balance));
    }

    // borrow limit is equal or bigger than loan amount
    // cannot liquidation collaterals
    if borrow_limit >= total_borrow_value {
        return Err(ContractError::CannotLiquidateSafeLoan {});
    }

    let liquidation_amount_res: LiquidationAmountResponse = query_liquidation_amount(
        deps.as_ref(),
        deps.api.addr_humanize(&config.liquidation_contract)?,
        total_borrow_value,
        borrow_limit,
        &cur_collaterals.to_human(deps.as_ref())?,
        collateral_prices,
    )?;

    let liquidation_amount = liquidation_amount_res.collaterals.to_raw(deps.as_ref())?;

    // Store left collaterals
    cur_collaterals.sub(liquidation_amount.clone())?;
    store_collaterals(deps.storage, &borrower_raw, &cur_collaterals)?;

    let liquidation_messages: Vec<CosmosMsg> = liquidation_amount
        .iter()
        .map(|collateral| {
            let whitelist_elem: WhitelistElem = read_whitelist_elem(deps.storage, &collateral.0)?;

            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps
                    .api
                    .addr_humanize(&whitelist_elem.custody_contract)?
                    .to_string(),
                funds: vec![],
                msg: to_binary(&CustodyExecuteMsg::LiquidateCollateral {
                    liquidator: info.sender.to_string(),
                    borrower: borrower.to_string(),
                    amount: collateral.1,
                    total_borrow_amount: total_borrow_value,
                })?,
            }))
        })
        .filter(|msg| msg.is_ok())
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    let market_repay_messages: Vec<CosmosMsg> = market_prev_balance_array
        .iter()
        .map(|market| {
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: market.0.to_string(),
                funds: vec![],
                msg: to_binary(&MarketExecuteMsg::RepayStableFromLiquidation {
                    borrower: borrower.to_string(),
                    prev_balance: market.1,
                })?,
            }))
        })
        .filter(|msg| msg.is_ok())
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(Response::new()
        .add_messages(liquidation_messages)
        .add_messages(market_repay_messages))
}

pub fn repay_stable_from_yield_reserve(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    market_contract: Addr,
    borrower: Addr,
) -> Result<Response, ContractError> {
    let market = read_marketlist_elem(
        deps.storage,
        &deps.api.addr_canonicalize(&market_contract.as_str())?,
    )?;

    let borrow_amount_res: BorrowerInfoResponse = query_borrower_info(
        deps.as_ref(),
        market_contract.clone(),
        borrower.clone(),
        false,
        env.block.height,
    )?;
    let borrow_amount = borrow_amount_res.loan_amount;

    let prev_balance: Uint256 = query_balance(
        deps.as_ref(),
        market_contract.clone(),
        market.stable_denom.to_owned(),
    )?;

    let repay_messages = vec![
        CosmosMsg::Bank(BankMsg::Send {
            to_address: market_contract.to_string(),
            amount: vec![Coin {
                denom: market.stable_denom,
                amount: borrow_amount.into(),
            }],
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market_contract.to_string(),
            funds: vec![],
            msg: to_binary(&MarketExecuteMsg::RepayStableFromLiquidation {
                borrower: borrower.to_string(),
                prev_balance,
            })?,
        }),
    ];

    Ok(Response::new().add_messages(repay_messages))
}

pub fn query_collaterals(deps: Deps, borrower: Addr) -> StdResult<CollateralsResponse> {
    let collaterals: Tokens = read_collaterals(
        deps.storage,
        &deps.api.addr_canonicalize(borrower.as_str())?,
    );

    Ok(CollateralsResponse {
        borrower: borrower.to_string(),
        collaterals: collaterals
            .iter()
            .map(|c| Ok((deps.api.addr_humanize(&c.0)?.to_string(), c.1)))
            .collect::<StdResult<Vec<(String, Uint256)>>>()?,
    })
}

pub fn query_all_collaterals(
    deps: Deps,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<AllCollateralsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some(deps.api.addr_canonicalize(start_after.as_str())?)
    } else {
        None
    };

    let all_collaterals: Vec<CollateralsResponse> = read_all_collaterals(deps, start_after, limit)?;

    Ok(AllCollateralsResponse { all_collaterals })
}

#[allow(clippy::ptr_arg)]
pub(crate) fn compute_borrow_limit(
    deps: Deps,
    collaterals: &Tokens,
    _block_time: Option<u64>,
) -> StdResult<(Uint256, Vec<Decimal256>)> {
    let config: Config = read_config(deps.storage)?;
    let oracle_contract = deps.api.addr_humanize(&config.oracle_contract)?;

    let mut borrow_limit: Uint256 = Uint256::zero();
    let mut collateral_prices: Vec<Decimal256> = vec![];
    for collateral in collaterals.iter() {
        let collateral_token = collateral.0.clone();
        let collateral_amount = collateral.1;

        let price: PriceResponse = query_price(
            deps,
            oracle_contract.clone(),
            (deps.api.addr_humanize(&collateral_token)?).to_string(),
            "".to_string(),
            None,
        )?;

        let elem: WhitelistElem = read_whitelist_elem(deps.storage, &collateral.0)?;
        let collateral_value = collateral_amount * price.rate;
        borrow_limit += collateral_value * elem.max_ltv;
        collateral_prices.push(price.rate);
    }

    // returns borrow_limit with collaterals value in stable denom
    Ok((borrow_limit, collateral_prices))
}

pub fn query_borrow_limit(
    deps: Deps,
    borrower: Addr,
    block_time: Option<u64>,
) -> StdResult<BorrowLimitResponse> {
    let collaterals = read_collaterals(
        deps.storage,
        &deps.api.addr_canonicalize(borrower.as_str())?,
    );

    // Compute borrow limit with collaterals
    let (borrow_limit, _) = compute_borrow_limit(deps, &collaterals, block_time)?;

    Ok(BorrowLimitResponse {
        borrower: borrower.to_string(),
        borrow_limit,
    })
}
