use std::{sync::Arc, time::Instant};

use async_trait::async_trait;
use zksync_config::configs::FriWitnessGeneratorConfig;
use zksync_object_store::ObjectStore;
use zksync_prover_dal::{ConnectionPool, Prover, ProverDal};
use zksync_prover_job_processor::{Executor, JobSaver};
use zksync_types::basic_fri_types::AggregationRound;

use crate::{
    artifacts_manager::{ArtifactsManager, RecursionTipArtifactsManager},
    metrics::RECURSION_TIP_METRICS,
};

use super::recursion_tip_executor::RecursionTipExecutor;

pub struct RecursionTipJobSaver {
    connection_pool: ConnectionPool<Prover>,
    object_store: Arc<dyn ObjectStore>,
    public_blob_store: Option<Arc<dyn ObjectStore>>,
    config: FriWitnessGeneratorConfig,
}

impl RecursionTipJobSaver {
    pub fn new(
        connection_pool: ConnectionPool<Prover>,
        object_store: Arc<dyn ObjectStore>,
        public_blob_store: Option<Arc<dyn ObjectStore>>,
        config: FriWitnessGeneratorConfig,
    ) -> Self {
        Self {
            connection_pool,
            object_store,
            public_blob_store,
            config,
        }
    }
}

#[async_trait]
impl JobSaver for RecursionTipJobSaver {
    type ExecutorType = RecursionTipExecutor;

    async fn save_job_result(
        &self,
        data: (
            anyhow::Result<<Self::ExecutorType as Executor>::Output>,
            <Self::ExecutorType as Executor>::Metadata,
        ),
    ) -> anyhow::Result<()> {
        let (result, metadata) = data;

        match result {
            Ok(recursion_tip_output) => {
                tracing::debug!(
                    "recursion tip job {} finished successfully",
                    metadata.l1_batch_number.0
                );

                let start_time = Instant::now();

                let blob_urls = RecursionTipArtifactsManager::save_to_bucket(
                    metadata.l1_batch_number.0,
                    recursion_tip_output.recursion_tip_circuit.clone(),
                    self.object_store.as_ref(),
                    self.config.shall_save_to_public_bucket,
                    self.public_blob_store.clone(),
                )
                .await;

                RECURSION_TIP_METRICS
                    .blob_save_time
                    .observe(start_time.elapsed());

                tracing::info!(
                    "Saved recursion tip artifacts for job {}",
                    metadata.l1_batch_number.0
                );

                RecursionTipArtifactsManager::save_to_database(
                    &self.connection_pool,
                    metadata.l1_batch_number.0,
                    metadata.pick_time,
                    blob_urls,
                    recursion_tip_output.recursion_tip_circuit,
                )
                .await?;

                RECURSION_TIP_METRICS
                    .full_time
                    .observe(metadata.start_time.elapsed());
            }
            Err(err) => {
                tracing::error!(
                    "Error occurred while processing recursion tip job {}: {:?}",
                    metadata.l1_batch_number.0,
                    err
                );

                self.connection_pool
                    .connection()
                    .await
                    .unwrap()
                    .fri_witness_generator_dal()
                    .mark_witness_job_failed(
                        &err.to_string(),
                        metadata.l1_batch_number.0,
                        AggregationRound::RecursionTip,
                    )
                    .await;
            }
        }

        Ok(())
    }
}
