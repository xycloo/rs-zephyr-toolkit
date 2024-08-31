use sha2::{Digest, Sha256};
use soroban_sdk::xdr::{
    ContractEvent, ContractEventBody, FeeBumpTransactionInnerTx, GeneralizedTransactionSet, Hash,
    LedgerCloseMeta, LedgerEntry, LedgerEntryChange, LedgerKey, Limits, ScBytes, ScVal,
    Transaction, TransactionEnvelope, TransactionMeta, TransactionPhase, TransactionResultMeta,
    TransactionResultResult, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, TxSetComponent, VecM, WriteXdr,
};

use crate::EnvClient;

/// Represents all of the entry changes that happened in the
/// ledger close.
#[derive(Clone)]
#[allow(missing_docs)]
pub struct EntryChanges {
    pub state: Vec<LedgerEntry>,
    pub removed: Vec<LedgerKey>,
    pub updated: Vec<LedgerEntry>,
    pub created: Vec<LedgerEntry>,
}

/// Ledger meta reader.
///
/// Aids developers in dealing with raw XDR structures.
///
pub struct MetaReader<'a>(&'a soroban_sdk::xdr::LedgerCloseMeta);

#[allow(missing_docs)]
impl<'a> MetaReader<'a> {
    pub fn new(meta: &'a LedgerCloseMeta) -> Self {
        Self(meta)
    }

    pub fn newtork_id(&self) -> [u8; 32] {
        let env = EnvClient::empty();
        let ScVal::Bytes(ScBytes(val)) = env.to_scval(env.soroban().ledger().network_id()) else {
            panic!()
        };
        val.try_into().unwrap()
    }

    pub fn txhash_by_transaction(&self, tx_envelope: &TransactionEnvelope) -> [u8; 32] {
        let network = self.newtork_id();
        let tagged = match tx_envelope {
            TransactionEnvelope::Tx(v1) => {
                let tx = v1.tx.clone();
                TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone())
            }
            TransactionEnvelope::TxFeeBump(feebump) => {
                TransactionSignaturePayloadTaggedTransaction::TxFeeBump(feebump.tx.clone())
            }
            _ => panic!(),
        };

        let payload = TransactionSignaturePayload {
            network_id: Hash(network),
            tagged_transaction: tagged,
        };

        let mut hasher = Sha256::new();
        hasher.update(payload.to_xdr(Limits::none()).unwrap());
        let result: [u8; 32] = hasher.finalize().into();
        result
    }

    pub fn ledger_sequence(&self) -> u32 {
        match &self.0 {
            LedgerCloseMeta::V1(v1) => v1.ledger_header.header.ledger_seq,
            LedgerCloseMeta::V0(v0) => v0.ledger_header.header.ledger_seq,
        }
    }

    pub fn ledger_timestamp(&self) -> u64 {
        match &self.0 {
            LedgerCloseMeta::V1(v1) => v1.ledger_header.header.scp_value.close_time.0,
            LedgerCloseMeta::V0(v0) => v0.ledger_header.header.scp_value.close_time.0,
        }
    }

    // todo: add handles for other entries.

    pub fn envelopes(&self) -> Vec<TransactionEnvelope> {
        match &self.0 {
            LedgerCloseMeta::V0(v0) => v0.tx_set.txs.to_vec(),
            LedgerCloseMeta::V1(v1) => {
                let phases = match &v1.tx_set {
                    GeneralizedTransactionSet::V1(v1) => &v1.phases,
                };

                let mut envelopes = Vec::new();

                for phase in phases.iter() {
                    match phase {
                        TransactionPhase::V0(v0) => {
                            for txset_component in v0.iter() {
                                match txset_component {
                                    TxSetComponent::TxsetCompTxsMaybeDiscountedFee(
                                        txset_maybe_discounted_fee,
                                    ) => {
                                        for tx_envelope in txset_maybe_discounted_fee.txs.to_vec() {
                                            envelopes.push(tx_envelope)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                envelopes
            }
        }
    }

    pub fn envelopes_with_meta(&self) -> Vec<(&TransactionEnvelope, TransactionResultMeta)> {
        let mut composed = Vec::new();
        let processing = self.tx_processing();

        match &self.0 {
            LedgerCloseMeta::V0(_) => (), // todo
            LedgerCloseMeta::V1(v1) => {
                let phases = match &v1.tx_set {
                    GeneralizedTransactionSet::V1(v1) => &v1.phases,
                };

                for phase in phases.iter() {
                    match phase {
                        TransactionPhase::V0(v0) => {
                            for txset_component in v0.iter() {
                                match txset_component {
                                    TxSetComponent::TxsetCompTxsMaybeDiscountedFee(
                                        txset_maybe_discounted_fee,
                                    ) => {
                                        for (idx, tx_envelope) in
                                            txset_maybe_discounted_fee.txs.iter().enumerate()
                                        {
                                            let txhash = self.txhash_by_transaction(tx_envelope);

                                            let mut tprocessing = None;
                                            for meta in processing.clone() {
                                                if meta.result.transaction_hash.0 == txhash {
                                                    tprocessing = Some(meta)
                                                }
                                            }
                                            composed.push((
                                                tx_envelope,
                                                tprocessing
                                                    .unwrap_or(processing[idx].clone())
                                                    .clone(),
                                            ))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        composed
    }

    pub fn tx_processing(&self) -> Vec<TransactionResultMeta> {
        match &self.0 {
            LedgerCloseMeta::V1(v1) => v1.tx_processing.to_vec(),
            LedgerCloseMeta::V0(v0) => v0.tx_processing.to_vec(),
        }
    }

    pub fn v1_success_ledger_entries(&self) -> EntryChanges {
        let mut state_entries = Vec::new();
        let mut removed_entries = Vec::new();
        let mut updated_entries = Vec::new();
        let mut created_entries = Vec::new();

        match &self.0 {
            LedgerCloseMeta::V0(_) => (),
            LedgerCloseMeta::V1(v1) => {
                for tx_processing in v1.tx_processing.iter() {
                    let result = &tx_processing.result.result.result;
                    let success = match result {
                        TransactionResultResult::TxSuccess(_) => true,
                        TransactionResultResult::TxFeeBumpInnerSuccess(_) => true,
                        _ => false,
                    };

                    if success {
                        match &tx_processing.tx_apply_processing {
                            TransactionMeta::V3(meta) => {
                                let ops = &meta.operations;

                                for operation in ops.clone().into_vec() {
                                    for change in operation.changes.0.iter() {
                                        match &change {
                                            LedgerEntryChange::State(state) => {
                                                state_entries.push(state.clone())
                                            }
                                            LedgerEntryChange::Created(created) => {
                                                created_entries.push(created.clone())
                                            }
                                            LedgerEntryChange::Updated(updated) => {
                                                updated_entries.push(updated.clone())
                                            }
                                            LedgerEntryChange::Removed(removed) => {
                                                removed_entries.push(removed.clone())
                                            }
                                        };
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        };

        EntryChanges {
            state: state_entries,
            removed: removed_entries,
            updated: updated_entries,
            created: created_entries,
        }
    }

    pub fn v1_ledger_entries(&self) -> EntryChanges {
        let mut state_entries = Vec::new();
        let mut removed_entries = Vec::new();
        let mut updated_entries = Vec::new();
        let mut created_entries = Vec::new();

        match &self.0 {
            LedgerCloseMeta::V0(_) => (),
            LedgerCloseMeta::V1(v1) => {
                for tx_processing in v1.tx_processing.iter() {
                    match &tx_processing.tx_apply_processing {
                        TransactionMeta::V3(meta) => {
                            let ops = &meta.operations;

                            for operation in ops.clone().into_vec() {
                                for change in operation.changes.0.iter() {
                                    match &change {
                                        LedgerEntryChange::State(state) => {
                                            state_entries.push(state.clone())
                                        }
                                        LedgerEntryChange::Created(created) => {
                                            created_entries.push(created.clone())
                                        }
                                        LedgerEntryChange::Updated(updated) => {
                                            updated_entries.push(updated.clone())
                                        }
                                        LedgerEntryChange::Removed(removed) => {
                                            removed_entries.push(removed.clone())
                                        }
                                    };
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        };

        EntryChanges {
            state: state_entries,
            removed: removed_entries,
            updated: updated_entries,
            created: created_entries,
        }
    }

    pub fn soroban_events(&self) -> Vec<ContractEvent> {
        let mut events = Vec::new();

        for result in self.tx_processing() {
            if let TransactionMeta::V3(v3) = &result.tx_apply_processing {
                if let Some(soroban) = &v3.soroban_meta {
                    for event in soroban.events.iter() {
                        events.push(event.clone())
                    }
                }
            }
        }

        events
    }

    pub fn pretty(&self) -> PrettyMetaReader {
        PrettyMetaReader { inner: self }
    }
}

/// Pretty representation of a Soroban event.
#[derive(Clone, Debug)]
pub struct PrettyContractEvent {
    /// Event xdr that derived this object.
    pub raw: ContractEvent,

    /// Contract address that emitted the event.
    pub contract: [u8; 32],

    /// Contract event topics.
    pub topics: VecM<ScVal>,

    /// Contract event data
    pub data: ScVal,
}

impl From<ContractEvent> for PrettyContractEvent {
    fn from(value: ContractEvent) -> Self {
        let ContractEventBody::V0(event) = &value.body;

        Self {
            contract: value.contract_id.as_ref().unwrap().0,
            topics: event.topics.clone(),
            data: event.data.clone(),
            raw: value,
        }
    }
}
pub struct PrettyMetaReader<'a> {
    inner: &'a MetaReader<'a>,
}

impl<'a> PrettyMetaReader<'a> {
    pub fn soroban_events(&self) -> Vec<PrettyContractEvent> {
        let mut events = Vec::new();

        for result in self.inner.tx_processing() {
            if let TransactionMeta::V3(v3) = &result.tx_apply_processing {
                if let Some(soroban) = &v3.soroban_meta {
                    for event in soroban.events.iter() {
                        events.push(event.clone().into())
                    }
                }
            }
        }

        events
    }

    pub fn soroban_events_and_txhash(&self) -> Vec<(PrettyContractEvent, [u8; 32])> {
        let mut events = Vec::new();

        for result in self.inner.tx_processing() {
            let txhash = result.result.transaction_hash.0;

            if let TransactionMeta::V3(v3) = &result.tx_apply_processing {
                if let Some(soroban) = &v3.soroban_meta {
                    for event in soroban.events.iter() {
                        events.push((event.clone().into(), txhash))
                    }
                }
            }
        }

        events
    }
}
