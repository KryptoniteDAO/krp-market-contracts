use cosmwasm_bignumber::Decimal256;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use pyth_sdk_cw::PriceIdentifier;

#[cw_serde]
pub struct InstantiateMsg {
    pub pyth_contract: String,
    pub owner: Addr,
}

#[cw_serde]
pub struct SetConfigFeedValidMsg {
    pub asset_address: String,
    pub valid: bool,
}

#[cw_serde]
pub struct ChangeOwnerMsg {
    pub new_owner: String,
}

#[cw_serde]
pub struct ChangePythContract {
    pub new_contract: String,
}

#[cw_serde]
pub struct PriceResponse {
    pub asset: String,
    pub emv_price: Decimal256,
    pub emv_price_raw: i64,
    pub price: Decimal256,
    pub price_raw: i64,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

#[cw_serde]
pub struct PythFeederConfigResponse {
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub price_feed_age: u64,
    pub check_feed_age: bool,
    pub is_valid: bool,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub pyth_contract: String,
}

pub struct ConfigFeedInfoParams {
    pub asset_address: String,
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub price_feed_age: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    ConfigFeedInfo {
        asset: String,
        price_feed_id: String,
        price_feed_symbol: String,
        price_feed_decimal: u32,
        check_feed_age: bool,
        price_feed_age: u64,
    },

    SetConfigFeedValid {
        asset: String,
        valid: bool,
    },
    ChangeOwner {
        new_owner: String,
    },
    ChangePythContract {
        pyth_contract: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PriceResponse)]
    QueryPrice { asset: String },
    #[returns(Vec < PriceResponse >)]
    QueryPrices { assets: Vec<String> },
    #[returns(ConfigResponse)]
    QueryConfig {},
    #[returns(PythFeederConfigResponse)]
    QueryPythFeederConfig { asset: String },
    #[returns(Decimal256)]
    QueryExchangeRateByAssetLabel {
        base_label: String,
        quote_label: String,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
