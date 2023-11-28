use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{Addr, Env, MessageInfo, OwnedDeps, Response, StdResult};

pub const CREATOR: &str = "creator";
pub const PYTH_CONTRACT: &str = "pyth_contract";

pub fn mock_instantiate_msg(pyth_contract: &String) -> InstantiateMsg {
    InstantiateMsg {
        pyth_contract: pyth_contract.clone(),
        owner: Addr::unchecked(CREATOR),
    }
}

pub fn mock_instantiate(
    msg: InstantiateMsg,
) -> (
    OwnedDeps<MockStorage, MockApi, MockQuerier>,
    Env,
    MessageInfo,
    StdResult<Response>,
) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(CREATOR, &[]);

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);

    (deps, env, info, res)
}
