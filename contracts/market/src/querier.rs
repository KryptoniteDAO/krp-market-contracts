use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{to_json_binary, Addr, Deps, QueryRequest, StdResult, WasmQuery};

use moneymarket::distribution_model::{KptEmissionRateResponse, QueryMsg as DistributionQueryMsg};
use moneymarket::interest_model::{BorrowRateResponse, QueryMsg as InterestQueryMsg};
use moneymarket::overseer::{BorrowLimitResponse, ConfigResponse, QueryMsg as OverseerQueryMsg};

pub fn query_borrow_rate(
    deps: Deps,
    interest_addr: Addr,
    market_balance: Uint256,
    total_liabilities: Decimal256,
    total_reserves: Decimal256,
) -> StdResult<BorrowRateResponse> {
    let borrow_rate: BorrowRateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: interest_addr.to_string(),
            msg: to_json_binary(&InterestQueryMsg::BorrowRate {
                market_balance,
                total_liabilities,
                total_reserves,
            })?,
        }))?;

    Ok(borrow_rate)
}

pub fn query_borrow_limit(
    deps: Deps,
    overseer_addr: Addr,
    borrower: Addr,
    block_time: Option<u64>,
) -> StdResult<BorrowLimitResponse> {
    let borrow_limit: BorrowLimitResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: overseer_addr.to_string(),
            msg: to_json_binary(&OverseerQueryMsg::BorrowLimit {
                borrower: borrower.to_string(),
                block_time,
            })?,
        }))?;

    Ok(borrow_limit)
}

pub fn query_kpt_emission_rate(
    deps: Deps,
    distribution_model: Addr,
    deposit_rate: Decimal256,
    target_deposit_rate: Decimal256,
    threshold_deposit_rate: Decimal256,
    current_emission_rate: Decimal256,
) -> StdResult<KptEmissionRateResponse> {
    let kpt_emission_rate: KptEmissionRateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: distribution_model.to_string(),
            msg: to_json_binary(&DistributionQueryMsg::KptEmissionRate {
                deposit_rate,
                target_deposit_rate,
                threshold_deposit_rate,
                current_emission_rate,
            })?,
        }))?;

    Ok(kpt_emission_rate)
}

pub fn query_target_deposit_rate(deps: Deps, overseer_contract: Addr) -> StdResult<Decimal256> {
    let overseer_config: ConfigResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: overseer_contract.to_string(),
            msg: to_json_binary(&OverseerQueryMsg::Config {})?,
        }))?;

    Ok(overseer_config.target_deposit_rate)
}
