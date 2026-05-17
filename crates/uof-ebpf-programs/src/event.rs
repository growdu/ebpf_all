//! Event type IDs and shared event structures for eBPF programs.
//!
//! These are defined here (not in uof-ebpf) because uof-ebpf has serde
//! which requires std, but eBPF programs must be no_std compatible.

use core::default::Default;

// ---------------------------------------------------------------------------
// Event type constants
// ---------------------------------------------------------------------------

/// Syscall entry/exit event.
pub const EVENT_TYPE_SYSCALL: u16 = 1;
/// Block I/O event.
pub const EVENT_TYPE_IO: u16 = 2;
/// Scheduler event.
pub const EVENT_TYPE_SCHED: u16 = 3;
/// Network socket event.
pub const EVENT_TYPE_NET: u16 = 4;
/// Lock contention event.
pub const EVENT_TYPE_LOCK: u16 = 5;
/// User-space probe event.
pub const EVENT_TYPE_UPROBE: u16 = 6;

// ---------------------------------------------------------------------------
// Event header
// ---------------------------------------------------------------------------

/// Fixed-size header written at the start of every ring-buffer event.
/// The layout must be stable — it is shared between kernel and user space.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EventHeader {
    pub ts_ns: u64,
    pub event_type: u16,
    pub version: u16,
    pub cpu_id: u32,
    pub pid: u32,
    pub tid: u32,
    pub uid: u32,
    pub gid: u32,
    pub cgroup_id: u64,
    pub mount_ns: u64,
    pub payload_len: u32,
}

impl Default for EventHeader {
    fn default() -> Self {
        Self {
            ts_ns: 0,
            event_type: 0,
            version: 1,
            cpu_id: 0,
            pid: 0,
            tid: 0,
            uid: 0,
            gid: 0,
            cgroup_id: 0,
            mount_ns: 0,
            payload_len: 0,
        }
    }
}

/// Syscall trace event (entry + exit emitted separately).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SyscallEvent {
    pub hdr: EventHeader,
    pub syscall_id: u32,
    pub phase: u8,
    pub flags: u8,
    pub args: [u64; 6],
    pub ret: i64,
}

/// Block I/O trace event.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IoEvent {
    pub hdr: EventHeader,
    pub operation: u8,
    pub opcode: u8,
    pub sector: u64,
    pub num_sectors: u32,
    pub latency_ns: u32,
    pub ret: i64,
}

/// Scheduler trace event.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SchedEvent {
    pub hdr: EventHeader,
    pub kind: u8,
    pub prev_pid: u32,
    pub next_pid: u32,
    pub latency_ns: u64,
}

/// Network socket trace event.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct NetEvent {
    pub hdr: EventHeader,
    pub direction: u8,
    pub protocol: u16,
    pub saddr: u32,
    pub daddr: u32,
    pub sport: u16,
    pub dport: u16,
    pub payload_len: u32,
    pub latency_ns: u32,
}

/// Lock contention trace event.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LockEvent {
    pub hdr: EventHeader,
    pub op: u8,
    pub lock_id: u32,
    pub wait_ns: u32,
    pub held_ns: u32,
}

/// User-space function probe event.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UprobeEvent {
    pub hdr: EventHeader,
    pub func_addr: u64,
    pub ret_addr: u64,
    pub args: [u64; 6],
}