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

    env.put(&StoredEvent { idx: 3, value: 20 })
}

#[no_mangle]
pub extern "C" fn test() {
    let env = EnvClient::empty();

    env.conclude(
        &env.read_filter()
            .column_equal_to("value", 20)
            .column_gt("idx", 2)
            .read::<StoredEvent>()
            .unwrap(),
    );
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

        let invocation = program.invoke_vm("add2").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        let invocation = program.invoke_vm("add3").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        let invocation = program.invoke_vm("add32").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        println!("Elapsed: {:?}", start.elapsed());

        // A new row has been indexed in the database.
        assert_eq!(db.get_rows_number(0, "events").await.unwrap(), 4);

        let invocation = program.invoke_vm("test").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.as_ref().unwrap();
        assert!(inner_invocation.is_ok());

        println!("{:?}", invocation.unwrap().unwrap());

        // Drop the connection and all the noise created in the local database.
        db.close().await;
    }
}
