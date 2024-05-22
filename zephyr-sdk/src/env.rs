use std::fmt::Debug;

use rs_zephyr_common::{http::AgnosticRequest, wrapping::WrappedMaxBytes, RelayedMessageRequest};
use serde::{Deserialize, Serialize};
use soroban_sdk::{xdr::{AccountId, ContractEvent, DiagnosticEvent, Hash, HostFunction, InvokeContractArgs, LedgerEntry, Limits, PublicKey, ReadXdr, ScVal, ScVec, SorobanAuthorizationEntry, SorobanTransactionData, VecM, WriteXdr}, TryIntoVal, Val};

use crate::{database::{Database, DatabaseInteract, UpdateTable}, external::{self, conclude_host, read_ledger_meta, scval_to_valid_host_val, soroban_simulate_tx, tx_send_message}, logger::EnvLogger, Condition, MetaReader, SdkError, TableRows};


/// Zephyr's host environment client.
#[derive(Clone)]
pub struct EnvClient {
    xdr: Option<soroban_sdk::xdr::LedgerCloseMeta>,
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

    /// Converts an ScVal into a soroban host object.
    /// Returns a Soroban Val.
    /// Panics when the conversion fails.
    pub fn from_scval<T: soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>(&self, scval: &soroban_sdk::xdr::ScVal) -> T {
        self.scval_to_valid_host_val(scval).unwrap()
    }

    /// Converts an environment object to the corresponding scval
    /// xdr representation.
    pub fn to_scval<T: soroban_sdk::TryIntoVal<soroban_sdk::Env, soroban_sdk::Val>>(&self, val: T) -> soroban_sdk::xdr::ScVal {
        let val: soroban_sdk::Val = val.try_into_val(self.soroban()).unwrap();
        let val_payload = val.get_payload() as i64;

        let (status, offset, size) = unsafe {
            external::valid_host_val_to_scval(val_payload)
        };
        
        SdkError::express_from_status(status).unwrap();
        let xdr = {
            let memory: *const u8 = offset as *const u8;

            let slice = unsafe {
                core::slice::from_raw_parts(memory, size as usize)
            };

            soroban_sdk::xdr::ScVal::from_xdr(slice, Limits::none()).unwrap()
        };

        xdr
    }
    

    /// Converts an ScVal into a soroban host object.
    /// Returns a result with a Soroban Val.
    pub fn scval_to_valid_host_val<T: soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>>(&self, scval: &soroban_sdk::xdr::ScVal) -> Result<T, SdkError> {
        let val_bytes = scval.to_xdr(Limits::none()).unwrap();
        let (offset, size) = (val_bytes.as_ptr() as i64, val_bytes.len() as i64);

        let (status, val) = unsafe { scval_to_valid_host_val(offset, size) };
        SdkError::express_from_status(status)?;

        
        let val = soroban_sdk::Val::from_payload(val as u64);
        
        Ok(T::try_from_val(&self.soroban(), &val).unwrap())
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
    pub fn update(&self) -> UpdateTable {
        UpdateTable::new()
    }

    /// Updates a row to a database table.
    /// 
    /// This function uses the [`DatabaseInteract`] trait
    /// along with the `DatabaseDerive` macro to update the row 
    /// derived from the `DatabaseDerive` struct.
    pub fn update_inner<T: DatabaseInteract>(&self, row: &T, conditions: &[Condition]) {
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
            
            Some(soroban_sdk::xdr::LedgerCloseMeta::from_xdr(slice, Limits::none()).unwrap())
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
    
    /// Read request body into the specified format type.
    pub fn read_request_body<'a, T: Deserialize<'a>>(&self) -> T {
        let (offset, size) = unsafe { read_ledger_meta() };

        let request: &'a str = {
            let memory = 0 as *const u8;
            let slice = unsafe {
                let start = memory.offset(offset as isize);
                core::slice::from_raw_parts(start, size as usize)
            };
            
            bincode::deserialize(slice).unwrap()
        };

        serde_json::from_str(&request).unwrap()
    }

    /// Wrapper around self.simulate. This is a simpler SDK handler which 
    pub fn simulate_contract_call(&self, source: String, contract: [u8; 32], fname: soroban_sdk::Symbol, args: soroban_sdk::Vec<Val>) -> Result<InvokeHostFunctionSimulationResult, SdkError> {
        let source = stellar_strkey::ed25519::PublicKey::from_string(&source).unwrap().0;
        let contract_address = soroban_sdk::xdr::ScAddress::Contract(Hash(contract));
        let ScVal::Symbol(function_name) = self.to_scval(fname) else {panic!()};
        
        let args = {
            let mut vec = Vec::new();
            for arg in args {
                let scval = self.to_scval(arg);
                vec.push(scval);
            }

            vec.try_into().unwrap()
        };
        let function = HostFunction::InvokeContract(InvokeContractArgs {
            contract_address,
            function_name,
            args
        });
        
        self.simulate(source, function)
    }

    /// Simulates any stellar host function.
    pub fn simulate(&self, source: [u8; 32], function: HostFunction) -> Result<InvokeHostFunctionSimulationResult, SdkError> {
        //ce) = source.0; 
        let key_bytes = function.to_xdr(Limits::none()).unwrap();
        let (offset, size) = (key_bytes.as_ptr() as i64, key_bytes.len() as i64);

        let source_parts = WrappedMaxBytes::array_to_max_parts::<4>(&source);
        let (status, inbound_offset, inbound_size) = unsafe {
            soroban_simulate_tx(source_parts[0], source_parts[1], source_parts[2], source_parts[3], offset, size)
        };

        SdkError::express_from_status(status)?;

        let memory: *const u8 = inbound_offset as *const u8;
        let slice = unsafe { core::slice::from_raw_parts(memory, inbound_size as usize) };
        let deser = bincode::deserialize::<InvokeHostFunctionSimulationResult>(slice).map_err(|_| SdkError::Conversion)?;

        Ok(deser)
    }
}


#[derive(Eq, PartialEq, Debug, Deserialize, Serialize)]
pub struct LedgerEntryDiff {
    pub state_before: Option<LedgerEntry>,
    pub state_after: Option<LedgerEntry>,
}

/// Result of simulating `InvokeHostFunctionOp` operation.
#[derive(Debug, Deserialize, Serialize)]
pub struct InvokeHostFunctionSimulationResult {
    /// Result value of the invoked function or error returned for invocation.
    pub invoke_result: std::result::Result<ScVal, ScVal>,
    /// Authorization data, either passed through from the call (when provided),
    /// or recorded during the invocation.
    pub auth: Vec<SorobanAuthorizationEntry>,
    /// All the events that contracts emitted during invocation.
    /// Empty for failed invocations.
    pub contract_events: Vec<ContractEvent>,
    /// Diagnostic events recorded during simulation.
    /// This is populated when diagnostics is enabled and even when the
    /// invocation fails.
    pub diagnostic_events: Vec<DiagnosticEvent>,
    /// Soroban transaction extension containing simulated resources and
    /// the estimated resource fee.
    /// `None` for failed invocations.
    pub transaction_data: Option<SorobanTransactionData>,
    /// The number of CPU instructions metered during the simulation,
    /// without any adjustments applied.
    /// This is expected to not match `transaction_data` in case if
    /// instructions are adjusted via `SimulationAdjustmentConfig`.
    pub simulated_instructions: u32,
    /// The number of memory bytes metered during the simulation,
    /// without any adjustments applied.
    pub simulated_memory: u32,
    /// Differences for any RW entries that have been modified during
    /// the transaction execution.
    /// Empty for failed invocations.
    pub modified_entries: Vec<LedgerEntryDiff>,
}
