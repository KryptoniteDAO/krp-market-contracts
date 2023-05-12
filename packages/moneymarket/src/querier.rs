use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary,
    Addr,
    AllBalanceResponse,
    BalanceResponse,
    BankQuery,
    Coin,
    Deps,
    QueryRequest,
    StdError,
    StdResult,
    Uint128,
    WasmQuery, //QuerierWrapper,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use crate::oracle::{PriceResponse, QueryMsg as OracleQueryMsg};

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
            msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;

    Ok(Uint256::from(token_info.total_supply))
}

pub fn query_tax_rate_and_cap(_deps: Deps, _denom: String) -> StdResult<(Decimal256, Uint256)> {
    // let terra_querier = TerraQuerier::new(&deps.querier);
    // let rate = terra_querier.query_tax_rate()?.rate;
    // let cap = terra_querier.query_tax_cap(denom)?.cap;
    let rate = Decimal256::zero();
    let cap = Uint256::zero();
    Ok((rate.into(), cap.into()))
}

pub fn query_tax_rate(_deps: Deps) -> StdResult<Decimal256> {
    // let terra_querier = TerraQuerier::new(&deps.querier);
    // Ok(terra_querier.query_tax_rate()?.rate.into())
    Ok(Decimal256::zero().into())
}

pub fn compute_tax(_deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    // let terra_querier = TerraQuerier::new(&deps.querier);
    // let tax_rate = Decimal256::from((terra_querier.query_tax_rate()?).rate);
    // let tax_cap = Uint256::from((terra_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let tax_rate = Decimal256::zero();
    let tax_cap = Uint256::zero();
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(
        amount * Decimal256::one() - amount / (Decimal256::one() + tax_rate),
        tax_cap,
    ))
}

pub fn deduct_tax(_deps: Deps, coin: Coin) -> StdResult<Coin> {
    //let tax_amount = compute_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        //amount: (Uint256::from(coin.amount) - tax_amount).into(),
        amount: Uint256::from(coin.amount).into(),
    })
}

pub fn deduct_tax_new(_deps: Deps, burn_amount: Uint128) -> StdResult<Uint256> {
    // let tax_cap = Uint256::one();
    // let protocal_fee_rate = Decimal256::from_ratio(5, 1000);

    // Ok(std::cmp::min(
    //     Uint256::from(burn_amount) * Decimal256::one() - Uint256::from(burn_amount) / (Decimal256::one() + protocal_fee_rate),
    //     tax_cap,
    //     ))
    Ok(Uint256::from(burn_amount))
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
    quote: String,
    time_constraints: Option<TimeConstraints>,
) -> StdResult<PriceResponse> {
    let oracle_price: PriceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: oracle_addr.to_string(),
            msg: to_binary(&OracleQueryMsg::Price { base, quote })?,
        }))?;

    if let Some(time_constraints) = time_constraints {
        let valid_update_time = time_constraints.block_time - time_constraints.valid_timeframe;
        if oracle_price.last_updated_base < valid_update_time
            || oracle_price.last_updated_quote < valid_update_time
        {
            let error_msg = format!(" Price is too old;time_constraints.block_time:{}, time_constraints.valid_timeframe:{},
            oracle_price.last_updated_base: {}, valid_update_time: {}, oracle_price.last_updated_quote: {}",
                                    time_constraints.block_time, time_constraints.valid_timeframe, oracle_price.last_updated_base,
                                    valid_update_time, oracle_price.last_updated_quote);

            return Err(StdError::generic_err(error_msg));
        }
    }

    Ok(oracle_price)
}
