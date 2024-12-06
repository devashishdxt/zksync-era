use std::time::Instant;

use zkevm_test_harness::zkevm_circuits::recursion::recursion_tip::RecursionTipConfig;
use zksync_prover_fri_types::circuit_definitions::{
    circuit_definitions::recursion_layer::{
        recursion_tip::RecursionTipCircuit, ZkSyncRecursiveLayerCircuit,
    },
    recursion_layer_proof_config,
};
use zksync_prover_job_processor::Executor;

use crate::metrics::RECURSION_TIP_METRICS;

use super::types::{RecursionTipInput, RecursionTipMetadata, RecursionTipOutput};

pub struct RecursionTipExecutor {}

impl RecursionTipExecutor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Executor for RecursionTipExecutor {
    type Input = RecursionTipInput;

    type Output = RecursionTipOutput;

    type Metadata = RecursionTipMetadata;

    fn execute(
        &self,
        input: Self::Input,
        metadata: Self::Metadata,
    ) -> anyhow::Result<Self::Output> {
        let start_time = Instant::now();

        tracing::info!(
            "Starting recursion tip execution for block {}",
            metadata.l1_batch_number.0,
        );

        let config = RecursionTipConfig {
            proof_config: recursion_layer_proof_config(),
            vk_fixed_parameters: input.node_vk.clone().into_inner().fixed_parameters,
            _marker: std::marker::PhantomData,
        };

        let recursive_tip_circuit = RecursionTipCircuit {
            witness: input.recursion_tip_witness,
            config,
            transcript_params: (),
            _marker: std::marker::PhantomData,
        };

        tracing::info!(
            "Finished recursion tip job executor on batch {} in {:?}",
            metadata.l1_batch_number.0,
            start_time.elapsed()
        );
        RECURSION_TIP_METRICS
            .witness_generation_time
            .observe(start_time.elapsed());

        Ok(RecursionTipOutput {
            recursion_tip_circuit: ZkSyncRecursiveLayerCircuit::RecursionTipCircuit(
                recursive_tip_circuit,
            ),
        })
    }
}
