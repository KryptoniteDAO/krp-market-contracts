use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_bignumber::Decimal256;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub pyth_contract: String,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PriceResponse {
    pub asset: String,
    pub emv_price: Decimal256,
    pub emv_price_raw: i64,
    pub price: Decimal256,
    pub price_raw: i64,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryPrice {
        asset: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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
