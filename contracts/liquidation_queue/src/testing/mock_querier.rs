use moneymarket::overseer::{WhitelistResponse, WhitelistResponseElem};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_json, to_json_binary, Coin, ContractResult, Decimal, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery, CustomQuery,
};
use std::collections::HashMap;
use moneymarket::oracle_pyth::PriceResponse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
/// Query oracle price to oracle contract
    QueryPrice { asset: String },
    Whitelist {
        collateral_token: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}


/// QueryTaxWrapper is an override of QueryRequest::Custom for testing
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryTaxWrapper {
    pub query_data: QueryTaxMsg,
}

// implement custom query
impl CustomQuery for QueryTaxWrapper {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum  QueryTaxMsg {
    ///Query tax, usually is zero
    TaxRate {},
///Query tax cap, usually is zero
    TaxCap { denom: String }, 
}

/// TaxRateResponse is data format returned from TreasuryRequest::TaxRate query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaxRateResponse {
    pub rate: Decimal,
}

/// TaxCapResponse is data format returned from TreasuryRequest::TaxCap query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaxCapResponse {
    pub cap: Uint128,
}


/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: Default::default(),
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<QueryTaxWrapper>,
    tax_querier: TaxQuerier,
    oracle_price_querier: OraclePriceQuerier,
    collateral_querier: CollateralQuerier,
}

#[derive(Clone, Default)]
pub struct CollateralQuerier {
    collaterals: HashMap<String, Decimal256>,
}

impl CollateralQuerier {
    pub fn new(collaterals: &[(&String, &Decimal256)]) -> Self {
        CollateralQuerier {
            collaterals: collaterals_to_map(collaterals),
        }
    }
}

pub(crate) fn collaterals_to_map(
    collaterals: &[(&String, &Decimal256)],
) -> HashMap<String, Decimal256> {
    let mut collateral_map: HashMap<String, Decimal256> = HashMap::new();
    for (col, max_ltv) in collaterals.iter() {
        collateral_map.insert((*col).clone(), **max_ltv);
    }
    collateral_map
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

#[derive(Clone, Default)]
pub struct OraclePriceQuerier {
    // this lets us iterate over all pairs that match the first string
    oracle_price: HashMap<String, (Decimal256, i64, Decimal256, i64, u64, u64)>,
}

#[allow(clippy::type_complexity)]
impl OraclePriceQuerier {
    pub fn new(oracle_price: &[(&String, &(Decimal256, i64, Decimal256, i64, u64, u64))]) -> Self {
        OraclePriceQuerier {
            oracle_price: oracle_price_to_map(oracle_price),
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn oracle_price_to_map(
    oracle_price: &[(&String, &(Decimal256, i64, Decimal256, i64, u64, u64))],
) -> HashMap< String, (Decimal256, i64, Decimal256, i64, u64, u64)> {
    let mut oracle_price_map: HashMap< String, (Decimal256, i64, Decimal256, i64, u64, u64)> = HashMap::new();
    for (base_quote, oracle_price) in oracle_price.iter() {
        oracle_price_map.insert((*base_quote).clone(), **oracle_price);
    }
    oracle_price_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<QueryTaxWrapper> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<QueryTaxWrapper>) -> QuerierResult {
        match &request {
                QueryRequest::Custom(QueryTaxWrapper { query_data }) => {
                    match query_data {
                        QueryTaxMsg::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            SystemResult::Ok(ContractResult::from(to_json_binary(&res)))
                        }
                        QueryTaxMsg::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            SystemResult::Ok(ContractResult::from(to_json_binary(&res)))
                        }
                    }
                } 
                QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: _,
                    msg,
                }) => match from_json(msg).unwrap() {
                    QueryMsg::QueryPrice { asset, } => {
                        match self.oracle_price_querier.oracle_price.get(&(asset)) {
                            Some(v) => {
                                SystemResult::Ok(ContractResult::from(to_json_binary(&PriceResponse {
                                    asset,
                                    emv_price: v.0, 
                                    emv_price_raw: v.1,
                                    price: v.2,
                                    price_raw: v.3,
                                    last_updated_base: v.4,
                                    last_updated_quote: v.5,
                                })))
                            }
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No oracle price exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    QueryMsg::Whitelist {
                        collateral_token,
                        start_after: _,
                        limit: _,
                    } => {
                        match self
                            .collateral_querier
                            .collaterals
                            .get(&collateral_token.unwrap())
                        {
                            Some(v) => {
                                SystemResult::Ok(ContractResult::from(to_json_binary(&WhitelistResponse {
                                    elems: vec![WhitelistResponseElem {
                                        name: "name".to_string(),
                                        symbol: "symbol".to_string(),
                                        max_ltv: *v,
                                        custody_contract: "custody0000".to_string(),
                                        collateral_token: "token0000".to_string(),
                                    }],
                                })))
                            }
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                },
                _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<QueryTaxWrapper>) -> Self {
        WasmMockQuerier {
            base,
            tax_querier: TaxQuerier::default(),
            oracle_price_querier: OraclePriceQuerier::default(),
            collateral_querier: CollateralQuerier::default(),
        }
    }

    // configure the tax mock querier
    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    #[allow(clippy::type_complexity)]
    pub fn with_oracle_price(
        &mut self,
        oracle_price: &[(&String, &(Decimal256, i64, Decimal256, i64, u64, u64))],
    ) {
        self.oracle_price_querier = OraclePriceQuerier::new(oracle_price);
    }

    pub fn with_collateral_max_ltv(&mut self, collaterals: &[(&String, &Decimal256)]) {
        self.collateral_querier = CollateralQuerier::new(collaterals);
    }
}
