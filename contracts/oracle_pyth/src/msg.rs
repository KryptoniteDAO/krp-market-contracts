use pyth_sdk_cw::PriceIdentifier;
use cosmwasm_bignumber::Decimal256;
use cosmwasm_schema::{cw_serde};

#[cw_serde]
pub struct InstantiateMsg {
    pub pyth_contract: String,
}


#[cw_serde]
pub struct ConfigFeedMsg {
    pub asset_address: String,
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub price_feed_age: u64,
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
pub struct PriceResponse {
    pub rate: Decimal256,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

#[cw_serde]
pub struct PythFeederConfigResponse {
    pub price_feed_id: PriceIdentifier,
    pub price_feed_symbol: String,
    pub price_feed_decimal: u32,
    pub price_feed_age: u64,
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
        asset_address: String,
        price_feed_id: PriceIdentifier,
        price_feed_symbol: String,
        price_feed_decimal: u32,
        price_feed_age: u64,
    },

    SetConfigFeedValid {
        asset_address: String,
        valid: bool,
    },
    ChangeOwner {
        new_owner: String,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Price {
        base: String,
        quote: Option<String>,
    },
    QueryConfig {},
    QueryPythFeederConfig {
        asset_address: String,
    },
}