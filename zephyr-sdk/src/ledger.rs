use std::fmt::Debug;

use rs_zephyr_common::wrapping::WrappedMaxBytes;
use soroban_sdk::{Map, TryFromVal, Val};
use soroban_sdk::xdr::{LedgerEntryData, Limits, ScVal, WriteXdr};
use crate::{env::EnvClient, ContractDataEntry, ContractDataEntryStellarXDR, external::{read_contract_data_entry_by_contract_id_and_key, read_contract_entries_by_contract, read_contract_entries_by_contract_to_env, read_contract_instance}, SdkError};


impl EnvClient {
    fn express_and_deser_entry(status: i64, offset: i64, size: i64) -> Result<Option<ContractDataEntry>, SdkError> {
        SdkError::express_from_status(status)?;

        let memory: *const u8 = offset as *const u8;
        let slice = unsafe { core::slice::from_raw_parts(memory, size as usize) };

        let deser = bincode::deserialize::<Option<ContractDataEntryStellarXDR>>(slice).map_err(|_| SdkError::Conversion)?;
        if deser.is_none() {
            return Ok(None);
        }
        Ok(Some(deser.unwrap().into()))
    }
    
    /// Returns the instance object of a certain contract from
    /// the host's ledger.
    pub fn read_contract_instance(&self, contract: [u8; 32]) -> Result<Option<ContractDataEntry>, SdkError> {
        let contract_parts = WrappedMaxBytes::array_to_max_parts::<4>(&contract);
        let (status, offset, size) = unsafe { read_contract_instance(contract_parts[0], contract_parts[1], contract_parts[2], contract_parts[3]) };

        Self::express_and_deser_entry(status, offset, size)
    }

    /// Returns the requested entry object of a certain contract 
    /// from the host's ledger.
    pub fn read_contract_entry_by_scvalkey(&self, contract: [u8; 32], key: ScVal) -> Result<Option<ContractDataEntry>, SdkError> {
        let key_bytes = key.to_xdr(Limits::none()).unwrap();
        let (offset, size) = (key_bytes.as_ptr() as i64, key_bytes.len() as i64);

        let contract_parts = WrappedMaxBytes::array_to_max_parts::<4>(&contract);
        let (status, inbound_offset, inbound_size) = unsafe {
            read_contract_data_entry_by_contract_id_and_key(contract_parts[0], contract_parts[1], contract_parts[2], contract_parts[3], offset, size)
        };

        Self::express_and_deser_entry(status, inbound_offset, inbound_size)
    }

    /// Returns the whole requested entry object of a certain contract 
    /// from the host's ledger.
    pub fn read_full_contract_entry_by_key<T: soroban_sdk::TryIntoVal<soroban_sdk::Env, soroban_sdk::Val>>(&self, contract: [u8; 32], val: T) -> Result<Option<ContractDataEntry>, SdkError> {
        let key = self.to_scval(val);
        let key_bytes = key.to_xdr(Limits::none()).unwrap();
        let (offset, size) = (key_bytes.as_ptr() as i64, key_bytes.len() as i64);

        let contract_parts = WrappedMaxBytes::array_to_max_parts::<4>(&contract);
        let (status, inbound_offset, inbound_size) = unsafe {
            read_contract_data_entry_by_contract_id_and_key(contract_parts[0], contract_parts[1], contract_parts[2], contract_parts[3], offset, size)
        };

        Self::express_and_deser_entry(status, inbound_offset, inbound_size)
    }

    /// Returns the requested entry object of a certain contract 
    /// from the host's ledger.
    pub fn read_contract_entry_by_key<T: soroban_sdk::TryIntoVal<soroban_sdk::Env, soroban_sdk::Val>, R: soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val> + Debug>(&self, contract: [u8; 32], val: T) -> Result<Option<R>, SdkError> {
        let key = self.to_scval(val);
        let key_bytes = key.to_xdr(Limits::none()).unwrap();
        let (offset, size) = (key_bytes.as_ptr() as i64, key_bytes.len() as i64);

        let contract_parts = WrappedMaxBytes::array_to_max_parts::<4>(&contract);
        let (status, inbound_offset, inbound_size) = unsafe {
            read_contract_data_entry_by_contract_id_and_key(contract_parts[0], contract_parts[1], contract_parts[2], contract_parts[3], offset, size)
        };

        let resp = Self::express_and_deser_entry(status, inbound_offset, inbound_size)?;
        println!("{:?}", resp);
        if resp.is_none() {
            return Ok(None);
        }
        
        let LedgerEntryData::ContractData(data) = resp.unwrap().entry.data else {panic!()};

        Ok(Some(self.from_scval::<R>(&data.val)))
    }

    /// Returns all the entry objects of a certain contract 
    /// from the host's ledger.
    pub fn read_contract_entries(&self, contract: [u8; 32]) -> Result<Vec<ContractDataEntry>, SdkError> {
        let contract_parts = WrappedMaxBytes::array_to_max_parts::<4>(&contract);
        
        let (status, offset, size) = unsafe { read_contract_entries_by_contract(contract_parts[0], contract_parts[1], contract_parts[2], contract_parts[3]) };

        SdkError::express_from_status(status)?;

        let memory: *const u8 = offset as *const u8;
        let slice = unsafe { core::slice::from_raw_parts(memory, size as usize) };

        let deser = bincode::deserialize::<Vec<ContractDataEntryStellarXDR>>(slice).map_err(|_| SdkError::Conversion)?;
        Ok(deser.iter().map(|entry| entry.clone().into()).collect())
    }

    /// Returns all the entry objects of a certain contract 
    /// from the host's ledger. This function retuns an iteraror
    /// over Soroban host objects, and should be used along with the
    /// Soroban SDK.
    pub fn read_contract_entries_to_env(&self, env: &soroban_sdk::Env, contract: [u8; 32]) -> Result<Map<Val, Val>, SdkError> {
        let contract_parts = WrappedMaxBytes::array_to_max_parts::<4>(&contract);
        let (status, mapobject ) = unsafe { read_contract_entries_by_contract_to_env(contract_parts[0], contract_parts[1], contract_parts[2], contract_parts[3]) };
        SdkError::express_from_status(status)?;

        let map = Map::try_from_val(env, &Val::from_payload(mapobject as u64)).unwrap();
        Ok(map)
    }
}
