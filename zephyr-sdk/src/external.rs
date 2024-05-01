//! Function definitions of all of the ZephyrVM's host functions.
//! 
//! Note that this does not inlcude Soroban host functions, already defined
//! by the Soroban SDK.

#[allow(dead_code)]
extern "C" {
    #[allow(improper_ctypes)]
    #[link_name = "read_contract_data_entry_by_contract_id_and_key"]
    pub fn read_contract_data_entry_by_contract_id_and_key(contract_part_1: i64, contract_part_2: i64, contract_part_3: i64, contract_part_4: i64, offset: i64, size: i64) -> (i64, i64, i64);

    #[allow(improper_ctypes)]
    #[link_name = "read_contract_instance"]
    pub fn read_contract_instance(contract_part_1: i64, contract_part_2: i64, contract_part_3: i64, contract_part_4: i64) -> (i64, i64, i64);

    #[allow(improper_ctypes)]
    #[link_name = "read_contract_entries_by_contract"]
    pub fn read_contract_entries_by_contract(contract_part_1: i64, contract_part_2: i64, contract_part_3: i64, contract_part_4: i64) -> (i64, i64, i64);

    #[allow(improper_ctypes)]
    #[link_name = "scval_to_valid_host_val"]
    pub fn scval_to_valid_host_val(offset: i64, size: i64) -> (i64, i64);

    #[allow(improper_ctypes)]
    #[link_name = "read_contract_entries_by_contract_to_env"]
    pub fn read_contract_entries_by_contract_to_env(contract_part_1: i64, contract_part_2: i64, contract_part_3: i64, contract_part_4: i64) -> (i64, i64);

    #[allow(improper_ctypes)]
    #[link_name = "conclude"]
    pub fn conclude_host(offset: i64, size: i64);

    #[allow(improper_ctypes)]
    #[link_name = "tx_send_message"]
    pub fn tx_send_message(offset: i64, size: i64) -> i64;

    #[allow(improper_ctypes)] // we alllow as we enabled multi-value
    #[link_name = "read_raw"]
    pub fn read_raw() -> (i64, i64, i64);

    #[allow(improper_ctypes)] // we alllow as we enabled multi-value
    #[link_name = "write_raw"]
    pub fn write_raw() -> i64;

    #[allow(improper_ctypes)] // we alllow as we enabled multi-value
    #[link_name = "update_raw"]
    pub fn update_raw() -> i64;

    #[allow(improper_ctypes)] // we alllow as we enabled multi-value
    #[link_name = "read_ledger_meta"]
    pub fn read_ledger_meta() -> (i64, i64);

    #[link_name = "zephyr_stack_push"]
    pub fn env_push_stack(param: i64);

    #[link_name = "zephyr_logger"]
    pub fn log(param: i64);
}
