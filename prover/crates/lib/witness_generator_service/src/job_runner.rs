use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use zksync_config::configs::FriWitnessGeneratorConfig;
use zksync_object_store::ObjectStore;
use zksync_prover_dal::{ConnectionPool, Prover};
use zksync_prover_job_processor::{Backoff, BackoffAndCancellable, JobRunner};
use zksync_prover_keystore::keystore::Keystore;
use zksync_types::protocol_version::ProtocolSemanticVersion;

use crate::recursion_tip::{RecursionTipExecutor, RecursionTipJobPicker, RecursionTipJobSaver};

/// Convenience struct helping with building Witness Generator runners.
#[derive(Debug)]
pub struct WgRunnerBuilder {
    connection_pool: ConnectionPool<Prover>,
    object_store: Arc<dyn ObjectStore>,
    keystore: Keystore,
    protocol_version: ProtocolSemanticVersion,
    pod_name: String,
    public_blob_store: Option<Arc<dyn ObjectStore>>,
    config: FriWitnessGeneratorConfig,
    cancellation_token: CancellationToken,
}

impl WgRunnerBuilder {
    pub fn new(
        connection_pool: ConnectionPool<Prover>,
        object_store: Arc<dyn ObjectStore>,
        keystore: Keystore,
        protocol_version: ProtocolSemanticVersion,
        pod_name: String,
        public_blob_store: Option<Arc<dyn ObjectStore>>,
        config: FriWitnessGeneratorConfig,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            connection_pool,
            object_store,
            keystore,
            protocol_version,
            pod_name,
            public_blob_store,
            config,
            cancellation_token,
        }
    }

    pub fn recursion_tip_runner(
        &self,
        count: usize,
    ) -> JobRunner<RecursionTipExecutor, RecursionTipJobPicker, RecursionTipJobSaver> {
        let executor = RecursionTipExecutor::new();
        let job_picker = RecursionTipJobPicker::new(
            self.connection_pool.clone(),
            self.object_store.clone(),
            self.keystore.clone(),
            self.protocol_version,
            self.pod_name.clone(),
        );
        let job_saver = RecursionTipJobSaver::new(
            self.connection_pool.clone(),
            self.object_store.clone(),
            self.public_blob_store.clone(),
            self.config.clone(),
        );

        let backoff = Backoff::default();

        JobRunner::new(
            executor,
            job_picker,
            job_saver,
            count,
            Some(BackoffAndCancellable::new(
                backoff,
                self.cancellation_token.clone(),
            )),
        )
    }
}
