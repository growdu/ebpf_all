# eBPF 探针完整实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成所有 eBPF 探针实现，从 tracepoint 正确读取 kernel 数据并写入 ringbuf

**Architecture:** 计划在 `uof-ebpf-programs` 中创建 `common.rs` 提供通用辅助函数，然后逐个完善各探针模块。探针通过 `#[tracepoint]` 属性挂载到 kernel tracepoints，使用 `ctx.args().add(offset)` 读取 tracepoint 上下文数据，最后通过 `ringbuf().output()` 写入事件。

**重要 API 说明:**
- `TracePointContext` 有 `args()` 方法返回 `*const u8` 指针
- 使用 `*ctx.args().add(offset) as *const T` 读取tracepoint 字段
- 不是使用 `bpf_probe_read`，而是直接指针解引用

**Tech Stack:** aya-ebpf 0.1, bpfel-unknown-none target, Nightly Rust

---

## 文件结构

```
crates/uof-ebpf-programs/src/
├── event.rs      # 事件类型定义 ✅ 不需要修改
├── maps.rs       # RingBuf map ✅ 不需要修改
├── common.rs     # [新增] 通用辅助函数
├── sched.rs      # [完善] 调度器探针 - sched_switch, sched_wakeup, sched_process_fork, sched_process_exit
├── io.rs         # [完善] 块设备 I/O 探针 - block_rq_insert, block_rq_complete, block_rq_issue
├── syscall.rs    # [已完整] ✅
├── lock.rs       # [完善] 锁探针 - lock_acquire, lock_release
├── net.rs        # [完善] 网络探针 - inet_sock_set_state, netif_receive_skb
├── uprobe.rs     # [完善] 用户空间探针 - uprobe, uretprobe
└── lib.rs        # 导出模块，需要更新
```

---

## Task 1: 创建 common.rs 通用辅助模块

**Files:**
- Create: `crates/uof-ebpf-programs/src/common.rs`

- [ ] **Step 1: 创建 common.rs 文件**

```rust
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
```

- [ ] **Step 2: 更新 lib.rs 导出 common 模块**

Modify: `crates/uof-ebpf-programs/src/lib.rs:3`

在 `pub mod syscall;` 后添加:
```rust
pub mod common;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 编译成功，无错误

- [ ] **Step 4: 提交**

```bash
git add crates/uof-ebpf-programs/src/common.rs crates/uof-ebpf-programs/src/lib.rs
git commit -m "feat(ebpf-probes): add common helper module for tracepoint events"
```

---

## Task 2: 完善 sched.rs 调度器探针

**Files:**
- Modify: `crates/uof-ebpf-programs/src/sched.rs`

**实现说明:**
- `sched:sched_switch` tracepoint 字段: prev_pid, prev_state, next_pid, next_prio, next_cpu
- `sched:sched_wakeup` tracepoint 字段: pid, prio, success, target_cpu
- `sched:sched_process_fork` tracepoint 字段: pid, child_pid, clone_flags
- `sched:sched_process_exit` tracepoint 字段: pid, exit_code, exit_signal

- [ ] **Step 1: 查看当前 sched.rs 内容**

当前 sched.rs 已存在，使用 `prev_pid: 0, next_pid: 0` 硬编码。需要完善以读取实际 tracepoint 字段。

- [ ] **Step 2: 重写 sched.rs 使用 ctx.args().add(offset)**

```rust
use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{SchedEvent, EVENT_TYPE_SCHED};
use crate::common::{make_header, submit_event};

// Kind constants
const KIND_SWITCH: u8 = 0;
const KIND_WAKEUP: u8 = 1;
const KIND_FORK: u8 = 2;
const KIND_EXIT: u8 = 3;

/// Scheduler context switch tracepoint.
/// Tracepoint: sched:sched_switch
/// Fields: prev_pid (offset 0), prev_state (offset 8), next_pid (offset 16), next_prio (offset 20), next_cpu (offset 24)
#[tracepoint(target = "sched", name = "sched_switch")]
pub fn handle_sched_switch(ctx: TracePointContext) -> i64 {
    let prev_pid = unsafe { *ctx.args().add(0) as *const u32 };
    let next_pid = unsafe { *ctx.args().add(16) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
    let evt = SchedEvent {
        hdr,
        kind: KIND_SWITCH,
        prev_pid,
        next_pid,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Scheduler wakeup tracepoint.
/// Tracepoint: sched:sched_wakeup
/// Fields: pid (offset 0), prio (offset 4), success (offset 8), target_cpu (offset 12)
#[tracepoint(target = "sched", name = "sched_wakeup")]
pub fn handle_sched_wakeup(ctx: TracePointContext) -> i64 {
    let pid = unsafe { *ctx.args().add(0) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
    let evt = SchedEvent {
        hdr,
        kind: KIND_WAKEUP,
        prev_pid: pid,
        next_pid: 0,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Scheduler process fork tracepoint.
/// Tracepoint: sched:sched_process_fork
/// Fields: pid (offset 0), child_pid (offset 8), clone_flags (offset 16)
#[tracepoint(target = "sched", name = "sched_process_fork")]
pub fn handle_sched_process_fork(ctx: TracePointContext) -> i64 {
    let parent_pid = unsafe { *ctx.args().add(0) as *const u32 };
    let child_pid = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
    let evt = SchedEvent {
        hdr,
        kind: KIND_FORK,
        prev_pid: parent_pid,
        next_pid: child_pid,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Scheduler process exit tracepoint.
/// Tracepoint: sched:sched_process_exit
/// Fields: pid (offset 0), exit_code (offset 4), exit_signal (offset 8)
#[tracepoint(target = "sched", name = "sched_process_exit")]
pub fn handle_sched_process_exit(ctx: TracePointContext) -> i64 {
    let pid = unsafe { *ctx.args().add(0) as *const u32 };

    let hdr = make_header(EVENT_TYPE_SCHED, core::mem::size_of::<SchedEvent>() as u32);
    let evt = SchedEvent {
        hdr,
        kind: KIND_EXIT,
        prev_pid: pid,
        next_pid: 0,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add crates/uof-ebpf-programs/src/sched.rs
git commit -m "feat(ebpf-probes): implement sched tracepoint handlers with actual field reading"
```

---

## Task 3: 完善 io.rs 块设备 I/O 探针

**Files:**
- Modify: `crates/uof-ebpf-programs/src/io.rs`

**实现说明:**
- `block:block_rq_insert` tracepoint 字段: sector, num_sectors, dev, operation
- `block:block_rq_complete` tracepoint 字段: sector, num_sectors, errors, latency
- `block:block_rq_issue` tracepoint 字段: sector, num_sectors, cmd_type

- [ ] **Step 1: 查看当前 io.rs 内容**

当前 io.rs 使用硬编码字段值，需要完善。

- [ ] **Step 2: 重写 io.rs 使用 ctx.args().add(offset)**

```rust
use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{IoEvent, EVENT_TYPE_IO};
use crate::common::{make_header, submit_event};

/// Handle block request insert tracepoint.
/// Tracepoint: block:block_rq_insert
/// Fields: sector (offset 0, u64), nr_sector (offset 8, u32)
/// Emits: operation=0 (insert)
#[tracepoint(target = "block", name = "block_rq_insert")]
pub fn handle_block_rq_insert(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_IO, core::mem::size_of::<IoEvent>() as u32);
    let evt = IoEvent {
        hdr,
        operation: 0,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle block request complete tracepoint.
/// Tracepoint: block:block_rq_complete
/// Fields: sector (offset 0, u64), nr_sector (offset 8, u32), error (offset 12, i32)
/// Emits: operation=1 (complete)
#[tracepoint(target = "block", name = "block_rq_complete")]
pub fn handle_block_rq_complete(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };
    let error = unsafe { *ctx.args().add(12) as *const i32 };

    let hdr = make_header(EVENT_TYPE_IO, core::mem::size_of::<IoEvent>() as u32);
    let evt = IoEvent {
        hdr,
        operation: 1,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: error as i64,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Handle block request issue tracepoint.
/// Tracepoint: block:block_rq_issue
/// Fields: sector (offset 0, u64), nr_sector (offset 8, u32)
/// Emits: operation=2 (issue)
#[tracepoint(target = "block", name = "block_rq_issue")]
pub fn handle_block_rq_issue(ctx: TracePointContext) -> i64 {
    let sector = unsafe { *ctx.args().add(0) as *const u64 };
    let nr_sector = unsafe { *ctx.args().add(8) as *const u32 };

    let hdr = make_header(EVENT_TYPE_IO, core::mem::size_of::<IoEvent>() as u32);
    let evt = IoEvent {
        hdr,
        operation: 2,
        opcode: 0,
        sector,
        num_sectors: nr_sector,
        latency_ns: 0,
        ret: 0,
    };
    unsafe { submit_event(&evt) };
    0
}
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add crates/uof-ebpf-programs/src/io.rs
git commit -m "feat(ebpf-probes): implement block I/O tracepoint handlers"
```

---

## Task 4: 完善 lock.rs 锁探针

**Files:**
- Modify: `crates/uof-ebpf-programs/src/lib.rs` (移除内联 lock 模块)
- Modify: `crates/uof-ebpf-programs/src/lock.rs` (创建独立文件)

**实现说明:**
- `lock:lock_acquire` tracepoint 字段: lock_addr, ret, contended
- `lock:lock_release` tracepoint 字段: lock_addr, wait_time, hold_time

- [ ] **Step 1: 创建 lock.rs 文件**

当前 lock 模块在 lib.rs 内联定义，需要提取为独立文件。

Create: `crates/uof-ebpf-programs/src/lock.rs`

```rust
//! Lock contention tracepoint handlers.
//!
//! Tracepoints: lock:lock_acquire, lock:lock_release

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{LockEvent, EVENT_TYPE_LOCK};
use crate::common::{make_header, submit_event};

/// Lock acquire tracepoint.
/// Tracepoint: lock:lock_acquire
/// Fields: lock_addr (offset 0, u64), ret (offset 8, i32), contended (offset 12, i32)
#[tracepoint(target = "lock", name = "lock_acquire")]
pub fn handle_lock_acquire(ctx: TracePointContext) -> i64 {
    let lock_addr = unsafe { *ctx.args().add(0) as *const u64 };
    let ret = unsafe { *ctx.args().add(8) as *const i32 };
    let contended = unsafe { *ctx.args().add(12) as *const i32 };

    let hdr = make_header(EVENT_TYPE_LOCK, core::mem::size_of::<LockEvent>() as u32);
    let evt = LockEvent {
        hdr,
        op: 0, // acquire
        lock_id: (lock_addr & 0xFFFFFFFF) as u32,
        wait_ns: if contended != 0 { ret as u32 } else { 0 },
        held_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Lock release tracepoint.
/// Tracepoint: lock:lock_release
/// Fields: lock_addr (offset 0, u64), wait_time (offset 8, u32), hold_time (offset 12, u32)
#[tracepoint(target = "lock", name = "lock_release")]
pub fn handle_lock_release(ctx: TracePointContext) -> i64 {
    let lock_addr = unsafe { *ctx.args().add(0) as *const u64 };
    let wait_time = unsafe { *ctx.args().add(8) as *const u32 };
    let hold_time = unsafe { *ctx.args().add(12) as *const u32 };

    let hdr = make_header(EVENT_TYPE_LOCK, core::mem::size_of::<LockEvent>() as u32);
    let evt = LockEvent {
        hdr,
        op: 1, // release
        lock_id: (lock_addr & 0xFFFFFFFF) as u32,
        wait_ns: wait_time,
        held_ns: hold_time,
    };
    unsafe { submit_event(&evt) };
    0
}
```

- [ ] **Step 2: 更新 lib.rs 移除内联 lock 模块**

Modify: `crates/uof-ebpf-programs/src/lib.rs`

将 lib.rs 中内联的 `pub mod lock { ... }` 模块替换为:
```rust
pub mod lock;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add crates/uof-ebpf-programs/src/lock.rs crates/uof-ebpf-programs/src/lib.rs
git commit -m "feat(ebpf-probes): implement lock tracepoint handlers"
```

---

## Task 5: 完善 net.rs 网络探针

**Files:**
- Modify: `crates/uof-ebpf-programs/src/lib.rs` (移除内联 net 模块)
- Modify: `crates/uof-ebpf-programs/src/net.rs` (创建独立文件)

**实现说明:**
- `sock:inet_sock_set_state` tracepoint 字段: family, protocol, saddr, daddr, sport, dport, old_state, new_state
- `net:netif_receive_skb` tracepoint 字段: len, protocol

- [ ] **Step 1: 创建 net.rs 文件**

Create: `crates/uof-ebpf-programs/src/net.rs`

```rust
//! Network socket tracepoint handlers.
//!
//! Tracepoints: net:netif_receive_skb, net:netif_tx

use aya_ebpf::{macros::tracepoint, programs::TracePointContext};

use crate::event::{NetEvent, EVENT_TYPE_NET};
use crate::common::{make_header, submit_event};

/// Network packet receive tracepoint.
/// Tracepoint: net:netif_receive_skb
/// Fields: len (offset 0, u32), protocol (offset 4, u16)
#[tracepoint(target = "net", name = "netif_receive_skb")]
pub fn handle_netif_receive_skb(ctx: TracePointContext) -> i64 {
    let len = unsafe { *ctx.args().add(0) as *const u32 };
    let protocol = unsafe { *ctx.args().add(4) as *const u16 };

    let hdr = make_header(EVENT_TYPE_NET, core::mem::size_of::<NetEvent>() as u32);
    let evt = NetEvent {
        hdr,
        direction: 0, // receive
        protocol,
        saddr: 0,
        daddr: 0,
        sport: 0,
        dport: 0,
        payload_len: len,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}

/// Network packet send tracepoint.
/// Tracepoint: net:netif_tx
/// Fields: len (offset 0, u32), protocol (offset 4, u16)
#[tracepoint(target = "net", name = "netif_tx")]
pub fn handle_netif_tx(ctx: TracePointContext) -> i64 {
    let len = unsafe { *ctx.args().add(0) as *const u32 };
    let protocol = unsafe { *ctx.args().add(4) as *const u16 };

    let hdr = make_header(EVENT_TYPE_NET, core::mem::size_of::<NetEvent>() as u32);
    let evt = NetEvent {
        hdr,
        direction: 1, // send
        protocol,
        saddr: 0,
        daddr: 0,
        sport: 0,
        dport: 0,
        payload_len: len,
        latency_ns: 0,
    };
    unsafe { submit_event(&evt) };
    0
}
```

- [ ] **Step 2: 更新 lib.rs 移除内联 net 模块**

Modify: `crates/uof-ebpf-programs/src/lib.rs`

将 lib.rs 中内联的 `pub mod net { ... }` 模块替换为:
```rust
pub mod net;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add crates/uof-ebpf-programs/src/net.rs crates/uof-ebpf-programs/src/lib.rs
git commit -m "feat(ebpf-probes): implement network tracepoint handlers"
```

---

## Task 6: 完善 uprobe.rs 用户空间探针

**Files:**
- Modify: `crates/uof-ebpf-programs/src/lib.rs` (移除内联 uprobe 模块)
- Modify: `crates/uof-ebpf-programs/src/uprobe.rs` (创建独立文件)

**实现说明:**
- uprobe 用户态探针需要动态加载，用户指定函数地址
- 框架提供基础结构，捕获函数入口参数和返回值

- [ ] **Step 1: 创建 uprobe.rs 文件**

Create: `crates/uof-ebpf-programs/src/uprobe.rs`

```rust
//! User-space probe handlers.
//!
//! These are dynamic probes that attach to user-space function entries
//! and returns. The actual function addresses are specified at load time.

use aya_ebpf::{macros::{uprobe, uretprobe}, programs::{ProbeContext, RetProbeContext}};

use crate::event::{UprobeEvent, EVENT_TYPE_UPROBE};
use crate::common::{make_header, submit_event};

/// User-space function entry probe.
///
/// This is a template that should be loaded with a specific function address.
/// The probe captures the function's arguments (up to 6).
#[uprobe]
pub fn handle_uprobe(ctx: ProbeContext) -> u32 {
    let args = [
        ctx.arg::<u64>(0).unwrap_or(0),
        ctx.arg::<u64>(1).unwrap_or(0),
        ctx.arg::<u64>(2).unwrap_or(0),
        ctx.arg::<u64>(3).unwrap_or(0),
        ctx.arg::<u64>(4).unwrap_or(0),
        ctx.arg::<u64>(5).unwrap_or(0),
    ];

    let hdr = make_header(EVENT_TYPE_UPROBE, core::mem::size_of::<UprobeEvent>() as u32);
    let evt = UprobeEvent {
        hdr,
        func_addr: 0, // Set by user-space at load time
        ret_addr: 0,
        args,
    };
    unsafe { submit_event(&evt) };
    0
}

/// User-space function return probe.
///
/// This is a template that should be loaded with a specific function address.
/// The probe captures the return value.
#[uretprobe]
pub fn handle_uretprobe(ctx: RetProbeContext) -> u32 {
    let ret = ctx.ret::<u64>().unwrap_or(0);

    let hdr = make_header(EVENT_TYPE_UPROBE, core::mem::size_of::<UprobeEvent>() as u32);
    let evt = UprobeEvent {
        hdr,
        func_addr: 0, // Set by user-space at load time
        ret_addr: ret,
        args: [0; 6],
    };
    unsafe { submit_event(&evt) };
    0
}
```

- [ ] **Step 2: 更新 lib.rs 移除内联 uprobe 模块**

Modify: `crates/uof-ebpf-programs/src/lib.rs`

将 lib.rs 中内联的 `pub mod uprobe { ... }` 模块替换为:
```rust
pub mod uprobe;
```

- [ ] **Step 3: 验证编译**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 编译成功

- [ ] **Step 4: 提交**

```bash
git add crates/uof-ebpf-programs/src/uprobe.rs crates/uof-ebpf-programs/src/lib.rs
git commit -m "feat(ebpf-probes): implement uprobe handlers"
```

---

## Task 7: 最终验证和清理

- [ ] **Step 1: 完整编译验证**

Run: `cargo build --target bpfel-unknown-none -p uof-ebpf-programs 2>&1`
Expected: 编译成功，无警告

- [ ] **Step 2: 检查所有模块是否正确导出**

确认 lib.rs 导出所有模块: event, maps, common, io, sched, syscall, lock, net, uprobe

- [ ] **Step 3: 运行 clippy (如果可用)**

Run: `cargo clippy --target bpfel-unknown-none -p uof-ebpf-programs 2>&1 | head -50`
Expected: 无严重警告

- [ ] **Step 4: 提交所有更改**

```bash
git add -A
git commit -m "feat(ebpf-probes): complete all eBPF probe implementations

- Add common.rs with init_event_header and emit_to_ringbuf helpers
- Complete sched.rs: sched_switch, sched_wakeup, sched_process_fork, sched_process_exit
- Complete io.rs: block_rq_insert, block_rq_complete, block_rq_issue
- Complete lock.rs: lock_acquire, lock_release (extracted from lib.rs)
- Complete net.rs: netif_receive_skb, netif_tx (extracted from lib.rs)
- Complete uprobe.rs: handle_uprobe, handle_uretprobe (extracted from lib.rs)
"
```

---

## 自检清单

- [ ] spec coverage: 每个设计中的探针都有对应 Task 实现
- [ ] placeholder scan: 无 "TBD", "TODO", "implement later" 等占位符
- [ ] type consistency: 所有事件类型与 event.rs 中定义一致
- [ ] 所有 Task 步骤都包含实际代码，无省略

---

**Plan complete.** 文件保存在 `docs/superpowers/plans/2026-06-02-ebpf-probes-implementation-plan.md`

**两个执行选项:**

**1. Subagent-Driven (recommended)** - 每个 Task 由独立 subagent 实现，期间有检查点审核

**2. Inline Execution** - 在当前 session 中使用 executing-plans 批量执行

选择哪个方式？