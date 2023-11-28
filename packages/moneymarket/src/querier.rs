use std::ops::Deref;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_json_binary,
    Addr,
    AllBalanceResponse,
    BalanceResponse,
    BankQuery,
    Coin,
    Deps,
    QueryRequest,
    StdResult,
    Uint128,
    WasmQuery, 
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use crate::common::CustomQuerier;
// use crate::common::QueryTaxWrapper;

use crate::oracle::PriceResponse;
use crate::oracle_pyth::{PriceResponse as PythPriceResponse,QueryMsg as PythOracleQueryMsg};

pub fn query_all_balances(deps: Deps, account_addr: Addr) -> StdResult<Vec<Coin>> {
    // load price form the oracle
    let all_balances: AllBalanceResponse =
        deps.querier
            .query(&QueryRequest::Bank(BankQuery::AllBalances {
                address: account_addr.to_string(),
            }))?;
    Ok(all_balances.amount)
}

pub fn query_balance(deps: Deps, account_addr: Addr, denom: String) -> StdResult<Uint256> {
    // load price form the oracle
    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount.into())
}

//modify response type to Cw20BalanceResponse，query balance correct，otherwise always is 0
pub fn query_token_balance(
    deps: Deps,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint256> {
    // load balance form the token contract
    let balance: Cw20BalanceResponse = deps
        .querier
        .query_wasm_smart(
            contract_addr.to_string(),
            &Cw20QueryMsg::Balance {
                address: account_addr.to_string(),
            },
        )
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });
    Ok(Uint256::from(balance.balance))
}

/*
pub fn query_token_balance(
    deps: Deps,
   // querier: &QuerierWrapper,
    contract_addr: impl Into<String>,
    account_addr: impl Into<String>,
) -> StdResult<Uint256> {
    // load balance from the token contract
    let resp: Cw20BalanceResponse = deps
        .querier
        .query_wasm_smart(
            contract_addr,
            &Cw20QueryMsg::Balance {
                address: account_addr.into(),
            },
        )
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });

    Ok(Uint256::from(resp.balance))
}
*/

pub fn query_supply(deps: Deps, contract_addr: Addr) -> StdResult<Uint256> {
    // load price form the oracle
    let token_info: TokenInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: contract_addr.to_string(),
            msg: to_json_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;

    Ok(Uint256::from(token_info.total_supply))
}

pub fn query_tax_rate_and_cap(deps: Deps, denom: String) -> StdResult<(Decimal256, Uint256)> {
    let custom_querier = CustomQuerier::new(deps.querier.deref());
    let rate = custom_querier.query_tax_rate()?.rate;
    let cap = custom_querier.query_tax_cap(denom)?.cap;

    Ok((rate.into(), cap.into()))
}

pub fn query_tax_rate(deps: Deps) -> StdResult<Decimal256> {
    let custom_querier = CustomQuerier::new(deps.querier.deref());
    Ok(custom_querier.query_tax_rate()?.rate.into())
}



pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    let custom_querier = CustomQuerier::new(deps.querier.deref());
    // let custom_querier: Deps<QueryTaxWrapper> = deps.as_ref();
    let tax_rate = Decimal256::from((custom_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((custom_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(
        amount * Decimal256::one() - amount / (Decimal256::one() + tax_rate),
        tax_cap,
    ))
}

pub fn deduct_tax(deps: Deps, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: (Uint256::from(coin.amount) - tax_amount).into(),
    })
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TimeConstraints {
    pub block_time: u64,
    pub valid_timeframe: u64,
}

pub fn query_price(
    deps: Deps,
    oracle_addr: Addr,
    base: String,
    _quote: String,
    _time_constraints: Option<TimeConstraints>,
) -> StdResult<PriceResponse> {
    // The time check has been set here
    let pyth_oracle_price: PythPriceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: oracle_addr.to_string(),
            msg: to_json_binary(&PythOracleQueryMsg::QueryPrice { asset: base })?,
        }))?;

    let oracle_price = PriceResponse {
        rate: pyth_oracle_price.emv_price,
        last_updated_base: pyth_oracle_price.last_updated_base,
        last_updated_quote: pyth_oracle_price.last_updated_quote,
    };

    // let oracle_price: PriceResponse =
    //     deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
    //         contract_addr: oracle_addr.to_string(),
    //         msg: to_json_binary(&OracleQueryMsg::Price { base, quote })?,
    //     }))?;
    //
    //
    // if let Some(time_constraints) = time_constraints {
    //     let valid_update_time = time_constraints.block_time - time_constraints.valid_timeframe;
    //     if oracle_price.last_updated_base < valid_update_time
    //         || oracle_price.last_updated_quote < valid_update_time
    //     {
    //         let error_msg = format!(" Price is too old;time_constraints.block_time:{}, time_constraints.valid_timeframe:{},
    //         oracle_price.last_updated_base: {}, valid_update_time: {}, oracle_price.last_updated_quote: {}",
    //                                 time_constraints.block_time, time_constraints.valid_timeframe, oracle_price.last_updated_base,
    //                                 valid_update_time, oracle_price.last_updated_quote);
    //
    //         return Err(StdError::generic_err(error_msg));
    //     }
    // }

    Ok(oracle_price)
}
