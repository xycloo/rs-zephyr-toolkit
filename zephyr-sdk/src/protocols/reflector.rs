use crate::{utils::address_to_alloc_string, EnvClient};
use soroban_sdk::{contracttype, Address, Symbol, TryIntoVal};
use std::str::FromStr;

#[contracttype]
pub enum Asset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

/// Get the last price of an asset listed on reflector.
pub fn reflector_price(env: &EnvClient, oracle: Address, asset: Address) -> i128 {
    let result = env.simulate_contract_call(
        "GANXGJV2RNOFMOSQ2DTI3RKDBAVERXUVFC27KW3RLVQCLB3RYNO3AAI4".into(),
        stellar_strkey::Contract::from_str(&address_to_alloc_string(&env, oracle))
            .unwrap()
            .0,
        Symbol::new(&env.soroban(), "lastprice"),
        (Asset::Stellar(asset),)
            .try_into_val(env.soroban())
            .unwrap(),
    );

    let data: PriceData = env.from_scval(&result.unwrap().invoke_result.unwrap());
    data.price
}
