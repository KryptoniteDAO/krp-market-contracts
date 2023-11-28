use crate::handler::{change_owner, change_pyth_contract, config_feed_info, set_config_feed_valid};
use crate::querier::{query_config, query_pyth_feeder_config};
use crate::testing::mock_fn::{mock_instantiate, mock_instantiate_msg, CREATOR, PYTH_CONTRACT};
use cosmwasm_std::testing::mock_info;
use pyth_sdk_cw::PriceIdentifier;

#[test]
fn test_instantiate() {
    let msg = mock_instantiate_msg(&PYTH_CONTRACT.clone().to_string());
    let (mut deps, _env, info, res) = mock_instantiate(msg);
    assert!(res.is_ok());
    let config = query_config(deps.as_ref()).unwrap();
    assert_eq!(config.pyth_contract, PYTH_CONTRACT.clone().to_string());
    assert_eq!(config.owner, CREATOR.clone().to_string());

    //change owner and pyth contract
    let res = change_owner(deps.as_mut(), info.clone(), "new_owner".to_string());
    assert!(res.is_ok());
    let res = change_pyth_contract(deps.as_mut(), info.clone(), "new_pyth_contract".to_string());
    assert!(res.is_err());
    let new_info = mock_info("new_owner", &[]);
    let res = change_pyth_contract(
        deps.as_mut(),
        new_info.clone(),
        "new_pyth_contract".to_string(),
    );
    assert!(res.is_ok());
    let config = query_config(deps.as_ref()).unwrap();
    assert_eq!(config.pyth_contract, "new_pyth_contract".to_string());
    assert_eq!(config.owner, "new_owner".to_string());
}

#[test]
fn test_update_config_feed_info() {
    let msg = mock_instantiate_msg(&PYTH_CONTRACT.clone().to_string());
    let (mut deps, _env, info, res) = mock_instantiate(msg);
    assert!(res.is_ok());

    let asset = "factory/sei1h3ukufh4lhacftdf6kyxzum4p86rcnel35v4jk/usdt".to_string();
    // config feed info
    let price_feed_id =
        "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string();
    let price_feed_symbol = "Crypto.ETH/USD";
    let price_feed_decimal = 8;
    let price_feed_age = 360;
    let check_feed_age = true;
    let price_feed_id_type = PriceIdentifier::from_hex(price_feed_id).unwrap();
    let config_feed_info_res = config_feed_info(
        deps.as_mut(),
        info.clone(),
        asset.clone(),
        price_feed_id_type.clone(),
        price_feed_symbol.clone().to_string(),
        price_feed_decimal.clone(),
        check_feed_age.clone(),
        price_feed_age.clone(),
    );

    assert!(config_feed_info_res.is_ok());

    let feeder_config = query_pyth_feeder_config(deps.as_ref(), asset.clone()).unwrap();
    assert_eq!(feeder_config.check_feed_age, check_feed_age);
    assert_eq!(feeder_config.price_feed_age, price_feed_age);
    assert_eq!(feeder_config.price_feed_decimal, price_feed_decimal);
    assert_eq!(feeder_config.price_feed_id, price_feed_id_type);
    assert_eq!(feeder_config.price_feed_symbol, price_feed_symbol);
    assert_eq!(feeder_config.is_valid, true);

    // set config feed valid
    let res = set_config_feed_valid(deps.as_mut(), info.clone(), asset.clone(), false);
    assert!(res.is_ok());
    let feeder_config = query_pyth_feeder_config(deps.as_ref(), asset.clone()).unwrap();
    assert_eq!(feeder_config.is_valid, false);

    // change owner
    let res = change_owner(deps.as_mut(), info.clone(), "new_owner".to_string());
    assert!(res.is_ok());

    let res = set_config_feed_valid(deps.as_mut(), info.clone(), asset.clone(), false);
    assert!(res.is_err());
    let config_feed_info_res = config_feed_info(
        deps.as_mut(),
        info.clone(),
        asset.clone(),
        price_feed_id_type.clone(),
        price_feed_symbol.clone().to_string(),
        price_feed_decimal.clone(),
        check_feed_age.clone(),
        price_feed_age.clone(),
    );
    assert!(config_feed_info_res.is_err());
}
