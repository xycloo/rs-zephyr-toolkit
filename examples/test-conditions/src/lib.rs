//! First SDK-side local Zephyr test.
//! This is a reference of how you can test against specific
//! situations locally.

use std::fmt::format;

use serde::Serialize;

use zephyr_sdk::{
    prelude::*,
    soroban_sdk::{
        xdr::{ScVal, ScVec},
        Symbol,
    },
    DatabaseDerive, EnvClient,
};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    for (tx, meta) in env.reader().envelopes_with_meta() {
        env.log()
            .debug(format!("Got tx {:?} for meta {:?}", tx, meta), None);
    }
}

#[derive(DatabaseDerive, Debug, Serialize)]
#[with_name("events")]
pub struct StoredEvent {
    idx: i32,
    value: i32,
}

#[no_mangle]
pub extern "C" fn add() {
    let env = EnvClient::empty();

    env.put(&StoredEvent { idx: 1, value: 10 })
}

#[no_mangle]
pub extern "C" fn add2() {
    let env = EnvClient::empty();

    env.put(&StoredEvent { idx: 2, value: 20 })
}

#[no_mangle]
pub extern "C" fn add3() {
    let env = EnvClient::empty();

    env.put(&StoredEvent { idx: 3, value: 30 })
}

#[no_mangle]
pub extern "C" fn add32() {
    let env = EnvClient::empty();

    env.to_scval(zephyr_sdk::soroban_sdk::Val::from_payload(89));

    env.put(&StoredEvent { idx: 3, value: 20 })
}

#[no_mangle]
pub extern "C" fn test() {
    let env = EnvClient::empty();

    env.update()
        .column_equal_to("value", 20)
        .column_equal_to("idx", 2)
        .execute(&StoredEvent { idx: 9, value: 10 })
        .unwrap();
}

#[cfg(test)]
mod test {
    use ledger_meta_factory::{Transition, TransitionPretty};
    use stellar_xdr::next::{Hash, Int128Parts, ScSymbol, ScVal};
    use zephyr_sdk::testutils::TestHost;

    #[tokio::test]
    async fn test_storage() {
        let env = TestHost::default();
        // Note: this is a default postgres connection string. If you're on production
        // (or even public-facing dev) make sure not to share this string and use an
        // environment variable.
        let mut db = env.database("postgres://postgres:postgres@localhost:5432");
        let mut program =
            env.new_program("./target/wasm32-unknown-unknown/release/test_conditions.wasm");

        // Create a new ephemeral table in the local database.
        let created = db
            .load_table(
                0,
                "events",
                vec!["idx", "value"],
                Some(vec![(0, "BIGINT"), (1, "BIGINT")]),
            )
            .await;

        // note this is a very strict check since it makes sure that the table
        // didn't previously exist. It's recommended to enable it only when you're
        // sure your program is executing correctly, else you'll have to manually
        // drop the database tables in case the below assertions are failing.
        assert!(created.is_ok());

        // We make sure that there are no pre-existing rows in the table.
        assert_eq!(db.get_rows_number(0, "events").await.unwrap(), 0);

        // We make sure first that there haven't been any issues in the host by asserting
        // that the outer result is ok.
        // Then we assert that there was no error on the guest side (inner result) too.
        let start = std::time::Instant::now();

        let invocation = program.invoke_vm("add").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        println!("{}", inner_invocation.unwrap().1);

        let invocation = program.invoke_vm("add2").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        println!("{}", inner_invocation.unwrap().1);

        let invocation = program.invoke_vm("add3").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        println!("{}", inner_invocation.unwrap().1);

        let invocation = program.invoke_vm("add32").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        // assert!(inner_invocation.is_ok());

        println!("{}", inner_invocation.unwrap().1);

        println!("Elapsed: {:?}", start.elapsed());

        // A new row has been indexed in the database.
        //assert_eq!(db.get_rows_number(0, "events").await.unwrap(), 4);

        let invocation = program.invoke_vm("test").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        println!("{}", inner_invocation.unwrap().1);

        // Drop the connection and all the noise created in the local database.
        db.close().await;
    }
}

/*#[test]
fn test_() {
    use stellar_xdr::next::{
        ContractDataDurability, Hash, LedgerKey, LedgerKeyContractData, Limits, ScVal, ScVec,
    };

    let contract_hash = Hash([
        0x89, 0x5b, 0x6c, 0x84, 0xb7, 0x0d, 0x1a, 0x66, 0x79, 0x8c, 0xc0, 0xc4, 0x82, 0xe3, 0xf5,
        0xd0, 0x5e, 0xa0, 0xdf, 0xc4, 0x12, 0xc2, 0x11, 0x8a, 0x65, 0x6a, 0x62, 0xbc, 0x92, 0x9c,
        0x6f, 0x8d,
    ]);

    let key = ScVal::Vec(Some(ScVec(
        vec![
            ScVal::Symbol("ResConfig".try_into().unwrap()),
            ScVal::Address(ScAddress::Contract(Hash(
                stellar_strkey::Contract::from_string(
                    "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
                )
                .unwrap()
                .0,
            ))),
        ]
        .try_into()
        .unwrap(),
    )));

    let key2 = ScVal::Vec(Some(ScVec(
        vec![
            ScVal::Symbol("EmisConfig".try_into().unwrap()),
            ScVal::U32(1),
        ]
        .try_into()
        .unwrap(),
    )));

    let ledger_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(contract_hash),
        key: key.into(),
        durability: ContractDataDurability::Persistent,
    });

    //println!("{:?}", ledger_key.to_xdr_base64(Limits::none()));

    let envelope = TransactionEnvelope::from_xdr_base64("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", Limits::none()).unwrap();

    let TransactionEnvelope::Tx(v1) = envelope else {
        panic!()
    };
    let TransactionExt::V1(soroban) = v1.tx.ext else {
        panic!()
    };
    for entry in soroban.resources.footprint.read_only.to_vec() {
        println!(
            "{:?} {}\n",
            entry,
            entry.to_xdr_base64(Limits::none()).unwrap()
        )
    }
}
*/