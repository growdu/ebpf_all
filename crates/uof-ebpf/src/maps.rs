//! eBPF map definitions shared between kernel and user space.
//!
//! ## Primary Maps
//!
//! - [`RINGBUF_NAME`] – primary event streaming channel
//! - [`PERF_EVENT_ARRAY_NAME`] – perf counter array
//! - [`PID_CTX_NAME`] – per-pid context cache
//! - [`POLICY_NAME`] – sampling / filtering rules
//! - [`CONFIG_NAME`] – probe enable/disable flags

pub const RINGBUF_NAME: &str = "uof_events";
pub const PERF_EVENT_ARRAY_NAME: &str = "uof_perf_events";
pub const PID_CTX_NAME: &str = "uof_pid_ctx";
pub const POLICY_NAME: &str = "uof_policy";
pub const CONFIG_NAME: &str = "uof_config";

/// Per-pid cached metadata (populated from /proc on first-seen pid).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PidContext {
    pub host_pid: u32,
    pub start_time_ns: u64,
    pub comm: [u8; 16],
    pub parent_pid: u32,
}

/// Global sampling policy stored in [`POLICY_NAME`].
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SamplingPolicy {
    /// Cap on events per second per probe.
    pub events_per_sec: u32,
    /// Sampling probability numerator out of 10000 (e.g. 1000 = 10%).
    pub probability_numerator: u16,
    pub flags: u16,
}
