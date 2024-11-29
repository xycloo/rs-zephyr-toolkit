//! Utilities for working with common data patterns.
//!
use crate::{EnvClient, SdkError};
use ed25519_dalek::{
    ed25519::signature::{Keypair, SignerMut},
    SigningKey, VerifyingKey,
};
use sha2::{Digest, Sha256};
use soroban_sdk::{
    xdr::{
        self, DecoratedSignature, Hash, HashIdPreimage, HashIdPreimageSorobanAuthorization,
        Int128Parts, LedgerFootprint, LedgerKey, Limits, ScMapEntry, ScString, ScSymbol, ScVal,
        ScVec, Signature, SignatureHint, SorobanAuthorizedInvocation, Transaction,
        TransactionEnvelope, TransactionSignaturePayload,
        TransactionSignaturePayloadTaggedTransaction, TransactionV1Envelope, VecM, WriteXdr,
    },
    Address,
};

/// Returns an allocated String object starting from a Soroban SDK Address object.
pub fn address_to_alloc_string(env: &EnvClient, address: soroban_sdk::Address) -> String {
    soroban_string_to_alloc_string(env, address.to_string())
}

/// Builds an address from a slice.
pub fn address_from_str(env: &EnvClient, address: &str) -> Address {
    Address::from_string(&soroban_sdk::String::from_str(env.soroban(), address))
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
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
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
        from.to_string()
            .try_into()
            .map_err(|_| SdkError::Conversion)?,
    )))
}

/// Hash a stellar transaction.
pub fn sha256(payload: &[u8]) -> [u8; 32] {
    Sha256::digest(payload).into()
}

/// Hash a stellar transaction.
pub fn hash_transaction(
    tx: &Transaction,
    network_passphrase: &str,
) -> Result<[u8; 32], xdr::Error> {
    let signature_payload = TransactionSignaturePayload {
        network_id: Hash(Sha256::digest(network_passphrase).into()),
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
    };
    Ok(Sha256::digest(signature_payload.to_xdr(Limits::none())?).into())
}

/// Sign any payload.
pub fn ed25519_sign(secret_key: &str, payload: &[u8]) -> (VerifyingKey, [u8; 64]) {
    let mut signing = SigningKey::from_bytes(
        &stellar_strkey::ed25519::PrivateKey::from_string(secret_key)
            .unwrap()
            .0,
    );

    (
        signing.verifying_key(),
        signing.sign(payload).to_bytes().try_into().unwrap(),
    )
}

/// Sign a stellar transaction.
pub fn sign_transaction(tx: Transaction, network_passphrase: &str, secret_key: &str) -> String {
    let tx_hash = hash_transaction(&tx, network_passphrase).unwrap();
    let (verifying, tx_signature) = ed25519_sign(secret_key, &tx_hash);

    let decorated_signature = DecoratedSignature {
        hint: SignatureHint(verifying.to_bytes()[28..].try_into().unwrap()),
        signature: Signature(tx_signature.try_into().unwrap()),
    };

    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx: tx.clone(),
        signatures: [decorated_signature].try_into().unwrap(),
    });

    envelope.to_xdr_base64(Limits::none()).unwrap()
}

/// Builds an [`HashIdPreimage::SorobanAuthorization`] from the given nonce, signature, and invocation.
pub fn build_authorization_preimage(
    nonce: i64,
    signature_expiration_ledger: u32,
    invocation: SorobanAuthorizedInvocation,
) -> HashIdPreimage {
    HashIdPreimage::SorobanAuthorization(HashIdPreimageSorobanAuthorization {
        network_id: xdr::Hash(Sha256::digest("Test SDF Network ; September 2015").into()),
        nonce,
        signature_expiration_ledger,
        invocation,
    })
}

/// Pushes a key to the read-only footprint
pub fn footprint_read_push(footprint: &mut LedgerFootprint, key: LedgerKey) {
    let mut read = footprint.read_only.to_vec();
    read.push(key);
    footprint.read_only = read.try_into().unwrap();
}

/// Pushes a key to the read-write footprint
pub fn footprint_read_write_push(footprint: &mut LedgerFootprint, key: LedgerKey) {
    let mut read = footprint.read_write.to_vec();
    read.push(key);
    footprint.read_write = read.try_into().unwrap();
}

/// Helper to add both contract code and instance to the footprint.
/// Useful especially for smart accounts.
pub fn add_contract_to_footprint(
    footprint: &mut LedgerFootprint,
    contract_id: &str,
    wasm_hash: &[u8],
) {
    footprint_read_push(
        footprint,
        LedgerKey::ContractData(xdr::LedgerKeyContractData {
            contract: xdr::ScAddress::Contract(xdr::Hash(
                stellar_strkey::Contract::from_string(contract_id)
                    .unwrap()
                    .0,
            )),
            key: xdr::ScVal::LedgerKeyContractInstance,
            durability: xdr::ContractDataDurability::Persistent,
        }),
    );

    footprint_read_push(
        footprint,
        xdr::LedgerKey::ContractCode(xdr::LedgerKeyContractCode {
            hash: xdr::Hash(to_array(wasm_hash.to_vec())),
        }),
    );
}
