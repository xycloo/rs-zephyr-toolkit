//! Common structures between the environment and the SDK.
//! This crate omits the structures that are shared between
//! Zephyr and Mercury due to the latter's closed-source nature.

pub mod http;
pub mod log;
pub mod wrapping;

pub fn to_fixed<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

#[repr(u32)]
pub enum ZephyrStatus {
    Unknown = 0,
    Success = 1,
    DbWriteError = 2,
    DbReadError = 3,
    NoValOnStack = 4,
    HostConfiguration = 5,
}

use http::AgnosticRequest;
use log::ZephyrLog;
use serde::{Deserialize, Serialize};
use stellar_xdr::next::{LedgerEntry, ScAddress, ScVal};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Invalid permissions. Tried reading when in write-only")]
    ReadOnWriteOnly,

    #[error("Invalid permissions. Tried writing when in read-only")]
    WriteOnReadOnly,

    #[error("Zephyr query malformed.")]
    ZephyrQueryMalformed,

    #[error("Zephyr query error.")]
    ZephyrQueryError,

    #[error("Unable to write to DB.")]
    WriteError,

    #[error("Unable to parse operator.")]
    OperatorError,
}

impl From<anyhow::Error> for ZephyrStatus {
    fn from(value: anyhow::Error) -> Self {
        match value.downcast_ref() {
            Some(DatabaseError::WriteError) => ZephyrStatus::DbWriteError,
            Some(DatabaseError::ZephyrQueryError) => ZephyrStatus::DbReadError,
            Some(DatabaseError::ZephyrQueryMalformed) => ZephyrStatus::DbReadError,
            Some(DatabaseError::ReadOnWriteOnly) => ZephyrStatus::HostConfiguration,
            Some(DatabaseError::WriteOnReadOnly) => ZephyrStatus::HostConfiguration,
            Some(DatabaseError::OperatorError) => ZephyrStatus::DbWriteError, // todo: specific error
            None => ZephyrStatus::Unknown,
        }
    }
}

impl From<u32> for ZephyrStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Success,
            2 => Self::DbWriteError,
            3 => Self::DbReadError,
            4 => Self::NoValOnStack,
            5 => Self::HostConfiguration,
            _ => panic!("Unrecoverable status"),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub enum ZephyrVal {
    I128(i128),
    I64(i64),
    U64(u64),
    F64(f64),
    U32(u32),
    I32(i32),
    F32(f32),
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub enum ZephyrValError {
    ConversionError,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContractDataEntry {
    pub contract_id: ScAddress,
    pub key: ScVal,
    pub entry: LedgerEntry,
    pub durability: i32,
    pub last_modified: i32,
}


#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Account {
    pub account_id: String,
    pub native_balance: f64,
    pub buying_liabilities: f64,
    pub selling_liabilities: f64,
    pub seq_num: f64,
    pub num_subentries: i32,
    pub num_sponsored: i32,
    pub num_sponsoring: i32,
}

macro_rules! impl_inner_from_serialize_only {
    ($variant:ident, $inner:ty) => {
        impl From<$inner> for ZephyrVal {
            fn from(value: $inner) -> Self {
                ZephyrVal::$variant(value)
            }
        }
    };
}

macro_rules! impl_inner_from_deserialize_generic {
    ($variant:ident, $inner:ty) => {
        impl From<ZephyrVal> for $inner {
            fn from(value: ZephyrVal) -> Self {
                match value {
                    ZephyrVal::$variant(inner_val) => inner_val,
                    _ => panic!("Attempted to convert ZephyrVal variant to different inner type"),
                }
            }
        }
    };
}

macro_rules! impl_inner_from_deserialize_numeric {
    ($inner:ty) => {
        impl From<ZephyrVal> for $inner {
            fn from(value: ZephyrVal) -> Self {
                match value {
                    //ZephyrVal::F32(num) => num as $inner,
                    //ZephyrVal::F64(num) => num as $inner,
                    ZephyrVal::I128(num) => num as $inner,
                    ZephyrVal::I32(num) => num as $inner,
                    ZephyrVal::I64(num) => num as $inner,
                    ZephyrVal::U32(num) => num as $inner,
                    ZephyrVal::U64(num) => num as $inner,
                    _ => panic!("Attempted to convert ZephyrVal variant to different inner type"),
                }
            }
        }
    };
}

// Ser
impl_inner_from_serialize_only!(I128, i128);
impl_inner_from_serialize_only!(I64, i64);
impl_inner_from_serialize_only!(U64, u64);
impl_inner_from_serialize_only!(F64, f64);
impl_inner_from_serialize_only!(U32, u32);
impl_inner_from_serialize_only!(I32, i32);
impl_inner_from_serialize_only!(F32, f32);
impl_inner_from_serialize_only!(String, String);
impl_inner_from_serialize_only!(Bytes, Vec<u8>);

// Deser
impl_inner_from_deserialize_numeric!(i128);
impl_inner_from_deserialize_numeric!(i64);
impl_inner_from_deserialize_numeric!(u64);
impl_inner_from_deserialize_numeric!(u32);
impl_inner_from_deserialize_numeric!(i32);
impl_inner_from_deserialize_generic!(String, String);
impl_inner_from_deserialize_generic!(Bytes, Vec<u8>);
impl_inner_from_deserialize_generic!(F64, f64);
impl_inner_from_deserialize_generic!(F32, f32);


#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum RelayedMessageRequest {
    Http(AgnosticRequest),
    Log(ZephyrLog),
}
