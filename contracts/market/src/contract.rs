#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::borrow::{
    borrow_stable, claim_rewards, compute_interest, compute_interest_raw, compute_reward,
    query_borrower_info, query_borrower_infos, repay_stable, repay_stable_from_liquidation,
};
use crate::deposit::{compute_exchange_rate_raw, deposit_stable, redeem_stable};
use crate::error::ContractError;
use crate::querier::{query_borrow_rate, query_target_deposit_rate};
use crate::response::MsgInstantiateContractResponse;
use crate::state::{read_config, read_state, store_config, store_state, read_new_owner, store_new_owner, Config, State, NewOwnerAddr};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, BankMsg, Binary, CanonicalAddr, Coin, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ReceiveMsg, MinterResponse};

use moneymarket::common::optional_addr_validate;
use moneymarket::interest_model::BorrowRateResponse;
use moneymarket::market::{
    ConfigResponse, Cw20HookMsg, EpochStateResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    QueryMsg, StateResponse,
};
use moneymarket::querier::{deduct_tax, query_balance, query_supply};
use moneymarket::terraswap::InstantiateMsg as TokenInstantiateMsg;
use protobuf::Message;

pub const INITIAL_DEPOSIT_AMOUNT: u128 = 1000000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let initial_deposit = info
        .funds
        .iter()
        .find(|c| c.denom == msg.stable_denom)
        .map(|c| c.amount)
        .unwrap_or_else(Uint128::zero);

    if initial_deposit != Uint128::from(INITIAL_DEPOSIT_AMOUNT) {
        return Err(ContractError::InitialFundsNotDeposited(
            INITIAL_DEPOSIT_AMOUNT,
            msg.stable_denom,
        ));
    }

    store_config(
        deps.storage,
        &Config {
            contract_addr: deps.api.addr_canonicalize(env.contract.address.as_str())?,
            owner_addr: deps.api.addr_canonicalize(&msg.owner_addr)?,
            atoken_contract: CanonicalAddr::from(vec![]),
            overseer_contract: CanonicalAddr::from(vec![]),
            interest_model: CanonicalAddr::from(vec![]),
            distribution_model: CanonicalAddr::from(vec![]),
            collector_contract: CanonicalAddr::from(vec![]),
            distributor_contract: CanonicalAddr::from(vec![]),
            stable_denom: msg.stable_denom.clone(),
            max_borrow_factor: msg.max_borrow_factor,
        },
    )?;

    store_state(
        deps.storage,
        &State {
            total_liabilities: Decimal256::zero(),
            total_reserves: Decimal256::zero(),
            last_interest_updated: env.block.height,
            last_reward_updated: env.block.height,
            global_interest_index: Decimal256::one(),
            global_reward_index: Decimal256::zero(),
            kpt_emission_rate: msg.kpt_emission_rate,
            prev_atoken_supply: Uint256::zero(),
            prev_exchange_rate: Decimal256::one(),
        },
    )?;

    let mut messages: Vec<SubMsg> = vec![];

    let inst_aust_msg: SubMsg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: None,
            code_id: msg.atoken_code_id,
            funds: vec![],
            label: "Kryptonite stable coin share".to_string(),
            msg: to_binary(&TokenInstantiateMsg {
                //name: format!("Kryptonite stable coin share", msg.stable_denom[1..].to_uppercase()),
                name: "Kryptonite stable coin share".to_string(),
                // symbol: format!(
                //     "a{}T",
                //     msg.stable_denom[1..(msg.stable_denom.len() - 1)].to_uppercase()
                // ),
                symbol: "kUSD".to_string(),
                decimals: 6u8,
                initial_balances: vec![Cw20Coin {
                    address: env.contract.address.to_string(),
                    amount: Uint128::from(INITIAL_DEPOSIT_AMOUNT),
                }],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
            })?,
        }),
        1,
    );
    messages.push(inst_aust_msg);

    Ok(Response::new().add_submessages(messages))
   
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::RegisterContracts {
            overseer_contract,
            interest_model,
            distribution_model,
            collector_contract,
            distributor_contract,
        } => {
            let api = deps.api;
            register_contracts(
                deps,
                api.addr_validate(&overseer_contract)?,
                api.addr_validate(&interest_model)?,
                api.addr_validate(&distribution_model)?,
                api.addr_validate(&collector_contract)?,
                api.addr_validate(&distributor_contract)?,
            )
        }
        ExecuteMsg::UpdateConfig {
            interest_model,
            distribution_model,
            max_borrow_factor,
        } => {
            let api = deps.api;
            update_config(
                deps,
                env,
                info,
                optional_addr_validate(api, interest_model)?,
                optional_addr_validate(api, distribution_model)?,
                max_borrow_factor,
            )
        }
        ExecuteMsg::SetOwner { new_owner_addr } => {
            let api = deps.api;
            set_new_owner(deps, info, api.addr_validate(&new_owner_addr)?)
        }
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info),
        ExecuteMsg::ExecuteEpochOperations {
            deposit_rate,
            target_deposit_rate,
            threshold_deposit_rate,
            distributed_interest,
        } => execute_epoch_operations(
            deps,
            env,
            info,
            deposit_rate,
            target_deposit_rate,
            threshold_deposit_rate,
            distributed_interest,
        ),
        ExecuteMsg::DepositStable {} => deposit_stable(deps, env, info),
        ExecuteMsg::BorrowStable { borrow_amount, to } => {
            let api = deps.api;
            borrow_stable(
                deps,
                env,
                info,
                borrow_amount,
                optional_addr_validate(api, to)?,
            )
        }

        ExecuteMsg::RepayStable {} => repay_stable(deps, env, info),
        ExecuteMsg::RepayStableFromLiquidation {
            borrower,
            prev_balance,
        } => {
            let api = deps.api;
            repay_stable_from_liquidation(
                deps,
                env,
                info,
                api.addr_validate(&borrower)?,
                prev_balance,
            )
        }
        ExecuteMsg::ClaimRewards { to } => {
            let api = deps.api;
            claim_rewards(deps, env, info, optional_addr_validate(api, to)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        1 => {
            // get new token's contract address
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                msg.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                ContractError::Std(StdError::parse_err(
                    "MsgInstantiateContractResponse",
                    "failed to parse data",
                ))
            })?;
            let token_addr = Addr::unchecked(res.get_contract_address());

            register_atoken(deps, token_addr)
        }
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::RedeemStable {}) => {
            // only asset contract can execute this message
            let config: Config = read_config(deps.storage)?;
            if deps.api.addr_canonicalize(contract_addr.as_str())? != config.atoken_contract {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            redeem_stable(deps, env, cw20_sender_addr, cw20_msg.amount)
        }

        // Ok(Cw20HookMsg::DepositStable {}) => {

        //     let config: Config = read_config(deps.storage)?;

        //     let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;

        //     deposit_stable(deps, env,cw20_sender_addr, Uint256::from(cw20_msg.amount))

        // }

        // Ok(Cw20HookMsg::RepayStable {}) => {
        //     let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;

        //     repay_stable(deps, env, cw20_sender_addr, Uint256::from(cw20_msg.amount))
        // }
        _ => Err(ContractError::MissingRedeemStableHook {}),
    }
}

pub fn register_atoken(deps: DepsMut, token_addr: Addr) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if config.atoken_contract != CanonicalAddr::from(vec![]) {
        return Err(ContractError::Unauthorized {});
    }

    config.atoken_contract = deps.api.addr_canonicalize(token_addr.as_str())?;
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("atoken", token_addr)]))
}

pub fn register_contracts(
    deps: DepsMut,
    overseer_contract: Addr,
    interest_model: Addr,
    distribution_model: Addr,
    collector_contract: Addr,
    distributor_contract: Addr,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;
    if config.overseer_contract != CanonicalAddr::from(vec![])
        || config.interest_model != CanonicalAddr::from(vec![])
        || config.distribution_model != CanonicalAddr::from(vec![])
        || config.collector_contract != CanonicalAddr::from(vec![])
        || config.distributor_contract != CanonicalAddr::from(vec![])
    {
        return Err(ContractError::Unauthorized {});
    }

    config.overseer_contract = deps.api.addr_canonicalize(overseer_contract.as_str())?;
    config.interest_model = deps.api.addr_canonicalize(interest_model.as_str())?;
    config.distribution_model = deps.api.addr_canonicalize(distribution_model.as_str())?;
    config.collector_contract = deps.api.addr_canonicalize(collector_contract.as_str())?;
    config.distributor_contract = deps.api.addr_canonicalize(distributor_contract.as_str())?;

    store_config(deps.storage, &config)?;

    Ok(Response::default())
}


pub fn set_new_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner_addr: Addr,
) -> Result<Response, ContractError> {
    let config = read_config(deps.as_ref().storage)?;
    let mut new_owner = read_new_owner(deps.as_ref().storage)?;
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    if sender_raw != config.owner_addr {
        return Err(ContractError::Unauthorized{});
    }
    new_owner.new_owner_addr = deps.api.addr_canonicalize(&new_owner_addr.to_string())?;
    store_new_owner(deps.storage, &new_owner)?;

    Ok(Response::default())
}

pub fn accept_ownership(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let new_owner = read_new_owner(deps.as_ref().storage)?;
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    let mut config = read_config(deps.as_ref().storage)?;
    if sender_raw != new_owner.new_owner_addr {
        return Err(ContractError::Unauthorized{});
    }

    config.owner_addr = new_owner.new_owner_addr;
    store_config(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    interest_model: Option<Addr>,
    distribution_model: Option<Addr>,
    max_borrow_factor: Option<Decimal256>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner_addr {
        return Err(ContractError::Unauthorized {});
    }

    if interest_model.is_some() {
        let mut state: State = read_state(deps.storage)?;
        compute_interest(deps.as_ref(), &config, &mut state, env.block.height, None)?;
        store_state(deps.storage, &state)?;

        if let Some(interest_model) = interest_model {
            config.interest_model = deps.api.addr_canonicalize(interest_model.as_str())?;
        }
    }

    if let Some(distribution_model) = distribution_model {
        config.distribution_model = deps.api.addr_canonicalize(distribution_model.as_str())?;
    }

    if let Some(max_borrow_factor) = max_borrow_factor {
        config.max_borrow_factor = max_borrow_factor;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn execute_epoch_operations(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _deposit_rate: Decimal256,
    target_deposit_rate: Decimal256,
    _threshold_deposit_rate: Decimal256,
    distributed_interest: Uint256,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    if config.overseer_contract != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let mut state: State = read_state(deps.storage)?;

    // Compute interest and reward before updating kpt_emission_rate
    let atoken_supply = query_supply(
        deps.as_ref(),
        deps.api.addr_humanize(&config.atoken_contract)?,
    )?;

    let balance: Uint256 = query_balance(
        deps.as_ref(),
        deps.api.addr_humanize(&config.contract_addr)?,
        config.stable_denom.to_string(),
    )? - distributed_interest;

    let borrow_rate_res: BorrowRateResponse = query_borrow_rate(
        deps.as_ref(),
        deps.api.addr_humanize(&config.interest_model)?,
        balance,
        state.total_liabilities,
        state.total_reserves,
    )?;

    compute_interest_raw(
        &mut state,
        env.block.height,
        balance,
        atoken_supply,
        borrow_rate_res.rate,
        target_deposit_rate,
    );

    // recompute prev_exchange_rate with distributed_interest
    state.prev_exchange_rate =
        compute_exchange_rate_raw(&state, atoken_supply, balance + distributed_interest);

    compute_reward(&mut state, env.block.height);

    // Compute total_reserves to fund collector contract
    // Update total_reserves and send it to collector contract
    // only when there is enough balance
    let total_reserves = state.total_reserves * Uint256::one();
    let messages: Vec<CosmosMsg> = if !total_reserves.is_zero() && balance > total_reserves {
        state.total_reserves = state.total_reserves - Decimal256::from_uint256(total_reserves);

        // vec![
        //     CosmosMsg::Wasm(WasmMsg::Execute {
        //         contract_addr: deps.api.addr_humanize(&config.stable_contract)?.to_string(),
        //         funds : vec![],
        //         msg: to_binary(&Cw20ExecuteMsg::Transfer {
        //             recipient: deps.api
        //                  .addr_humanize(&config.collector_contract)?
        //                  .to_string(),
        //             amount: total_reserves.into(),
        //             })?
        //         }
        //     )
        // ]
        vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: deps
                .api
                .addr_humanize(&config.collector_contract)?
                .to_string(),
            amount: vec![deduct_tax(
                deps.as_ref(),
                Coin {
                    denom: config.stable_denom,
                    amount: total_reserves.into(),
                },
            )?],
        })]
    } else {
        vec![]
    };

    store_state(deps.storage, &state)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "execute_epoch_operations"),
        attr("total_reserves", total_reserves),
        attr("kpt_emission_rate", state.kpt_emission_rate.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State { block_height } => to_binary(&query_state(deps, env, block_height)?),
        QueryMsg::EpochState {
            block_height,
            distributed_interest,
        } => to_binary(&query_epoch_state(
            deps,
            block_height,
            distributed_interest,
        )?),
        QueryMsg::BorrowerInfo {
            borrower,
            block_height,
        } => to_binary(&query_borrower_info(
            deps,
            env,
            deps.api.addr_validate(&borrower)?,
            block_height,
        )?),
        QueryMsg::BorrowerInfos { start_after, limit } => to_binary(&query_borrower_infos(
            deps,
            optional_addr_validate(deps.api, start_after)?,
            limit,
        )?),
        
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = read_config(deps.storage)?;
    Ok(ConfigResponse {
        contract_addr: deps.api.addr_humanize(&config.contract_addr)?.to_string(),
        owner_addr: deps.api.addr_humanize(&config.owner_addr)?.to_string(),
        atoken_contract: deps.api.addr_humanize(&config.atoken_contract)?.to_string(),
        interest_model: deps.api.addr_humanize(&config.interest_model)?.to_string(),
        distribution_model: deps
            .api
            .addr_humanize(&config.distribution_model)?
            .to_string(),
        overseer_contract: deps
            .api
            .addr_humanize(&config.overseer_contract)?
            .to_string(),
        collector_contract: deps
            .api
            .addr_humanize(&config.collector_contract)?
            .to_string(),
        distributor_contract: deps
            .api
            .addr_humanize(&config.distributor_contract)?
            .to_string(),
        stable_denom: config.stable_denom,
        max_borrow_factor: config.max_borrow_factor,
    })
}

pub fn query_state(deps: Deps, env: Env, block_height: Option<u64>) -> StdResult<StateResponse> {
    let mut state: State = read_state(deps.storage)?;

    let block_height = if let Some(block_height) = block_height {
        block_height
    } else {
        env.block.height
    };

    if block_height < state.last_interest_updated {
        return Err(StdError::generic_err(
            "block_height must bigger than last_interest_updated",
        ));
    }

    if block_height < state.last_reward_updated {
        return Err(StdError::generic_err(
            "block_height must bigger than last_reward_updated",
        ));
    }

    let config: Config = read_config(deps.storage)?;

    // Compute interest rate with given block height
    compute_interest(deps, &config, &mut state, block_height, None)?;

    // // Compute reward rate with given block height
    // compute_reward(&mut state, block_height);

    Ok(StateResponse {
        total_liabilities: state.total_liabilities,
        total_reserves: state.total_reserves,
        last_interest_updated: state.last_interest_updated,
        global_interest_index: state.global_interest_index,
        prev_atoken_supply: state.prev_atoken_supply,
        prev_exchange_rate: state.prev_exchange_rate,
    })
}

pub fn query_epoch_state(
    deps: Deps,
    block_height: Option<u64>,
    distributed_interest: Option<Uint256>,
) -> StdResult<EpochStateResponse> {
    let config: Config = read_config(deps.storage)?;
    let mut state: State = read_state(deps.storage)?;

    let distributed_interest = distributed_interest.unwrap_or_else(Uint256::zero);
    let atoken_supply = query_supply(deps, deps.api.addr_humanize(&config.atoken_contract)?)?;

    let balance = query_balance(
        deps,
        deps.api.addr_humanize(&config.contract_addr)?,
        config.stable_denom.to_string(),
    )? - distributed_interest;

    if let Some(block_height) = block_height {
        if block_height < state.last_interest_updated {
            return Err(StdError::generic_err(
                "block_height must bigger than last_interest_updated",
            ));
        }

        let borrow_rate_res: BorrowRateResponse = query_borrow_rate(
            deps,
            deps.api.addr_humanize(&config.interest_model)?,
            balance,
            state.total_liabilities,
            state.total_reserves,
        )?;

        let target_deposit_rate: Decimal256 =
            query_target_deposit_rate(deps, deps.api.addr_humanize(&config.overseer_contract)?)?;

        // Compute interest rate to return latest epoch state
        compute_interest_raw(
            &mut state,
            block_height,
            balance,
            atoken_supply,
            borrow_rate_res.rate,
            target_deposit_rate,
        );
    }

    // compute_interest_raw store current exchange rate
    // as prev_exchange_rate, so just return prev_exchange_rate
    let exchange_rate =
        compute_exchange_rate_raw(&state, atoken_supply, balance + distributed_interest);

    Ok(EpochStateResponse {
        exchange_rate,
        atoken_supply,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
