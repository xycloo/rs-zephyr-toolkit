use rs_zephyr_common::{http::AgnosticRequest, RelayedMessageRequest};
use serde::Serialize;
use stellar_xdr::next::{Limits, ReadXdr};

use crate::{database::{Database, DatabaseInteract}, external::{conclude_host, read_ledger_meta, tx_send_message}, logger::EnvLogger, Condition, MetaReader, SdkError, TableRows};


/// Zephyr's host environment client.
#[derive(Clone)]
pub struct EnvClient {
    xdr: Option<stellar_xdr::next::LedgerCloseMeta>,
    inner_soroban_host: soroban_sdk::Env,
}


impl EnvClient {
    /// Returns the logger object.
    pub fn log(&self) -> EnvLogger {
        EnvLogger
    }

    /// Returns a soroban host stub.
    pub fn soroban(&self) -> &soroban_sdk::Env {
        &self.inner_soroban_host
    }

    pub(crate) fn message_relay(message: impl Serialize) {
        let serialized = bincode::serialize(&message).unwrap();
        
        let res = unsafe {
            tx_send_message(
                serialized.as_ptr() as i64, 
                serialized.len() as i64
            )
        };

        SdkError::express_from_status(res).unwrap()
    }

    /// Sends a web request message requests to the host.
    pub fn send_web_request(&self, request: AgnosticRequest) {
        let message = RelayedMessageRequest::Http(request);
        
        Self::message_relay(message)
    }
    
    /// Reads a database table.
    /// 
    /// This function uses the [`DatabaseInteract`] trait
    /// along with the `DatabaseDerive` macro to read the rows 
    /// into a `DatabaseDerive` struct.
    pub fn read<T: DatabaseInteract>(&self) -> Vec<T> {
        T::read_to_rows(&self)
    }

    /// Writes a row to a database table.
    /// 
    /// This function uses the [`DatabaseInteract`] trait
    /// along with the `DatabaseDerive` macro to write the row 
    /// derived from the `DatabaseDerive` struct.
    pub fn put<T: DatabaseInteract>(&self, row: &T) {
        row.put(&self)
    }

    /// Updates a row to a database table.
    /// 
    /// This function uses the [`DatabaseInteract`] trait
    /// along with the `DatabaseDerive` macro to update the row 
    /// derived from the `DatabaseDerive` struct.
    pub fn update<T: DatabaseInteract>(&self, row: &T, conditions: &[Condition]) {
        row.update(&self, conditions)
    }

    /// Raw function to write to the database a row.
    pub fn db_write(&self, table_name: &str, columns: &[&str], segments: &[&[u8]]) -> Result<(), SdkError> {
        Database::write_table(table_name, columns, segments)
    }

    /// Raw function to update a database row.
    pub fn db_update(&self, table_name: &str, columns: &[&str], segments: &[&[u8]], conditions: &[Condition]) -> Result<(), SdkError> {
        Database::update_table(table_name, columns, segments, conditions)
    }

    /// Raw function to read from database.
    pub fn db_read(&self, table_name: &str, columns: &[&str]) -> Result<TableRows, SdkError> {
        Database::read_table(table_name, columns)
    }

    /// Returns the XDR reader object.
    pub fn reader(&self) -> MetaReader {
        let meta = &self.xdr;

        if let Some(meta) = meta {
            MetaReader::new(meta)
        } else {
            panic!("Internal SDK error") // todo: handle
        }
    }

    /// New instance of the zephyr client with the ledger
    /// meta already set.
    pub fn new() -> Self {
        let (offset, size) = unsafe { read_ledger_meta() };

        let ledger_meta = {
            let memory = 0 as *const u8;
            let slice = unsafe {
                let start = memory.offset(offset as isize);
                core::slice::from_raw_parts(start, size as usize)
            };
            
            Some(stellar_xdr::next::LedgerCloseMeta::from_xdr(slice, Limits::none()).unwrap())
        };
        
        Self { xdr: ledger_meta, inner_soroban_host: soroban_sdk::Env::default() }
    }

    /// New empty instance of the zephyr client.
    pub fn empty() -> Self {
        Self { xdr: None, inner_soroban_host: soroban_sdk::Env::default() }
    }

    //
    // Functions-only code
    //
    
    /// Send a result to the host.
    /// 
    /// This function has no effect when used in ingestion zephyr programs
    /// and should only be used with serverless functions as target.
    /// 
    pub fn conclude<T: Serialize>(&self, result: T) {
        let v = bincode::serialize(&serde_json::to_string(&result).unwrap()).unwrap();
        
        unsafe {
            conclude_host(v.as_ptr() as i64, v.len() as i64)
        }
    }
}
