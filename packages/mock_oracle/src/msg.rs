use cosmwasm_schema::{cw_serde, QueryResponses};
use pyth_sdk_cw::PriceIdentifier;

#[cw_serde]
pub struct InstantiateMsg {}
#[cw_serde]
pub enum ExecuteMsg {
    UpdatePriceFeed { id: PriceIdentifier, price: i64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PriceFeedResponse)]
    PriceFeed { id: PriceIdentifier },
}

#[cw_serde]
pub struct PriceFeedResponse {
    pub price_feed: PriceFeed,
}

#[cw_serde]
pub struct PriceFeed {
    /// Unique identifier for this price.
    pub id: PriceIdentifier,
    /// Price.
    pub price: Price,
    /// Exponentially-weighted moving average (EMA) price.
    pub ema_price: Price,
}

#[cw_serde]
pub struct Price {
    /// Price.
    pub price: String,
    /// Confidence interval.
    pub conf: String,
    /// Exponent.
    pub expo: i32,
    /// Publish time.
    pub publish_time: UnixTimestamp,
}

pub type UnixTimestamp = i64;
