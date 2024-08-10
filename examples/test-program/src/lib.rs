//! First SDK-side local Zephyr test.
//! This is a reference of how you can test against specific
//! situations locally.

use std::fmt::format;

use zephyr_sdk::{
    prelude::*,
    soroban_sdk::{
        xdr::{ScVal, ScVec},
        Symbol,
    },
    DatabaseDerive, EnvClient,
};

#[derive(DatabaseDerive, Debug)]
#[with_name("events")]
pub struct StoredEvent {
    // note: we want to clearly distinguish between the various types of
    // SAC events so we store the first topic separately as a string
    topic1: String,
    remaining: ScVal,
    data: ScVal,
}

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();

    for event in env.reader().pretty().soroban_events() {
        // Note: we don't want to index a specific contract, so we want to make
        // sure that we don't assume anything about the events structure.
        if let Some(topic1) = event.topics.get(0) {
            if let Ok(action) = env.try_from_scval::<Symbol>(topic1) {
                if action == Symbol::new(&env.soroban(), "transfer") {
                    let remaining_topics = vec![
                        event.topics.get(1).unwrap_or(&ScVal::Void).clone(),
                        event.topics.get(2).unwrap_or(&ScVal::Void).clone(),
                        event.topics.get(3).unwrap_or(&ScVal::Void).clone(),
                    ];

                    let remaining = ScVal::Vec(Some(ScVec(remaining_topics.try_into().unwrap())));
                    let event = StoredEvent {
                        topic1: "transfer".into(),
                        remaining,
                        data: event.data,
                    };
                    env.log().debug(format!("Got transfer {:?}", event), None);
                    env.put(&event);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn test() {
    let env = EnvClient::empty();
    let account = env.read_account_from_ledger(stellar_strkey::ed25519::PublicKey::from_string("GDEAZIGD6LFY64O7PD5MIPDWY2WAGSWZHEUEUSFKK2T3MBBBYQMPSER4").unwrap().0);
    env.log().debug(format!("{:?}", account), None)
}

#[cfg(test)]
mod test {
    use ledger_meta_factory::{Transition, TransitionPretty};
    use stellar_xdr::next::{Hash, Int128Parts, ScSymbol, ScVal};
    use zephyr_sdk::testutils::TestHost;

    fn build_transition() -> Transition {
        let mut transition = TransitionPretty::new();
        transition.inner.set_sequence(2000);
        transition
            .contract_event(
                "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
                vec![
                    ScVal::Symbol(ScSymbol("transfer".try_into().unwrap())),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash([8; 32]))),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash([1; 32])))
                ],
                ScVal::I128(Int128Parts {
                    hi: 0,
                    lo: 100000000,
                }),
            )
            .unwrap();

            transition
            .contract_event(
                "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
                vec![
                    ScVal::Symbol(ScSymbol("other_action".try_into().unwrap())),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash([8; 32]))),
                    ScVal::Address(stellar_xdr::next::ScAddress::Contract(Hash([1; 32])))
                ],
                ScVal::I128(Int128Parts {
                    hi: 0,
                    lo: 100000000,
                }),
            )
            .unwrap();

        transition.inner
    }

    #[tokio::test]
    async fn test_storage() {
        let env = TestHost::default();
        // Note: this is a default postgres connection string. If you're on production
        // (or even public-facing dev) make sure not to share this string and use an
        // environment variable.
        let mut db = env.database("postgres://postgres:postgres@localhost:5432");
        let mut program = env.new_program("./target/wasm32-unknown-unknown/release/test_program.wasm");
        let transition = build_transition();
        program.set_transition(transition);

        // Create a new ephemeral table in the local database.
        let created = db
            .load_table(0, "events", vec!["topic1", "remaining", "data"])
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

        let invocation = program.invoke_vm("on_close").await;
        assert!(invocation.is_ok());
        let inner_invocation = invocation.unwrap();
        assert!(inner_invocation.is_ok());

        println!("Elapsed: {:?}", start.elapsed());

        // A new row has been indexed in the database.
        assert_eq!(db.get_rows_number(0, "events").await.unwrap(), 1);

        // Drop the connection and all the noise created in the local database.
        db.close().await;
    }
}
