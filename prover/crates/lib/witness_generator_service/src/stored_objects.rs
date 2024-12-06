use zkevm_test_harness::boojum::field::goldilocks::GoldilocksField;
use zksync_object_store::{serialize_using_bincode, Bucket, StoredObject};
use zksync_prover_fri_types::{
    circuit_definitions::{
        circuit_definitions::base_layer::ZkSyncBaseLayerClosedFormInput,
        encodings::recursion_request::RecursionQueueSimulator,
    },
    keys::ClosedFormInputKey,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ClosedFormInputWrapper(
    pub(crate) Vec<ZkSyncBaseLayerClosedFormInput<GoldilocksField>>,
    pub(crate) RecursionQueueSimulator<GoldilocksField>,
);

impl StoredObject for ClosedFormInputWrapper {
    const BUCKET: Bucket = Bucket::LeafAggregationWitnessJobsFri;
    type Key<'a> = ClosedFormInputKey;

    fn encode_key(key: Self::Key<'_>) -> String {
        let ClosedFormInputKey {
            block_number,
            circuit_id,
        } = key;
        format!("closed_form_inputs_{block_number}_{circuit_id}.bin")
    }

    serialize_using_bincode!();
}
