use cosmwasm_std::{Binary,entry_point, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use crate::error::ContractError;
use crate::handler::{change_owner, config_feed_info, set_config_feed_valid};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::querier::{query_config, query_price, query_pyth_feeder_config};
use crate::state::{Config, store_config};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(info.sender.as_str())?,
            pyth_contract: deps.api.addr_canonicalize(&msg.pyth_contract)?,
        },
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    match msg {
        ExecuteMsg::ConfigFeedInfo { asset_address, price_feed_id, price_feed_symbol, price_feed_decimal, price_feed_age }
        => config_feed_info(deps, info, asset_address, price_feed_id, price_feed_symbol, price_feed_decimal, price_feed_age),
        ExecuteMsg::SetConfigFeedValid { asset_address, valid } => set_config_feed_valid(deps, info, asset_address, valid),
        ExecuteMsg::ChangeOwner { new_owner } => change_owner(deps, info, new_owner),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price { base, quote:_ } => to_binary(&query_price(deps, env, base)?),
        QueryMsg::QueryConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::QueryPythFeederConfig { asset_address } => to_binary(&query_pyth_feeder_config(deps, asset_address)?),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn proper_initialization() {}
}
