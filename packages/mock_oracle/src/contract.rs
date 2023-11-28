use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{read_oracle_price, store_oracle_price};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use pyth_sdk::{Price, PriceFeed};
use pyth_sdk_cw::{PriceFeedResponse, PriceIdentifier};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdatePriceFeed { id, price } => update_price_feed(deps, id, price),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PriceFeed { id } => to_json_binary(&query_price_feed(&deps, env, id)?),
    }
}

pub fn update_price_feed(deps: DepsMut, id: PriceIdentifier, price: i64) -> StdResult<Response> {
    store_oracle_price(deps.storage, id.clone().as_ref(), price.clone())?;
    Ok(Response::new().add_attribute("price", price.to_string()))
}

/// Get the most recent value of the price feed indicated by `feed_id`.
pub fn query_price_feed(
    deps: &Deps,
    env: Env,
    feed_id: PriceIdentifier,
) -> StdResult<PriceFeedResponse> {
    let price_db = read_oracle_price(deps.storage, feed_id.clone().as_ref())?;

    let price = Price {
        price: price_db,
        conf: 0u64,
        expo: 0,
        publish_time: env.block.time.seconds() as i64,
    };

    let ema_price = Price {
        price: price_db,
        conf: 0u64,
        expo: 0,
        publish_time: env.block.time.seconds() as i64,
    };

    let price_feed = PriceFeed::new(feed_id.clone(), price, ema_price);

    Ok(PriceFeedResponse { price_feed })
}
