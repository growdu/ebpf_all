//! 通用辅助函数，供所有 eBPF 探针使用

use aya_ebpf::helpers::{bpf_ktime_get_ns, bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_get_smp_processor_id};

use crate::event::EventHeader;

/// Get the current process ID from bpf_get_current_pid_tgid.
pub fn current_pid() -> u32 {
    bpf_get_current_pid_tgid() >> 32
}

/// Create an EventHeader with the given event type.
pub fn make_header(event_type: u16, payload_len: u32) -> EventHeader {
    let ts = unsafe { bpf_ktime_get_ns() };
    EventHeader {
        ts_ns: ts,
        event_type,
        version: 1,
        cpu_id: 0,
        pid: current_pid(),
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len,
    }
}

/// Submit an event to the ring buffer.
pub unsafe fn submit_event<T>(event: &T) {
    let size = core::mem::size_of::<T>();
    crate::maps::ringbuf().output(event, 0);
}