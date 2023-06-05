use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    to_binary,
    Addr,
    AllBalanceResponse,
    BalanceResponse,
    BankQuery,
    CanonicalAddr,
    Coin,
    Deps,
    QueryRequest,
    StdResult,
    Uint128,
    WasmQuery, //QuerierWrapper,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use crate::market::{
    BorrowerInfoResponse, ConfigResponse as MarketConfigResponse, QueryMsg as MarketQueryMsg,
    StateResponse,
};
use crate::oracle::PriceResponse;
use crate::oracle_pyth::{PriceResponse as PythPriceResponse, QueryMsg as PythOracleQueryMsg};
use crate::overseer::{ConfigResponse, MarketlistResponse, QueryMsg as OverseerQueryMsg};

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
    _quote: String,
    _time_constraints: Option<TimeConstraints>,
) -> StdResult<PriceResponse> {
    // The time check has been set here
    let pyth_oracle_price: PythPriceResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: oracle_addr.to_string(),
            msg: to_binary(&PythOracleQueryMsg::QueryPrice { asset: base })?,
        }))?;

    let oracle_price = PriceResponse {
        rate: pyth_oracle_price.emv_price,
        last_updated_base: pyth_oracle_price.last_updated_base,
        last_updated_quote: pyth_oracle_price.last_updated_quote,
    };

    Ok(oracle_price)
}

pub fn query_overseer_config(
    deps: Deps,
    overseer_contract: CanonicalAddr,
) -> StdResult<ConfigResponse> {
    let overseer_config: ConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&overseer_contract)?.to_string(),
            msg: to_binary(&OverseerQueryMsg::Config {})?,
        }))?;

    Ok(overseer_config)
}

pub fn query_market_config(
    deps: Deps,
    market_contract: CanonicalAddr,
) -> StdResult<MarketConfigResponse> {
    let config: MarketConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&market_contract)?.to_string(),
            msg: to_binary(&MarketQueryMsg::Config {})?,
        }))?;

    Ok(config)
}

pub fn query_market_list(
    deps: Deps,
    overseer_contract: CanonicalAddr,
) -> StdResult<MarketlistResponse> {
    let market_list_res: MarketlistResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&overseer_contract)?.to_string(),
            msg: to_binary(&OverseerQueryMsg::MarketList {
                market_contract: None,
                start_after: None,
                limit: None,
            })?,
        }))?;

    Ok(market_list_res)
}

pub fn query_market_state(deps: Deps, market_contract: CanonicalAddr) -> StdResult<StateResponse> {
    let market_state: StateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&market_contract)?.to_string(),
            msg: to_binary(&MarketQueryMsg::State { block_height: None })?,
        }))?;

    Ok(market_state)
}

/// Query borrow amount from the market contract
pub fn query_borrower_info(
    deps: Deps,
    market_addr: Addr,
    borrower: Addr,
    is_loan_value: bool,
    block_height: u64,
) -> StdResult<BorrowerInfoResponse> {
    let borrower_amount: BorrowerInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: market_addr.to_string(),
            msg: to_binary(&MarketQueryMsg::BorrowerInfo {
                borrower: borrower.to_string(),
                is_loan_value: Some(is_loan_value),
                block_height: Some(block_height),
             
            })?,
        }))?;

    Ok(borrower_amount)
}
