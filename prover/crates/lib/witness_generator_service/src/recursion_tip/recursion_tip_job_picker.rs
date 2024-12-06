use std::{sync::Arc, time::Instant};

use anyhow::Context;
use async_trait::async_trait;
use zkevm_test_harness::{
    boojum::{
        field::{goldilocks::GoldilocksField, Field, U64Representable},
        gadgets::{queue::QueueState, traits::allocatable::CSAllocatable},
    },
    witness::{
        recursive_aggregation::compute_node_vk_commitment,
        utils::take_sponge_like_queue_state_from_simulator,
    },
    zkevm_circuits::{
        recursion::{
            leaf_layer::input::RecursionLeafParametersWitness,
            recursion_tip::input::{RecursionTipInputWitness, RecursionTipInstanceWitness},
        },
        scheduler::aux::BaseLayerCircuitType,
    },
};
use zksync_object_store::ObjectStore;
use zksync_prover_dal::{Connection, ConnectionPool, Prover, ProverDal};
use zksync_prover_fri_types::{
    circuit_definitions::circuit_definitions::recursion_layer::{
        ZkSyncRecursionLayerStorageType, RECURSION_TIP_ARITY,
    },
    keys::ClosedFormInputKey,
};
use zksync_prover_job_processor::{Executor, JobPicker};
use zksync_prover_keystore::{keystore::Keystore, utils::get_leaf_vk_params};
use zksync_types::protocol_version::ProtocolSemanticVersion;

use super::{recursion_tip_executor::RecursionTipExecutor, types::RecursionTipMetadata};

use crate::{
    artifacts_manager::{ArtifactsManager, RecursionTipArtifactsManager},
    metrics::RECURSION_TIP_METRICS,
    recursion_tip::types::RecursionTipInput,
    stored_objects::ClosedFormInputWrapper,
};

pub struct RecursionTipJobPicker {
    connection_pool: ConnectionPool<Prover>,
    object_store: Arc<dyn ObjectStore>,
    keystore: Keystore,
    protocol_version: ProtocolSemanticVersion,
    pod_name: String,
}

impl RecursionTipJobPicker {
    pub fn new(
        connection_pool: ConnectionPool<Prover>,
        object_store: Arc<dyn ObjectStore>,
        keystore: Keystore,
        protocol_version: ProtocolSemanticVersion,
        pod_name: String,
    ) -> Self {
        Self {
            connection_pool,
            object_store,
            keystore,
            protocol_version,
            pod_name,
        }
    }
}

#[async_trait]
impl JobPicker for RecursionTipJobPicker {
    type ExecutorType = RecursionTipExecutor;

    async fn pick_job(
        &mut self,
    ) -> anyhow::Result<
        Option<(
            <Self::ExecutorType as Executor>::Input,
            <Self::ExecutorType as Executor>::Metadata,
        )>,
    > {
        let start_time = Instant::now();
        tracing::info!("Started recursion tip job picker");

        let connection = self
            .connection_pool
            .connection()
            .await
            .context("failed to get db connection")?;

        let mut metadata = match load_recursion_tip_metadata(
            connection,
            self.protocol_version,
            &self.pod_name,
            start_time,
        )
        .await
        {
            None => return Ok(None),
            Some(metadata) => metadata,
        };

        let recursion_tip_proofs = RecursionTipArtifactsManager::get_artifacts(
            &metadata.final_node_proof_job_ids,
            self.object_store.as_ref(),
        )
        .await?;

        RECURSION_TIP_METRICS
            .blob_fetch_time
            .observe(start_time.elapsed());

        let node_vk = self
            .keystore
            .load_recursive_layer_verification_key(
                ZkSyncRecursionLayerStorageType::NodeLayerCircuit as u8,
            )
            .context("get_recursive_layer_vk_for_circuit_type()")?;

        let node_layer_vk_commitment = compute_node_vk_commitment(node_vk.clone());

        let mut recursion_queues = vec![];
        for circuit_id in BaseLayerCircuitType::as_iter_u8() {
            let key = ClosedFormInputKey {
                block_number: metadata.l1_batch_number,
                circuit_id,
            };
            let ClosedFormInputWrapper(_, recursion_queue) = self.object_store.get(key).await?;
            recursion_queues.push((circuit_id, recursion_queue));
        }

        // RECURSION_TIP_ARITY is the maximum amount of proof that a single recursion tip can support.
        // Given recursion_tip has at most 1 proof per circuit, it implies we can't add more circuit types without bumping arity up.
        assert!(
            RECURSION_TIP_ARITY >= recursion_queues.len(),
            "recursion tip received more circuits ({}) than supported ({})",
            recursion_queues.len(),
            RECURSION_TIP_ARITY
        );
        let mut branch_circuit_type_set = [GoldilocksField::ZERO; RECURSION_TIP_ARITY];
        let mut queue_set: [_; RECURSION_TIP_ARITY] =
            std::array::from_fn(|_| QueueState::placeholder_witness());

        for (index, (circuit_id, recursion_queue)) in recursion_queues.iter().enumerate() {
            branch_circuit_type_set[index] =
                GoldilocksField::from_u64_unchecked(*circuit_id as u64);
            queue_set[index] = take_sponge_like_queue_state_from_simulator(recursion_queue);
        }

        let leaf_vk_commits = get_leaf_vk_params(&self.keystore).context("get_leaf_vk_params()")?;
        assert_eq!(
            leaf_vk_commits.len(),
            16,
            "expected 16 leaf vk commits, which corresponds to the numebr of circuits, got {}",
            leaf_vk_commits.len()
        );
        let leaf_layer_parameters: [RecursionLeafParametersWitness<GoldilocksField>; 16] =
            leaf_vk_commits
                .iter()
                .map(|el| el.1.clone())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

        let input = RecursionTipInputWitness {
            leaf_layer_parameters,
            node_layer_vk_commitment,
            branch_circuit_type_set,
            queue_set,
        };

        let recursion_tip_witness = RecursionTipInstanceWitness {
            input,
            vk_witness: node_vk.clone().into_inner(),
            proof_witnesses: recursion_tip_proofs.into(),
        };

        let input = RecursionTipInput {
            recursion_tip_witness,
            node_vk,
        };

        tracing::info!(
            "Finished picking recursion tip job on batch {} in {:?}",
            metadata.l1_batch_number.0,
            start_time.elapsed()
        );
        RECURSION_TIP_METRICS
            .pick_time
            .observe(start_time.elapsed());

        metadata.pick_time = Instant::now();

        Ok(Some((input, metadata)))
    }
}

async fn load_recursion_tip_metadata(
    mut connection: Connection<'_, Prover>,
    protocol_version: ProtocolSemanticVersion,
    pod_name: &str,
    start_time: Instant,
) -> Option<RecursionTipMetadata> {
    let (l1_batch_number, number_of_final_node_jobs) = connection
        .fri_witness_generator_dal()
        .get_next_recursion_tip_witness_job(protocol_version, pod_name)
        .await?;

    let final_node_proof_job_ids = connection
        .fri_prover_jobs_dal()
        .get_final_node_proof_job_ids_for(l1_batch_number)
        .await;

    assert_eq!(
        final_node_proof_job_ids.len(),
        number_of_final_node_jobs as usize,
        "recursion tip witness job was scheduled without all final node jobs being completed; expected {}, got {}",
        number_of_final_node_jobs, final_node_proof_job_ids.len()
    );

    Some(RecursionTipMetadata {
        l1_batch_number,
        final_node_proof_job_ids,
        start_time: start_time,
        pick_time: start_time,
    })
}
