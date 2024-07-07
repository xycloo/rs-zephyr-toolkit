//! Zephyr Rust SDK
//!
//! The zephyr rust sdk aids developers in writing programs for the
//! Zephyr Virtual Machine.
//!
//! ## Hello Ledger Example
//!
//! ```
//! use rs_zephyr_sdk::{bincode, log, stellar_xdr::next::{Limits, WriteXdr}, Condition, DatabaseDerive, DatabaseInteract, EnvClient, ZephyrVal};
//!
//! #[derive(DatabaseDerive, Clone)]
//! #[with_name("curr_seq")]
//! struct Sequence {
//!     pub current: u32,
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn on_close() {
//!    let env = EnvClient::new();
//!    let reader = env.reader();
//!
//!    let sequence = Sequence {
//!        current: reader.ledger_sequence()
//!    };
//!
//!    if let Some(last) = Sequence::read_to_rows(&env).iter().find(|x| x.current == sequence.current - 1) {
//!        sequence.update(&env, &[Condition::ColumnEqualTo("current".into(), bincode::serialize(&ZephyrVal::U32(last.current)).unwrap())]);
//!    } else {
//!        sequence.put(&env)
//!    }
//! }
//! ```
//!

#![warn(missing_docs)]

#[cfg(feature = "testutils")]
pub mod testutils;

/// Charting utilities and wrappers.
pub mod charting;
mod database;
mod env;
mod external;
mod ledger;
mod ledger_meta;
mod logger;
mod symbol;

pub mod prelude;

use rs_zephyr_common::ZephyrStatus;
use serde::Deserialize;
use serde::Serialize;
use soroban_sdk::xdr::LedgerEntry;
use soroban_sdk::xdr::Limits;
use soroban_sdk::xdr::ReadXdr;
use soroban_sdk::xdr::ScAddress;
use soroban_sdk::xdr::ScVal;
use stellar_xdr::next::WriteXdr;
use thiserror::Error;

pub use database::{DatabaseInteract, TableRow, TableRows};
pub use env::EnvClient;
pub use ledger_meta::{MetaReader, PrettyContractEvent};
pub use logger::EnvLogger;
pub use ledger_meta::EntryChanges;
pub use soroban_sdk;
pub use bincode;
pub use database::Condition;
pub use macros::DatabaseInteract as DatabaseDerive;
pub use rs_zephyr_common::{
    http::{AgnosticRequest, Method},
    ZephyrVal,
};

fn to_fixed<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

//extern crate wee_alloc;
//
//#[global_allocator]
//static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Zephyr SDK errors.
#[derive(Clone, Debug, Copy, Error)]
#[allow(missing_docs)]
pub enum SdkError {
    #[error("Conversion error.")]
    Conversion,

    #[error("Error in reading database.")]
    DbRead,

    #[error("Error in writing database.")]
    DbWrite,

    #[error("No value found on host pseudo store.")]
    NoValOnStack,

    #[error("Incorrect host configurations.")]
    HostConfiguration,

    #[error("Incorrect conditional instruction. Cannot read on an update action.")]
    ReadOnUpdateAction,

    #[error("Incorrect conditional instruction. Cannot update on a read action.")]
    UpdateOnReadAction,

    #[error("Unknown error.")]
    Unknown,
}

impl SdkError {
    fn express_from_status(status: i64) -> Result<(), Self> {
        match ZephyrStatus::from(status as u32) {
            ZephyrStatus::Success => Ok(()),
            ZephyrStatus::DbReadError => Err(SdkError::DbRead),
            ZephyrStatus::DbWriteError => Err(SdkError::DbWrite),
            ZephyrStatus::NoValOnStack => Err(SdkError::NoValOnStack),
            ZephyrStatus::HostConfiguration => Err(SdkError::HostConfiguration),
            ZephyrStatus::Unknown => Err(SdkError::Unknown),
        }
    }
}

/// Some sparse scval utils.
/// Note that these might be deprecated in the future.
#[allow(missing_docs)]
pub mod utils {
    use soroban_sdk::xdr::{Int128Parts, ScMapEntry, ScSymbol, ScVal, ScVec, VecM};

    use crate::SdkError;

    pub fn to_datakey_u32(int: u32) -> ScVal {
        ScVal::U32(int)
    }

    pub fn to_datakey_symbol(variant_str: &str) -> ScVal {
        let tot_s_val = ScVal::Symbol(ScSymbol(variant_str.to_string().try_into().unwrap()));

        ScVal::Vec(Some(ScVec(VecM::try_from(vec![tot_s_val]).unwrap())))
    }

    pub fn instance_entries(val: &ScVal) -> Option<Vec<ScMapEntry>> {
        if let ScVal::ContractInstance(instance) = val {
            if let Some(map) = &instance.storage {
                return Some(map.to_vec());
            }
        }

        None
    }

    pub fn to_scval_symbol(from: &str) -> Result<ScVal, SdkError> {
        Ok(ScVal::Symbol(ScSymbol(
            from.try_into().map_err(|_| SdkError::Conversion)?,
        )))
    }

    pub fn parts_to_i128(parts: &Int128Parts) -> i128 {
        ((parts.hi as i128) << 64) | (parts.lo as i128)
    }

    pub fn to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
        v.try_into().unwrap_or_else(|v: Vec<T>| {
            panic!("Expected a Vec of length {} but it was {}", N, v.len())
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContractDataEntryStellarXDR {
    pub contract_id: stellar_xdr::next::ScAddress,
    pub key: stellar_xdr::next::ScVal,
    pub entry: stellar_xdr::next::LedgerEntry,
    pub durability: i32,
    pub last_modified: i32,
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ContractDataEntry {
    pub contract_id: ScAddress,
    pub key: ScVal,
    pub entry: LedgerEntry,
    pub durability: i32,
    pub last_modified: i32,
}

impl Into<ContractDataEntry> for ContractDataEntryStellarXDR {
    fn into(self) -> ContractDataEntry {
        ContractDataEntry {
            contract_id: ScAddress::from_xdr(
                self.contract_id
                    .to_xdr(stellar_xdr::next::Limits::none())
                    .unwrap(),
                Limits::none(),
            )
            .unwrap(),
            key: ScVal::from_xdr(
                self.key.to_xdr(stellar_xdr::next::Limits::none()).unwrap(),
                Limits::none(),
            )
            .unwrap(),
            entry: LedgerEntry::from_xdr(
                self.entry
                    .to_xdr(stellar_xdr::next::Limits::none())
                    .unwrap(),
                Limits::none(),
            )
            .unwrap(),
            durability: self.durability,
            last_modified: self.last_modified,
        }
    }
}
