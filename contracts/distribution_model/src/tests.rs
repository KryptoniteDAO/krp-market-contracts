use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::from_json;
use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
use moneymarket::distribution_model::{
    KptEmissionRateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies_with_balance(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        emission_cap: Decimal256::from_uint256(100u64),
        emission_floor: Decimal256::from_uint256(10u64),
        increment_multiplier: Decimal256::percent(110),
        decrement_multiplier: Decimal256::percent(90),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_json(&res).unwrap();
    assert_eq!("owner0000", value.owner.as_str());
    assert_eq!("100", &value.emission_cap.to_string());
    assert_eq!("10", &value.emission_floor.to_string());
    assert_eq!("1.1", &value.increment_multiplier.to_string());
    assert_eq!("0.9", &value.decrement_multiplier.to_string());
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies_with_balance(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        emission_cap: Decimal256::from_uint256(100u64),
        emission_floor: Decimal256::from_uint256(10u64),
        increment_multiplier: Decimal256::percent(110),
        decrement_multiplier: Decimal256::percent(90),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        emission_cap: None,
        emission_floor: None,
        increment_multiplier: None,
        decrement_multiplier: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    
    let msg = ExecuteMsg::SetOwner {
        new_owner_addr: "owner0001".to_string(),
    };
    let info = mock_info("owner0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let msg = ExecuteMsg::AcceptOwnership {};
    let info = mock_info("owner0001", &[]);
    execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();


    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_json(&res).unwrap();
    assert_eq!("owner0001", value.owner.as_str());
    assert_eq!("100", &value.emission_cap.to_string());
    assert_eq!("10", &value.emission_floor.to_string());
    assert_eq!("1.1", &value.increment_multiplier.to_string());
    assert_eq!("0.9", &value.decrement_multiplier.to_string());

    // Unauthorized err
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        emission_cap: Some(Decimal256::from_uint256(100u64)),
        emission_floor: Some(Decimal256::from_uint256(10u64)),
        increment_multiplier: Some(Decimal256::percent(110)),
        decrement_multiplier: Some(Decimal256::percent(90)),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(ContractError::Unauthorized {}) => (),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn proper_emission_rate() {
    let mut deps = mock_dependencies_with_balance(&[]);

    let msg = InstantiateMsg {
        owner: "owner0000".to_string(),
        emission_cap: Decimal256::from_uint256(100u64),
        emission_floor: Decimal256::from_uint256(10u64),
        increment_multiplier: Decimal256::percent(110),
        decrement_multiplier: Decimal256::percent(90),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // high = 8.75
    // low = 6.25
    // no changes
    let query_msg = QueryMsg::KptEmissionRate {
        deposit_rate: Decimal256::percent(7),
        target_deposit_rate: Decimal256::percent(10),
        threshold_deposit_rate: Decimal256::percent(5),
        current_emission_rate: Decimal256::from_uint256(99u128),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: KptEmissionRateResponse = from_json(&res).unwrap();
    assert_eq!("99", &value.emission_rate.to_string());

    // increment
    let query_msg = QueryMsg::KptEmissionRate {
        deposit_rate: Decimal256::percent(5),
        target_deposit_rate: Decimal256::percent(10),
        threshold_deposit_rate: Decimal256::percent(5),
        current_emission_rate: Decimal256::from_uint256(80u128),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: KptEmissionRateResponse = from_json(&res).unwrap();
    assert_eq!("88", &value.emission_rate.to_string());

    // cap
    let query_msg = QueryMsg::KptEmissionRate {
        deposit_rate: Decimal256::percent(5),
        target_deposit_rate: Decimal256::percent(10),
        threshold_deposit_rate: Decimal256::percent(5),
        current_emission_rate: Decimal256::from_uint256(99u128),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: KptEmissionRateResponse = from_json(&res).unwrap();
    assert_eq!("100", &value.emission_rate.to_string());

    // decrement
    let query_msg = QueryMsg::KptEmissionRate {
        deposit_rate: Decimal256::percent(9),
        target_deposit_rate: Decimal256::percent(10),
        threshold_deposit_rate: Decimal256::percent(5),
        current_emission_rate: Decimal256::from_uint256(99u128),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: KptEmissionRateResponse = from_json(&res).unwrap();
    assert_eq!("89.1", &value.emission_rate.to_string());

    // floor
    let query_msg = QueryMsg::KptEmissionRate {
        deposit_rate: Decimal256::percent(9),
        target_deposit_rate: Decimal256::percent(10),
        threshold_deposit_rate: Decimal256::percent(5),
        current_emission_rate: Decimal256::from_uint256(11u128),
    };
    let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
    let value: KptEmissionRateResponse = from_json(&res).unwrap();
    assert_eq!("10", &value.emission_rate.to_string());
}
