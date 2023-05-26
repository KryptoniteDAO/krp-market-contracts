use crate::error::ContractError;
use crate::handler::{change_owner, change_pyth_contract, config_feed_info, set_config_feed_valid};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::querier::{query_config, query_exchange_rate_by_asset_label, query_price, query_prices, query_pyth_feeder_config};
use crate::state::{store_config, Config};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use pyth_sdk_cw::PriceIdentifier;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.addr_canonicalize(msg.owner.as_str())?,
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
        ExecuteMsg::ConfigFeedInfo {
            asset,
            price_feed_id,
            price_feed_symbol,
            price_feed_decimal,
            check_feed_age,
            price_feed_age,
        } => {
            let price_feed_id_type = PriceIdentifier::from_hex(price_feed_id).unwrap();
            config_feed_info(
                deps,
                info,
                asset,
                price_feed_id_type,
                price_feed_symbol,
                price_feed_decimal,
                check_feed_age,
                price_feed_age,
            )
        }
        ExecuteMsg::SetConfigFeedValid { asset, valid } => {
            set_config_feed_valid(deps, info, asset, valid)
        }
        ExecuteMsg::ChangeOwner { new_owner } => change_owner(deps, info, new_owner),
        ExecuteMsg::ChangePythContract { pyth_contract } => change_pyth_contract(deps, info, pyth_contract),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryPrice { asset } => to_binary(&query_price(deps, env, asset)?),
        QueryMsg::QueryPrices { assets } => to_binary(&query_prices(deps, env, assets)?),
        QueryMsg::QueryConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::QueryPythFeederConfig { asset } => {
            to_binary(&query_pyth_feeder_config(deps, asset)?)
        },
        QueryMsg::QueryExchangeRateByAssetLabel {base_label, quote_label} => {
            to_binary(&query_exchange_rate_by_asset_label(deps, env,base_label, quote_label)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
#[cfg(test)]
mod tests {

    #[test]
    fn proper_initialization() {}
}
