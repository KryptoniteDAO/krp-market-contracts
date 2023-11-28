use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::testing::{mock_env};
use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Timestamp,
};
use cw20_legacy::msg::InstantiateMsg as TokenInstantiateMsg;
use moneymarket::custody::{
    BAssetInfo, ExecuteMsg as CustodyExecuteMsg, InstantiateMsg as CustodyInstantiateMsg,
    QueryMsg as CustodyQueryMsg,
};
use moneymarket::distribution_model::InstantiateMsg as DistributionModelInstantiateMsg;
use moneymarket::interest_model::InstantiateMsg as InterestModelInstantiateMsg;
use moneymarket::market::{
    BorrowerInfoResponse, ExecuteMsg as MarketExecuteMsg, InstantiateMsg as MarketInstantiateMsg,
    MigrateMsg as MarketMigrateMsg, QueryMsg as MarketQueryMsg,
};
use moneymarket::oracle_pyth::{
    ExecuteMsg as OraclePythExecuteMsg, InstantiateMsg as OraclePythInstantiateMsg, PriceResponse,
    QueryMsg as OraclePythMsg,
};

use moneymarket::mock_pyth_contract::{
    ExecuteMsg as MockPythContractExecuteMsg, InstantiateMsg as MockPythContractInstantiateMsg,
};

use moneymarket::overseer::{
    BorrowLimitResponse, CollateralsResponse, ExecuteMsg as OverseerExecuteMsg,
    InstantiateMsg as OverseerInstantiateMsg, MigrateMsg as OverseerMigrateMsg,
    QueryMsg as OverseerQueryMsg,
};

use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor};
use pyth_sdk_cw::PriceIdentifier;

// use std::io::Write;
// use std::str::FromStr;
//use cw-multi-test::{AppBuilder, BankKeeper, ContractWrapper, Executor, App, TerraMock};

const OWNER: &str = "owner";
const USER: &str = "user";
const ADMIN: &str = "admin";

fn mock_app(owner: Addr, coins: Vec<Coin>, block_time: Option<u64>) -> App {
    let mut block = mock_env().block;
    if let Some(time) = block_time {
        block.time = Timestamp::from_seconds(time);
    }
    AppBuilder::new()
        .with_block(block)
        .build(|app, _, storage| app.bank.init_balance(storage, &owner, coins).unwrap())
}

fn mock_custody_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: CustodyInstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

fn mock_custody_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: CustodyExecuteMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

fn mock_custody_query(_deps: Deps, _env: Env, _msg: CustodyQueryMsg) -> StdResult<Binary> {
    to_json_binary(&())
}

fn store_token_contract_code(app: &mut App) -> u64 {
    let token_contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(token_contract)
}

fn store_custody_contract_code(app: &mut App) -> u64 {
    let custody_contract = Box::new(ContractWrapper::new_with_empty(
        mock_custody_execute,
        mock_custody_instantiate,
        mock_custody_query,
    ));
    app.store_code(custody_contract)
}

fn store_market_contract_code(app: &mut App) -> u64 {
    let market_contract = Box::new(
        ContractWrapper::new_with_empty(
            moneymarket_market::contract::execute,
            moneymarket_market::contract::instantiate,
            moneymarket_market::contract::query,
        )
        .with_reply_empty(moneymarket_market::contract::reply)
        .with_migrate_empty(moneymarket_market::contract::migrate),
    );

    app.store_code(market_contract)
}

fn store_overseer_contract_code(app: &mut App) -> u64 {
    let overseer_contract = Box::new(
        ContractWrapper::new_with_empty(
            moneymarket_overseer::contract::execute,
            moneymarket_overseer::contract::instantiate,
            moneymarket_overseer::contract::query,
        )
        .with_migrate_empty(moneymarket_overseer::contract::migrate),
    );

    app.store_code(overseer_contract)
}

fn store_oracle_contract_code(app: &mut App) -> u64 {
    let oracle_contract = Box::new(ContractWrapper::new_with_empty(
        oracle_pyth::contract::execute,
        oracle_pyth::contract::instantiate,
        oracle_pyth::contract::query,
    ));

    app.store_code(oracle_contract)
}

fn store_mock_pyth_contract_code(app: &mut App) -> u64 {
    let mock_pyth_contract = Box::new(ContractWrapper::new_with_empty(
        mock_oracle::contract::execute,
        mock_oracle::contract::instantiate,
        mock_oracle::contract::query,
    ));

    app.store_code(mock_pyth_contract)
}

fn store_interest_model_code(app: &mut App) -> u64 {
    let interest_model_contract = Box::new(ContractWrapper::new_with_empty(
        moneymarket_interest_model::contract::execute,
        moneymarket_interest_model::contract::instantiate,
        moneymarket_interest_model::contract::query,
    ));

    app.store_code(interest_model_contract)
}

fn store_distribution_model_code(app: &mut App) -> u64 {
    let distribution_model_contract = Box::new(ContractWrapper::new_with_empty(
        moneymarket_distribution_model::contract::execute,
        moneymarket_distribution_model::contract::instantiate,
        moneymarket_distribution_model::contract::query,
    ));

    app.store_code(distribution_model_contract)
}

fn create_contracts(input_coins: Option<Vec<Coin>>) -> (App, Addr, Addr, Addr, Addr, Addr, Addr) {
    let owner = Addr::unchecked(OWNER);
    let admin = Addr::unchecked(ADMIN);
    let mut init_coins: Vec<Coin> = vec![];
    init_coins.append(&mut coins(1000_000_000_000_000, "uusd"));
    if let Some(input_coins) = input_coins {
        init_coins.append(&mut input_coins.clone());
    }
    let mut app = mock_app(owner.clone(), init_coins, None);

    // these 3 contracts are not needed for now
    let liquidator_addr = "liquidation_addr";
    let collector_addr = "collector_addr";
    let distributor_addr = "distributor_addr";
    let reward_contract_addr = "reward_contract_addr";
    let swap_contract_addr = "swap_contract_addr";
    // let mock_pyth_oracle_addr = "mock_pyth_oracle_addr";

    // store contract codes
    let token_code_id = store_token_contract_code(&mut app);
    let custody_code_id = store_custody_contract_code(&mut app);
    let oracle_pyth_code_id = store_oracle_contract_code(&mut app);
    let mock_pyth_code_id = store_mock_pyth_contract_code(&mut app);
    let interest_model_code_id = store_interest_model_code(&mut app);
    let distribution_model_code_id = store_distribution_model_code(&mut app);
    let market_code_id = store_market_contract_code(&mut app);
    let overseer_code_id = store_overseer_contract_code(&mut app);

    // instantiate mock pyth contract
    let msg = MockPythContractInstantiateMsg {};
    let mock_pyth_oracle_addr = app
        .instantiate_contract(
            mock_pyth_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("MOCK_PYTH_CONTRACT"),
            None,
        )
        .unwrap();

    // instantiate oracle pyth contract
    let msg = OraclePythInstantiateMsg {
        owner: owner.clone(),
        pyth_contract: mock_pyth_oracle_addr.to_string(),
    };
    let oracle_addr: Addr = app
        .instantiate_contract(
            oracle_pyth_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("PYTH_ORACLE"),
            None,
        )
        .unwrap();

    // instantiate interest model contract
    let msg = InterestModelInstantiateMsg {
        owner: owner.to_string(),
        base_rate: Decimal256::percent(10),
        interest_multiplier: Decimal256::percent(10),
    };
    let interest_model_addr = app
        .instantiate_contract(
            interest_model_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("INTEREST MODEL"),
            None,
        )
        .unwrap();

    // instantiate distribution model contract
    let msg = DistributionModelInstantiateMsg {
        owner: owner.to_string(),
        emission_cap: Decimal256::from_uint256(100u64),
        emission_floor: Decimal256::from_uint256(10u64),
        increment_multiplier: Decimal256::percent(110),
        decrement_multiplier: Decimal256::percent(90),
    };
    let distribution_model_addr = app
        .instantiate_contract(
            distribution_model_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("INTEREST MODEL"),
            None,
        )
        .unwrap();

    // instantiate market contract
    let msg = MarketInstantiateMsg {
        owner_addr: owner.to_string(),
        stable_denom: "uusd".to_string(),
        atoken_code_id: token_code_id,
        kpt_emission_rate: Decimal256::one(),
        max_borrow_factor: Decimal256::one(),
    };
    let market_addr = app
        .instantiate_contract(
            market_code_id,
            owner.clone(),
            &msg,
            &coins(1000000, "uusd"),
            String::from("MARKET"),
            Some(admin.to_string()),
        )
        .unwrap();

    // instantiate overseer contract
    let msg = OverseerInstantiateMsg {
        owner_addr: owner.to_string(),
        oracle_contract: oracle_addr.to_string(),
        market_contract: market_addr.to_string(),
        liquidation_contract: liquidator_addr.to_string(),
        collector_contract: collector_addr.to_string(),
        stable_denom: "uusd".to_string(),
        epoch_period: 86400u64,
        threshold_deposit_rate: Decimal256::permille(3),
        target_deposit_rate: Decimal256::permille(5),
        buffer_distribution_factor: Decimal256::percent(20),
        kpt_purchase_factor: Decimal256::percent(20),
        price_timeframe: 60u64,
        dyn_rate_epoch: 8600u64,
        dyn_rate_maxchange: Decimal256::permille(5),
        dyn_rate_yr_increase_expectation: Decimal256::permille(1),
        dyn_rate_min: Decimal256::from_ratio(1000000000000u64, 1000000000000000000u64),
        dyn_rate_max: Decimal256::from_ratio(1200000000000u64, 1000000000000000000u64),
    };
    let overseer_addr = app
        .instantiate_contract(
            overseer_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("OVERSEER"),
            Some(admin.to_string()),
        )
        .unwrap();

    // register contracts to market
    let msg = MarketExecuteMsg::RegisterContracts {
        overseer_contract: overseer_addr.to_string(),
        interest_model: interest_model_addr.to_string(),
        distribution_model: distribution_model_addr.to_string(),
        collector_contract: collector_addr.to_string(),
        distributor_contract: distributor_addr.to_string(),
    };

    app.execute_contract(owner.clone(), market_addr.clone(), &msg, &[])
        .unwrap();

    // instantiate bsei
    let msg = TokenInstantiateMsg {
        name: "bsei".to_string(),
        symbol: "bsei".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
    };

    let bsei_token_addr = app
        .instantiate_contract(token_code_id, owner.clone(), &msg, &[], "bsei", None)
        .unwrap();

    // instantiate custody contract
    let msg = CustodyInstantiateMsg {
        owner: owner.to_string(),
        collateral_token: bsei_token_addr.to_string(),
        overseer_contract: overseer_addr.to_string(),
        market_contract: market_addr.to_string(),
        reward_contract: reward_contract_addr.to_string(),
        liquidation_contract: liquidator_addr.to_string(),
        stable_denom: "uusd".to_string(),
        swap_contract: swap_contract_addr.to_string(),
        swap_denoms: vec!["uusd".to_string()],
        basset_info: BAssetInfo {
            name: "bsei".to_string(),
            symbol: "bsei".to_string(),
            decimals: 6,
        },
    };

    let custody_contract_addr = app
        .instantiate_contract(
            custody_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("CUSTODY"),
            None,
        )
        .unwrap();

    (
        app,
        market_addr,
        overseer_addr,
        bsei_token_addr,
        custody_contract_addr,
        oracle_addr,
        mock_pyth_oracle_addr,
    )
}

fn migrate_contracts(app: &mut App, market_addr: &Addr, overseer_addr: &Addr) {
    let admin = Addr::unchecked(ADMIN);

    // store new contract code
    let market_code_id = store_market_contract_code(app);
    let overseer_code_id = store_overseer_contract_code(app);

    // migrate market contract
    let msg = MarketMigrateMsg {};
    app.migrate_contract(admin.clone(), market_addr.clone(), &msg, market_code_id)
        .unwrap();

    // migrate overseer contract
    let msg = OverseerMigrateMsg {};
    app.migrate_contract(admin, overseer_addr.clone(), &msg, overseer_code_id)
        .unwrap();
}

#[test]
fn test_migration() {
    let (mut app, market_addr, overseer_addr, _, _, _, _) = create_contracts(None);
    migrate_contracts(&mut app, &market_addr, &overseer_addr);
}

#[test]
fn test_successfully_repay_stable_from_yield_reserve() {
    let owner = Addr::unchecked(OWNER);
    let user = Addr::unchecked(USER);

    let (
        mut app,
        market_addr,
        overseer_addr,
        bsei_token_addr,
        custody_contract_addr,
        oracle_addr,
        mock_pyth_contract_addr,
    ) = create_contracts(None);
    app.send_tokens(
        owner.clone(),
        market_addr.clone(),
        &[coin(847_426_363u128, "uusd")],
    )
    .unwrap();
    app.send_tokens(
        owner.clone(),
        overseer_addr.clone(),
        &[coin(1_000_000_000u128, "uusd")],
    )
    .unwrap();

    // register whitelist
    let msg = OverseerExecuteMsg::Whitelist {
        name: "bsei".to_string(),
        symbol: "bsei".to_string(),
        collateral_token: bsei_token_addr.to_string(),
        custody_contract: custody_contract_addr.to_string(),
        max_ltv: Decimal256::percent(60),
    };

    app.execute_contract(owner.clone(), overseer_addr.clone(), &msg, &[])
        .unwrap();

    // lock some bsei
    let msg = OverseerExecuteMsg::LockCollateral {
        borrower: user.to_string(),
        collaterals: vec![(bsei_token_addr.to_string(), Uint256::from(1_000_000_000u64))],
    };

    app.execute_contract(
        custody_contract_addr.clone(),
        overseer_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();

    // query collateral balance in overseer contract  
    let res: CollateralsResponse = app
    .wrap()
    .query_wasm_smart(
        overseer_addr.clone(),
        &OverseerQueryMsg::Collaterals { borrower: user.to_string()},
    )
    .unwrap();

    assert_eq!(res, CollateralsResponse {
        borrower: user.to_string(),
        collaterals: vec![(bsei_token_addr.to_string(), Uint256::from(1_000_000_000u64))],
    });
  
    // set feed id for bsei price
    let feed_id = String::from("53614f1cb0c031d4af66c04cb9c756234adad0e1cee85303795091499a4084eb");
    let msg = OraclePythExecuteMsg::ConfigFeedInfo {
        asset: bsei_token_addr.to_string(),
        price_feed_id: feed_id.clone(),
        price_feed_symbol: "bSEI".to_string(),
        price_feed_decimal: 8,
        check_feed_age: true,
        price_feed_age: 60,
    };
    app.execute_contract(owner.clone(), oracle_addr.clone(), &msg, &[])
        .unwrap();

    let msg = MockPythContractExecuteMsg::UpdatePriceFeed {
        // prices: vec![(
        //     bsei_token_addr.to_string(),
        //     Decimal256::from_str("10").unwrap(),
        // )],
        id: PriceIdentifier::from_hex(feed_id).unwrap(),
        price: 1000_000_000i64,
    };

    app.execute_contract(owner, mock_pyth_contract_addr.clone(), &msg, &[])
        .unwrap();

    // query bSEI price
    let res: PriceResponse = app
        .wrap()
        .query_wasm_smart(
            oracle_addr.clone(),
            &OraclePythMsg::QueryPrice {
                asset: bsei_token_addr.to_string(),
            },
        )
        .unwrap();
  assert_eq!(
        res,
        PriceResponse {
            asset: bsei_token_addr.to_string(),
            emv_price: Decimal256::from_uint256(Uint256::from(10u64)),
            emv_price_raw: 1000000000,
            price: Decimal256::from_uint256(Uint256::from(10u64)),
            price_raw: 1000000000,
            last_updated_base: app.block_info().time.seconds(),
            last_updated_quote: app.block_info().time.seconds(),
        }
    );

    let res: BorrowLimitResponse = app
        .wrap()
        .query_wasm_smart(
            overseer_addr.clone(),
            &OverseerQueryMsg::BorrowLimit {
                borrower: user.to_string(),
                block_time: None,
            },
        )
        .unwrap();

    assert_eq!(
        res,
        BorrowLimitResponse {
            borrower: user.to_string(),
            borrow_limit: Uint256::from(6_000_000_000u64),
        }
    );

    // borrow kUSD agaist bsei
    let msg = MarketExecuteMsg::BorrowStable {
        borrow_amount: Uint256::from(847_426_363u64),
        to: None,
    };
    app.execute_contract(user.clone(), market_addr.clone(), &msg, &[])
        .unwrap();

    migrate_contracts(&mut app, &market_addr, &overseer_addr);

    // repay stable from yield reserve
    let msg = OverseerExecuteMsg::RepayStableFromYieldReserve {
        borrower: user.to_string(),
    };

    app.execute_contract(Addr::unchecked(OWNER), overseer_addr.clone(), &msg, &[])
        .unwrap();

    // check remain loan amount of the user
    let res: BorrowerInfoResponse = app
        .wrap()
        .query_wasm_smart(
            market_addr.clone(),
            &MarketQueryMsg::BorrowerInfo {
                borrower: user.to_string(),
                block_height: None,
            },
        )
        .unwrap();

    assert_eq!(res.loan_amount, Uint256::zero());
}
