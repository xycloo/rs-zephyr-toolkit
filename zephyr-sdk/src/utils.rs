//! Utilities for working with common data patterns.
//! 
use soroban_sdk::xdr::{Int128Parts, ScMapEntry, ScString, ScSymbol, ScVal, ScVec, VecM};
use crate::{EnvClient, SdkError};

/// Returns an allocated String object starting from a Soroban SDK Address object.
pub fn address_to_alloc_string(env: &EnvClient, address: soroban_sdk::Address) -> String {
    soroban_string_to_alloc_string(env, address.to_string())
}

/// Returns an allocated String object starting from a Soroban SDK String object.
pub fn soroban_string_to_alloc_string(env: &EnvClient, string: soroban_sdk::String) -> String {
    let soroban_string = env.to_scval(string);
    let ScVal::String(ScString(string)) = soroban_string else {
        panic!()
    };
    string.try_into().unwrap()
}

/// Extract the instance storage map from an ScVal.
pub fn instance_entries(val: &ScVal) -> Option<Vec<ScMapEntry>> {
    if let ScVal::ContractInstance(instance) = val {
        if let Some(map) = &instance.storage {
            return Some(map.to_vec());
        }
    }
    None
}

/// Convert Int128Parts into a native i128.
pub fn parts_to_i128(parts: &Int128Parts) -> i128 {
    ((parts.hi as i128) << 64) | (parts.lo as i128)
}

/// Converts a vector into an array.
/// Panics if the provided array size != vector's length.
pub fn to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into().unwrap_or_else(|v: Vec<T>| {
        panic!("Expected a Vec of length {} but it was {}", N, v.len())
    })
}

#[allow(missing_docs)]
pub fn to_datakey_u32(int: u32) -> ScVal {
    ScVal::U32(int)
}

#[allow(missing_docs)]
pub fn to_datakey_symbol(variant_str: &str) -> ScVal {
    let tot_s_val = ScVal::Symbol(ScSymbol(variant_str.to_string().try_into().unwrap()));

    ScVal::Vec(Some(ScVec(VecM::try_from(vec![tot_s_val]).unwrap())))
}

#[allow(missing_docs)]
pub fn to_scval_symbol(from: &str) -> Result<ScVal, SdkError> {
    Ok(ScVal::Symbol(ScSymbol(
        from.try_into().map_err(|_| SdkError::Conversion)?,
    )))
}
