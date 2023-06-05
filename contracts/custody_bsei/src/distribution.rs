use std::ops::Div;

use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryRequest, ReplyOn, Response, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};

use crate::contract::CLAIM_REWARDS_OPERATION;
use crate::contract::SWAP_TO_STABLE_OPERATION;
use crate::contract::SWAP_TO_MARKET_STABLE_OPERATION;
use crate::error::ContractError;
use crate::external::handle::{RewardContractExecuteMsg, RewardContractQueryMsg};
use crate::state::{read_config, BSeiAccruedRewardsResponse, Config};
use cosmwasm_bignumber::Decimal256;

use moneymarket::oracle::PriceResponse;
use moneymarket::querier::{
    deduct_tax, query_balance, query_market_list, query_market_state, query_overseer_config,
    query_price,
};
use moneymarket::swap_ext::SwapExecteMsg;
// REWARD_THRESHOLD
// This value is used as the minimum reward claim amount
// thus if a user's reward is less than 1 ust do not send the ClaimRewards msg
const REWARDS_THRESHOLD: Uint128 = Uint128::new(1000000);

/// Request withdraw reward operation to
/// reward contract and execute `distribute_hook`
/// Executor: overseer
pub fn distribute_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if config.overseer_contract != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let contract_addr = env.contract.address;
    let reward_contract = deps.api.addr_humanize(&config.reward_contract)?;

    let accrued_rewards =
        get_accrued_rewards(deps.as_ref(), reward_contract.clone(), contract_addr)?;
    if accrued_rewards < REWARDS_THRESHOLD {
        return Ok(Response::default());
    }

    // Do not emit the event logs here
    Ok(
        Response::new().add_submessages(vec![SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: reward_contract.to_string(),
                funds: vec![],
                msg: to_binary(&RewardContractExecuteMsg::ClaimRewards { recipient: None })?,
            }),
            CLAIM_REWARDS_OPERATION,
        )]),
    )
}

/// Apply swapped reward to global index
/// Executor: itself
pub fn distribute_hook(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let contract_addr = env.contract.address;
    let config: Config = read_config(deps.storage)?;
    let overseer_contract = deps.api.addr_humanize(&config.overseer_contract)?;

    // reward_amount = (prev_balance + reward_amount) - prev_balance
    // = (0 + reward_amount) - 0 = reward_amount = balance
    let balances = deps.querier.query_all_balances(contract_addr.clone())?;

    let resp = Response::new();
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut attrs: Vec<Attribute> = Vec::new();
    for coin in balances {
        if !coin.amount.is_zero() {
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: overseer_contract.to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: coin.denom.clone(),
                        amount: coin.amount.into(),
                    },
                )?],
            }));
        }

        attrs.push(attr("buffer_rewards_denom", coin.denom.clone()));
        attrs.push(attr("buffer_rewards_amount", coin.amount));
    }

    //Ok(resp.add_messages(messages).add_attributes(attrs))
    Ok(resp.add_attribute("action", "distribute_hook").add_attributes(attrs))
}

/// Swap all coins to stable_denom
/// and execute `swap_hook`
/// Executor: itself
pub fn swap_to_stable_denom(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    let contract_addr = env.contract.address.clone();
    // --------------------- add swap start --------------------------
    let balances = deps.querier.query_all_balances(contract_addr)?;
    let reward_denom = config.stable_denom.clone();
    let swap_addr = deps.api.addr_humanize(&config.swap_contract)?;
   

    let mut messages: Vec<SubMsg> = Vec::new();
    for coin in balances {
        if coin.denom == config.stable_denom {
            continue;
        }

        if coin.amount > Uint128::zero() {
            let swap_msg = SwapExecteMsg::SwapDenom {
                from_coin: coin.clone(),
                target_denom: reward_denom.clone(),
                to_address: Option::from(env.contract.address.to_string()),
            };
            messages.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: swap_addr.clone().to_string(),
                msg: to_binary(&swap_msg)?,
                funds: vec![coin.clone()],
            })));
        }
    }
    // --------------------- add swap end --------------------------

    if let Some(last) = messages.last_mut() {
        last.id = SWAP_TO_STABLE_OPERATION;
        last.reply_on = ReplyOn::Success;
    } else {
        return swap_to_market_stable_denom(deps, env);
    }

    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "swap_to_stable_denom"))
}

/// swap custody config stable to market stable
/// send maket stable denom to overseer contract
pub(crate) fn swap_to_market_stable_denom(
    deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let overseer_config = query_overseer_config(deps.as_ref(), config.overseer_contract.clone())?;
    // query marketlist in overseer contract
    let market_list = query_market_list(deps.as_ref(), config.overseer_contract.clone())?;
    let swap_addr = deps.api.addr_humanize(&config.swap_contract)?;

    let mut total_markets_loan_value = Decimal256::zero();
    let mut market_loan_value: Vec<(String, Decimal256)> = Vec::new();

    // query each market's loan and computer the value
    for elem in market_list.elems {
        let market_contract = deps.api.addr_canonicalize(elem.market_contract.as_str())?;
        let market_state = query_market_state(deps.as_ref(), market_contract)?;

        let price: PriceResponse = query_price(
            deps.as_ref(),
            deps.api
                .addr_validate(overseer_config.oracle_contract.clone().as_str())?,
            elem.stable_denom.to_string(),
            "".to_string(),
            None,
        )?;

        let loan_value = market_state.total_liabilities * price.rate;
        total_markets_loan_value += loan_value;
        market_loan_value.push((elem.stable_denom, loan_value));
    }
    let reward_denom = config.stable_denom.clone();
    let contract_addr = env.contract.address.clone();
    let balances = query_balance(deps.as_ref(), contract_addr.clone(), config.stable_denom)?;

    // Calculate the loan weight of each market, replace the stable coin configured
    // in the custody with the market stable coin according to this weight,
    // and send it to the overseer contract.
    let mut messages: Vec<SubMsg> = vec![];
    for loan_value_item in market_loan_value {
        if reward_denom.clone() == loan_value_item.0.clone() {
            continue;
        }

        let reward_coin_convert = Decimal256::from_uint256(balances)
            * loan_value_item.1.div(total_markets_loan_value)
            * Uint256::one();
        let coin_convert = Coin {
            amount: reward_coin_convert.into(),
            denom: reward_denom.clone(),
        };
        let swap_msg = SwapExecteMsg::SwapDenom {
            from_coin: coin_convert.clone(),
            target_denom: loan_value_item.0.clone(),
            to_address: Option::from(contract_addr.clone().to_string()),
        };

        messages.push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap_addr.clone().to_string(),
            msg: to_binary(&swap_msg)?,
            funds: vec![coin_convert.clone()],
        })));
    }

    if let Some(last) = messages.last_mut() {
        last.id = SWAP_TO_MARKET_STABLE_OPERATION;
        last.reply_on = ReplyOn::Success;
    } else {
        return distribute_hook(deps, env);
    }
    
    let mut attris : Vec<Attribute> = Vec::new();
    for msg in &messages {
        let json = to_binary(&msg).unwrap();
        attris.push(attr(&msg.id.to_string(), String::from_utf8_lossy(&json)));
    }

    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "swap_to_market_stable_denom")
        .add_attributes(attris))

}

pub(crate) fn get_accrued_rewards(
    deps: Deps,
    reward_contract_addr: Addr,
    contract_addr: Addr,
) -> StdResult<Uint128> {
    let rewards: BSeiAccruedRewardsResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: reward_contract_addr.to_string(),
            msg: to_binary(&RewardContractQueryMsg::AccruedRewards {
                address: contract_addr.to_string(),
            })?,
        }))?;

    Ok(rewards.rewards)
}
