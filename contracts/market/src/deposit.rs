use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128, WasmMsg,
};

use crate::borrow::{compute_interest, compute_reward};
use crate::error::ContractError;
use crate::state::{read_config, read_state, store_state, Config, State};

use cw20::Cw20ExecuteMsg;
use moneymarket::querier::{deduct_tax, query_balance, query_supply};

pub fn deposit_stable(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Check base denom deposit
    let deposit_amount: Uint256 = info
        .funds
        .iter()
        .find(|c| c.denom == config.stable_denom)
        .map(|c| Uint256::from(c.amount))
        .unwrap_or_else(Uint256::zero);

    // Cannot deposit zero amount
    if deposit_amount.is_zero() {
        return Err(ContractError::ZeroDeposit(config.stable_denom));
    }

    // Update interest related state
    let mut state: State = read_state(deps.storage)?;

    compute_interest(
        deps.as_ref(),
        &config,
        &mut state,
        env.block.height,
        Some(deposit_amount),
    )?;

    compute_reward(&mut state, env.block.height);

    // Load kryptonite token exchange rate with updated state
    let exchange_rate =
        compute_exchange_rate(deps.as_ref(), &config, &state, Some(deposit_amount))?;
    let mint_amount = deposit_amount / exchange_rate;

    state.prev_atoken_supply += mint_amount;
    store_state(deps.storage, &state)?;
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&config.atoken_contract)?.to_string(),
            funds: vec![],
            msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: mint_amount.into(),
            })?,
        }))
        .add_attributes(vec![
            attr("action", "deposit_stable"),
            attr("depositor", info.sender),
            attr("mint_amount", mint_amount),
            attr("deposit_amount", deposit_amount),
            // attr("total_liabilities", state.total_liabilities.to_string()),
            // attr("total_reserves", state.total_reserves.to_string()),
            // attr(
            //     "last_interest_updated",
            //     state.last_interest_updated.to_string(),
            // ),
            // attr("last_reward_updated", state.last_reward_updated.to_string()),
            // attr(
            //     "global_interest_index",
            //     state.global_interest_index.to_string(),
            // ),
            // attr("global_reward_index", state.global_reward_index.to_string()),
            // attr("kpt_emission_rate", state.kpt_emission_rate.to_string()),
            // attr("prev_atoken_supply", state.prev_atoken_supply.to_string()),
            // attr("prev_exchange_rate", state.prev_exchange_rate.to_string()),
            // attr("contract_balance", state.contract_balance.to_string()),
            // attr(
            //     "effective_deposit_rate",
            //     state.effective_deposit_rate.to_string(),
            // ),
            // attr("target_deposit_rate", state.target_deposit_rate.to_string()),
        ]))
}

pub fn redeem_stable(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    burn_amount: Uint128,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    //let protocol_fee_rate: Decimal256 = Decimal256::from_ratio(5, 1000);
    // let protocol_fee: Decimal256 =
    //     (Decimal256::from_uint256(Uint256::from(burn_amount))) * protocol_fee_rate;
    // Update interest related state
    let mut state: State = read_state(deps.storage)?;
    compute_interest(deps.as_ref(), &config, &mut state, env.block.height, None)?;
    compute_reward(&mut state, env.block.height);

    // Load kryptonite token exchange rate with updated state
    let exchange_rate = compute_exchange_rate(deps.as_ref(), &config, &state, None)?;
    let redeem_amount = Uint256::from(burn_amount) * exchange_rate;

    let current_balance = query_balance(
        deps.as_ref(),
        env.contract.address,
        config.stable_denom.to_string(),
    )?;
    // let current_balance: Uint256 = query_token_balance(
    //     deps.as_ref(),
    //     deps.as_ref().api.addr_humanize(&config.stable_contract)?,
    //     deps.as_ref().api.addr_humanize(&config.contract_addr)?,
    // )?;
    // Assert redeem amount
    assert_redeem_amount(&config, &state, current_balance, redeem_amount)?;

    state.prev_atoken_supply = state.prev_atoken_supply - Uint256::from(burn_amount);
    store_state(deps.storage, &state)?;
    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.addr_humanize(&config.atoken_contract)?.to_string(),
                funds: vec![],
                msg: to_json_binary(&Cw20ExecuteMsg::Burn {
                    amount: burn_amount,
                })?,
            }),
            // CosmosMsg::Wasm(WasmMsg::Execute {
            //     contract_addr: deps.api.addr_humanize(&config.stable_contract)?.to_string(),
            //     funds: vec![],
            //     msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            //         recipient: sender.to_string(),
            //         amount: redeem_amount.into(),
            //     })?,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.to_string(),
                amount: vec![deduct_tax(
                    deps.as_ref(),
                    Coin {
                        denom: config.stable_denom,
                        amount: redeem_amount.into(),
                    },
                )?],
            }),
        ])
        .add_attributes(vec![
            attr("action", "redeem_stable"),
            attr("burn_amount", burn_amount),
            attr("redeem_amount", redeem_amount),
        ]))
}

fn assert_redeem_amount(
    config: &Config,
    state: &State,
    current_balance: Uint256,
    redeem_amount: Uint256,
) -> Result<(), ContractError> {
    let current_balance = Decimal256::from_uint256(current_balance);
    let redeem_amount = Decimal256::from_uint256(redeem_amount);
    if redeem_amount + state.total_reserves > current_balance {
        return Err(ContractError::NoStableAvailable(
            config.stable_denom.clone(),
        ));
    }
    Ok(())
}

pub(crate) fn compute_exchange_rate(
    deps: Deps,
    config: &Config,
    state: &State,
    deposit_amount: Option<Uint256>,
) -> StdResult<Decimal256> {
    let atoken_supply = query_supply(deps, deps.api.addr_humanize(&config.atoken_contract)?)?;

    let balance = query_balance(
        deps,
        deps.api.addr_humanize(&config.contract_addr)?,
        config.stable_denom.to_string(),
    )? - deposit_amount.unwrap_or_else(Uint256::zero);

    // let balance: Uint256 = query_token_balance(
    //     deps,
    //     deps.api.addr_humanize(&config.stable_contract)?,
    //     deps.api.addr_humanize(&config.contract_addr)?,
    // )? - deposit_amount.unwrap_or_else(Uint256::zero);
    Ok(compute_exchange_rate_raw(state, atoken_supply, balance))
}

pub fn compute_exchange_rate_raw(
    state: &State,
    atoken_supply: Uint256,
    contract_balance: Uint256,
) -> Decimal256 {
    if atoken_supply.is_zero() {
        return Decimal256::one();
    }
    // (aterra / stable_denom)
    // exchange_rate = (balance + total_liabilities - total_reserves) / atoken_supply
    (Decimal256::from_uint256(contract_balance) + state.total_liabilities - state.total_reserves)
        / Decimal256::from_uint256(atoken_supply)
}
