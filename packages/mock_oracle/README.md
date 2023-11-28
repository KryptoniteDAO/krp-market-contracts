# 说明

此类主要实现了一个模拟的oracle合约，用于测试。
实现方法为：在合约中存储一个价格映射表，通过`update_price_feed`方法更新价格，通过`price_feed`方法查询价格。

# 编译

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.11
```

# 部署

```bash
seid tx wasm store artifacts/mock_oracle.wasm -y --from=admin \
--chain-id atlantic-2 --node https://sei-testnet-2-rpc.brocha.in/ \
--gas=1225925 --gas-prices=0.01usei --broadcast-mode=block
```

# 实例化

```bash
seid tx wasm instantiate 750 '{}' \
--chain-id atlantic-2 --node https://sei-testnet-2-rpc.brocha.in/ --from admin  \
--gas=200000 --gas-prices=0.01usei --broadcast-mode=block --label "test" \
--admin sei13xy3940qrar0k82k7fzhjpqaxj0h0tep7cpuxz
```

返回合约地址：`sei1f6ctrws0eqha3nn7gxetj7h95wlmjrslwqgs9ecumf4cs00wktfstel0m9`

# 配置及查询

设置价格

ps:`id`必须为十六进制的字符串

```bash
seid tx wasm execute sei1f6ctrws0eqha3nn7gxetj7h95wlmjrslwqgs9ecumf4cs00wktfstel0m9 \
'{"update_price_feed":{"id":"5bc91f13e412c07599167bae86f07543f076a638962b8d6017ec19dab4a82814","price":189000000000}}' \
--chain-id atlantic-2 --node https://sei-testnet-2-rpc.brocha.in/ --from admin  \
--gas=200000 --gas-prices=0.01usei --broadcast-mode=block
```

查询价格

```bash
seid query wasm contract-state smart sei1f6ctrws0eqha3nn7gxetj7h95wlmjrslwqgs9ecumf4cs00wktfstel0m9 \
	'{"price_feed":{"id":"5bc91f13e412c07599167bae86f07543f076a638962b8d6017ec19dab4a82814"}}' \
	--chain-id atlantic-2 --node https://sei-testnet-2-rpc.brocha.in/ --output json
```
