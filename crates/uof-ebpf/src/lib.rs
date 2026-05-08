use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C)]
pub struct RawEventHeader {
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

pub const EVENT_TYPE_SYSCALL: u16 = 1;
