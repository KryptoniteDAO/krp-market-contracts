use crate::msg::{ConfigResponse, PriceResponse, PythFeederConfigResponse};
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{Deps, Env, StdError, StdResult};
use pyth_sdk_cw::{query_price_feed, Price, PriceFeedResponse};
use std::ops::Div;

use crate::error::ContractError;
use crate::state::{read_config, read_pyth_feeder_config, Config, PythFeederConfig};
use bigint::uint::U256;

/**
 * Query the config of the oracle
 */
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        pyth_contract: deps.api.addr_humanize(&config.pyth_contract)?.to_string(),
    })
}

/**
 * Query the feeder config of the asset
 */
pub fn query_pyth_feeder_config(deps: Deps, asset: String) -> StdResult<PythFeederConfigResponse> {
    let pyth_feeder_config: PythFeederConfig =
        read_pyth_feeder_config(deps.storage, asset.clone())?;
    Ok(PythFeederConfigResponse {
        price_feed_id: pyth_feeder_config.price_feed_id,
        price_feed_symbol: pyth_feeder_config.price_feed_symbol.to_string(),
        price_feed_decimal: pyth_feeder_config.price_feed_decimal,
        price_feed_age: pyth_feeder_config.price_feed_age,
        check_feed_age: pyth_feeder_config.check_feed_age,
        is_valid: pyth_feeder_config.is_valid,
    })
}

/**
 * Query the price of the asset
 */
pub fn query_price(deps: Deps, env: Env, asset: String) -> StdResult<PriceResponse> {
    let config: Config = read_config(deps.storage)?;

    let pyth_feeder_config: PythFeederConfig =
        read_pyth_feeder_config(deps.storage, asset.clone())?;
    if !pyth_feeder_config.is_valid {
        return Err(StdError::generic_err("Asset is not valid"));
    }

    let pyth_contract = deps.api.addr_humanize(&config.pyth_contract)?;

    // query_price_feed is the standard way to read the current price from a Pyth price feed.
    // It takes the address of the Pyth contract (which is fixed for each network) and the id of the
    // price feed. The result is a PriceFeed object with fields for the current price and other
    // useful information. The function will fail if the contract address or price feed id are
    // invalid.
    let price_feed_response: PriceFeedResponse = query_price_feed(
        &deps.querier,
        pyth_contract,
        pyth_feeder_config.price_feed_id,
    )?;
    let price_feed = price_feed_response.price_feed;

    // Get an exponentially-weighted moving average price and confidence interval.
    // The same notes about availability apply to this price.
    // let ema_price = price_feed.get_ema_price_unchecked();
    let ema_price: Price;
    let current_price: Price;
    if pyth_feeder_config.check_feed_age {
        ema_price = price_feed
            .get_ema_price_no_older_than(
                env.block.time.seconds() as i64,
                pyth_feeder_config.price_feed_age,
            )
            .ok_or_else(|| ContractError::Std(StdError::not_found("EMA price is not available")))?;
        current_price = price_feed
            .get_price_no_older_than(
                env.block.time.seconds() as i64,
                pyth_feeder_config.price_feed_age,
            )
            .ok_or_else(|| {
                ContractError::Std(StdError::not_found("Current price is not available"))
            })?;
    } else {
        ema_price = price_feed.get_ema_price_unchecked();
        current_price = price_feed.get_price_unchecked();
    }

    let decimal: u32 = pyth_feeder_config.price_feed_decimal;
    let decimal_places =
        Decimal256::from_ratio(U256::from(1u64), U256::from(10u64.pow(decimal as u32)));
    let evm_price_decimal = Decimal256::from_ratio(ema_price.price, 1) * decimal_places;
    let current_price_decimal = Decimal256::from_ratio(current_price.price, 1) * decimal_places;

    let feed_time_u64: u64 = ema_price
        .publish_time
        .try_into()
        .map_err(|_| ContractError::Std(StdError::generic_err("Failed to convert i64 to u64")))?;
    Ok(PriceResponse {
        asset: asset.clone(),
        emv_price: evm_price_decimal,
        emv_price_raw: ema_price.price,
        price: current_price_decimal,
        price_raw: current_price.price,
        last_updated_base: feed_time_u64,
        last_updated_quote: feed_time_u64,
    })
}

/**
 * Query the prices of the given assets
 */
pub fn query_prices(deps: Deps, env: Env, assets: Vec<String>) -> StdResult<Vec<PriceResponse>> {
    let mut prices = Vec::new();
    for asset in assets {
        let price = query_price(deps.clone(), env.clone(), asset)?;
        prices.push(price);
    }
    Ok(prices)
}

pub fn query_exchange_rate_by_asset_label(
    deps: Deps,
    env: Env,
    base_label: String,
    quote_label: String,
) -> StdResult<Decimal256> {
    let base_price = query_price(deps.clone(), env.clone(), base_label)?;
    let quote_price = query_price(deps.clone(), env.clone(), quote_label)?;
    Ok(base_price.emv_price.div(quote_price.emv_price))
}

impl From<ContractError> for StdError {
    fn from(error: ContractError) -> Self {
        StdError::generic_err(error.to_string())
    }
}
