use std::time::Instant;

use zksync_types::L1BatchNumber;

#[derive(Debug, Clone)]
pub struct RecursionTipMetadata {
    pub l1_batch_number: L1BatchNumber,
    pub final_node_proof_job_ids: Vec<(u8, u32)>,
    pub start_time: Instant,
    pub pick_time: Instant,
}
