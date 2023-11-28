# Distribution Model

**NOTE**: Reference documentation for this contract is available [here](https://docs.kryptonite.finance/smart-contracts/money-market/distribution-model).


The Distribution Model contract manages the calculation of the KPT emission rate,
using fed-in deposit rate information. At the time of protocol genesis, the 
emission rate adjusts to double when the deposit rate is below the targeted rate
and decreases by 10% if the deposit rate is above the targeted rate. Further
descriptions on the KPT emission rate control mechanism can be found [here](https://docs.kryptonite.finance/protocol/krp-token-kpt#krp-token-supply).
