//! Canonical event type IDs and shared event structures.
//!
//! All events share a fixed-size [`EventHeader`] followed by a
//! type-specific payload.  The kernel-space eBPF program writes
//! these structs directly into the ring-buffer; the user-space
//! [`ProbeRuntime`] reads and decodes them.

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct EventHeader {
    /// Nanosecond timestamp.
    pub ts_ns: u64,
    /// One of the `EVENT_TYPE_*` constants.
    pub event_type: u16,
    /// Struct layout version; currently `1`.
    pub version: u16,
    /// CPU id where the event was recorded.
    pub cpu_id: u32,
    /// Process ID.
    pub pid: u32,
    /// Thread ID.
    pub tid: u32,
    /// User ID.
    pub uid: u32,
    /// Group ID.
    pub gid: u32,
    /// cgroup v2 ID.
    pub cgroup_id: u64,
    /// Mount namespace inode.
    pub mount_ns: u64,
    /// Byte size of the type-specific payload that follows this header.
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

// ---------------------------------------------------------------------------
// Syscall events
// ---------------------------------------------------------------------------

/// Syscall trace event (entry + exit emitted separately).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct SyscallEvent {
    pub hdr: EventHeader,
    /// Syscall number (e.g. 1 = read, 0 = read, 2 = write on x86).
    pub syscall_id: u32,
    /// 0 = entry, 1 = exit.
    pub phase: u8,
    pub flags: u8,
    pub args: [u64; 6],
    /// Return value (only valid on exit phase).
    pub ret: i64,
}

// ---------------------------------------------------------------------------
// I/O events
// ---------------------------------------------------------------------------

/// Block I/O trace event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct IoEvent {
    pub hdr: EventHeader,
    /// 0 = read, 1 = write.
    pub operation: u8,
    pub opcode: u8,
    pub sector: u64,
    pub num_sectors: u32,
    /// I/O latency in nanoseconds (only valid on completion).
    pub latency_ns: u32,
    pub ret: i64,
}

// ---------------------------------------------------------------------------
// Scheduler events
// ---------------------------------------------------------------------------

/// Scheduler trace event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct SchedEvent {
    pub hdr: EventHeader,
    /// 0 = switch, 1 = wakeup, 2 = fork, 3 = exit.
    pub kind: u8,
    pub prev_pid: u32,
    pub next_pid: u32,
    /// Scheduling latency in nanoseconds.
    pub latency_ns: u64,
}

// ---------------------------------------------------------------------------
// Network events
// ---------------------------------------------------------------------------

/// Network socket trace event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct NetEvent {
    pub hdr: EventHeader,
    /// 0 = send, 1 = recv.
    pub direction: u8,
    pub protocol: u16,
    pub saddr: u32,
    pub daddr: u32,
    pub sport: u16,
    pub dport: u16,
    pub payload_len: u32,
    pub latency_ns: u32,
}

// ---------------------------------------------------------------------------
// Lock events
// ---------------------------------------------------------------------------

/// Lock contention trace event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct LockEvent {
    pub hdr: EventHeader,
    /// 0 = acquire, 1 = release.
    pub op: u8,
    pub lock_id: u32,
    /// Wait time before acquiring (nanoseconds).
    pub wait_ns: u32,
    /// Time the lock was held (nanoseconds).
    pub held_ns: u32,
}

// ---------------------------------------------------------------------------
// Uprobe events
// ---------------------------------------------------------------------------

/// User-space function probe event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct UprobeEvent {
    pub hdr: EventHeader,
    /// Address of the probed function.
    pub func_addr: u64,
    /// Return address stored on stack at call time.
    pub ret_addr: u64,
    pub args: [u64; 6],
}
