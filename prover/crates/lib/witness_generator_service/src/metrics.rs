use std::time::Duration;

use vise::{Buckets, Histogram, Metrics};

#[derive(Debug, Metrics)]
#[metrics(prefix = "recursion_tip")]
/// Metrics for recursion tip witness generator execution
pub struct RecursionTipMetrics {
    /// How long does it take to fetch artifacts blob?
    #[metrics(buckets = Buckets::LATENCIES)]
    pub blob_fetch_time: Histogram<Duration>,
    /// How long does it take to save artifacts blob?
    #[metrics(buckets = Buckets::LATENCIES)]
    pub blob_save_time: Histogram<Duration>,
    /// How long does it take to pick inputs?
    #[metrics(buckets = Buckets::LATENCIES)]
    pub pick_time: Histogram<Duration>,
    /// How long does it take to generate witness?
    #[metrics(buckets = Buckets::LATENCIES)]
    pub witness_generation_time: Histogram<Duration>,
    /// How long does it take to save results?
    #[metrics(buckets = Buckets::LATENCIES)]
    pub save_time: Histogram<Duration>,
    /// How long does it take to process the full job?
    #[metrics(buckets = Buckets::LATENCIES)]
    pub full_time: Histogram<Duration>,
}

#[vise::register]
pub static RECURSION_TIP_METRICS: vise::Global<RecursionTipMetrics> = vise::Global::new();
