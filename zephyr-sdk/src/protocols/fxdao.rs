use crate::{
    utils::{address_from_str, address_to_alloc_string},
    EnvClient,
};
use soroban_sdk::{
    self, contracttype,
    xdr::{ContractDataEntry, LedgerEntryData, ScContractInstance, ScVal, ToXdr},
    Address, Symbol, TryIntoVal,
};

use super::{
    blend::{HfResponse, SCALAR_7, SCALAR_9},
    reflector::{Asset, PriceData},
};

#[contracttype]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum VaultsDataKeys {
    VaultsInfo(Symbol),
    Vault((Address, Symbol)),
    VaultIndex(VaultIndexKey),
}

#[contracttype]
#[derive(Debug)]
pub struct CoreState {
    pub col_token: Address,
    pub stable_issuer: Address,
    pub admin: Address,
    pub protocol_manager: Address,
    pub panic_mode: bool,
    pub treasury: Address,
    pub fee: u128,
    pub oracle: Address,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[contracttype]
pub enum CoreDataKeys {
    CoreState,
}

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum OptionalVaultKey {
    None,
    Some(VaultKey),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultKey {
    pub index: u128,
    pub account: Address,
    pub denomination: Symbol,
}

#[contracttype]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct VaultIndexKey {
    pub user: Address,
    pub denomination: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Vault {
    pub index: u128,
    pub next_key: OptionalVaultKey,
    pub account: Address,
    pub total_debt: u128,
    pub total_collateral: u128,
    pub denomination: Symbol,
}

#[contracttype]
#[derive(Debug, Clone)]
pub struct VaultsInfo {
    pub denomination: Symbol,
    pub total_vaults: u64,
    pub total_debt: u128,
    pub total_col: u128,
    pub lowest_key: OptionalVaultKey,
    pub min_col_rate: u128,
    // Min collateral ratio - ex: 1.10
    pub min_debt_creation: u128,
    // Min vault creation amount - ex: 5000
    pub opening_col_rate: u128, // Opening collateral ratio - ex: 1.15
}

/// Gets the user's valut object given the user address, fxdao contract and denomination.
pub fn get_user_vault(
    env: &EnvClient,
    contract: [u8; 32],
    user: Address,
    denomination: Symbol,
) -> Option<Vault> {
    let entries = env.read_contract_entries(contract).unwrap();

    for entry in entries {
        if let Ok(VaultsDataKeys::Vault((got_user, got_denom))) = env.try_from_scval(&entry.key) {
            if got_user == user.clone() && got_denom == denomination.clone() {
                let LedgerEntryData::ContractData(ContractDataEntry { val, .. }) = entry.entry.data
                else {
                    panic!()
                };
                let got_vault = env.from_scval::<Vault>(&val);

                return Some(got_vault);
            }
        }
    }

    None
}

/// Gets all valults for a certain denomination
pub fn get_all_denom_vaults_idxs(
    env: &EnvClient,
    contract: &str,
    denomination: &str,
) -> Vec<(String, i64)> {
    let contract = stellar_strkey::Contract::from_string(&contract).unwrap().0;
    let denom = Symbol::new(env.soroban(), &denomination);
    let entries = env.read_contract_entries(contract).unwrap();
    let mut vaults = Vec::new();

    for entry in entries {
        if let Ok(VaultsDataKeys::Vault((_, got_denom))) = env.try_from_scval(&entry.key) {
            if got_denom == denom.clone() {
                let LedgerEntryData::ContractData(ContractDataEntry { val, .. }) = entry.entry.data
                else {
                    panic!()
                };
                let got_vault = env.from_scval::<Vault>(&val);

                vaults.push((
                    address_to_alloc_string(env, got_vault.account),
                    got_vault.index as i64,
                ));
            }
        }
    }

    vaults
}

/// Get the fxdao health factor given a fxdao contract, user and denominiation.
pub fn get_hf(contract: String, user: String, denomination: String) -> HfResponse {
    let env = EnvClient::empty();

    let contract = stellar_strkey::Contract::from_string(&contract).unwrap().0;
    let user = address_from_str(&env, &user);
    let denom = Symbol::new(&env.soroban(), &denomination);

    let (xlm_price, min_ratio) = {
        let mut protocol_config: Option<CoreState> = None;
        let mut min_ratio = 0;

        let instance = env.read_contract_instance(contract).unwrap().unwrap();
        let LedgerEntryData::ContractData(ContractDataEntry { val, .. }) = instance.entry.data
        else {
            panic!()
        };
        let ScVal::ContractInstance(ScContractInstance { storage, .. }) = val else {
            panic!()
        };

        for kv in storage.unwrap().0.to_vec() {
            if let Ok(CoreDataKeys::CoreState) = env.try_from_scval(&kv.key) {
                protocol_config = Some(env.from_scval(&kv.val));
            } else if let Ok(VaultsDataKeys::VaultsInfo(got_denom)) = env.try_from_scval(&kv.key) {
                if got_denom == denom {
                    let info = env.try_from_scval::<VaultsInfo>(&kv.val);
                    if let Ok(info) = info {
                        min_ratio = info.min_col_rate;
                    }
                }
            }
        }

        let result = env.simulate_contract_call(
            "GANXGJV2RNOFMOSQ2DTI3RKDBAVERXUVFC27KW3RLVQCLB3RYNO3AAI4".into(),
            stellar_strkey::Contract::from_string(&address_to_alloc_string(
                &env,
                protocol_config.unwrap().oracle,
            ))
            .unwrap()
            .0,
            Symbol::new(&env.soroban(), "lastprice"),
            (
                address_from_str(&env, &stellar_strkey::Contract(contract).to_string()),
                Asset::Other(denom.clone()),
            )
                .try_into_val(env.soroban())
                .unwrap(),
        );

        let data: Option<PriceData> = env.from_scval(&result.unwrap().invoke_result.unwrap());

        (data.unwrap().price, min_ratio)
    };

    let Some(vault) = get_user_vault(&env, contract, user, denom) else {
        panic!()
    };
    let current_ratio = ((vault.total_collateral as f64 / SCALAR_7 as f64) * xlm_price as f64)
        / (vault.total_debt as f64 / SCALAR_7 as f64);

    HfResponse {
        min: min_ratio as i64,
        current: current_ratio as i64,
    }
}

/// Calculates the vault's index.
pub fn calc_vault_idx(collateral: f64, debt: f64) -> f64 {
    (SCALAR_9 as f64 * collateral) / debt
}

/// Get the previous key for fxdao.
pub fn fxdao_get_prev_key(
    env: &EnvClient,
    current_vaults: std::vec::Vec<(String, i64)>,
    idx: i64,
    denom: Symbol,
) -> ScVal {
    let mut lowest_other_vault = OptionalVaultKey::None;

    for n in 0..current_vaults.len() {
        let vault = current_vaults.get(n).unwrap();
        match lowest_other_vault.clone() {
            OptionalVaultKey::None => {
                lowest_other_vault = OptionalVaultKey::Some(VaultKey {
                    index: vault.1 as u128,
                    account: address_from_str(&env, &vault.0),
                    denomination: denom.clone(),
                })
            }

            OptionalVaultKey::Some(key) => {
                if vault.1 < key.index as i64 {
                    lowest_other_vault = OptionalVaultKey::Some(VaultKey {
                        index: vault.1 as u128,
                        account: address_from_str(&env, &vault.0),
                        denomination: denom.clone(),
                    })
                }
            }
        }
    }

    if let OptionalVaultKey::Some(VaultKey { index, .. }) = lowest_other_vault {
        if (index as i64) > idx {
            lowest_other_vault = OptionalVaultKey::None
        }
    }

    env.to_scval(lowest_other_vault)
}
