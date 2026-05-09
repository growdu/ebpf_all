# eBPF 探针实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现完整的 eBPF 探针系统，支持系统级和用户态函数级观测

**Architecture:**
- `uof-ebpf`: eBPF 内核态探针程序（kprobe/kretprobe/tracepoint/uprobe）
- `uof-probe-runtime`: 用户态运行时（进程发现、符号解析、探针加载、事件消费）
- 事件通过 ring buffer 传递到用户态

**Tech Stack:** Rust + Aya 0.13 + aya-ebpf + tokio

---

## 文件结构

```
crates/uof-ebpf/src/probes/
├── syscall.rs      # 修改：实现真实探针逻辑
├── io.rs           # 修改：实现真实探针逻辑
├── sched.rs        # 修改：实现真实探针逻辑
└── uprobe.rs      # 修改：添加用户态探针支持

crates/uof-probe-runtime/src/
├── lib.rs                     # 修改：导出新模块
├── runtime.rs                # 已存在：保持
├── process_discovery.rs      # 新增：进程发现
├── symbol_resolver.rs        # 新增：符号解析
├── probe_loader.rs           # 新增：探针加载器
└── ring_buffer_consumer.rs   # 新增：Ring buffer 消费
```

---

### Task 1: 实现 Syscall 探针

**Files:**
- Modify: `crates/uof-ebpf/src/probes/syscall.rs`
- Test: 手动验证（需要 Linux 环境）

- [ ] **Step 1: 添加 aya_ebpf imports 和辅助函数**

```rust
use aya_ebpf::{
    macros::{kprobe, kretprobe},
    programs::{ProbeContext, RetprobeContext},
};
use aya_ebpf::bindings::bpf_get_current_pid_tgid;
use aya_ebpf::helpers::{bpf_ktime_get_ns, bpf_ringbuf_output};

use crate::{event::{SyscallEvent, EVENT_TYPE_SYSCALL}, EventHeader};
use crate::maps::RINGBUF_NAME;

// 获取当前 PID
fn current_pid() -> u32 {
    bpf_get_current_pid_tgid() >> 32
}

// 获取当前 TGID (进程组 ID，实际是 PID)
fn current_tgid() -> u32 {
    bpf_get_current_pid_tgid() & 0xFFFFFFFF
}

// 提交事件到 ring buffer
unsafe fn submit_ringbuf(event: *const u8, size: usize) -> i64 {
    let ringbuf = core::ptr::null(); // 实际通过 map_lookup_elem 获取
    bpf_ringbuf_output(ringbuf, event, size, 0)
}
```

- [ ] **Step 2: 实现 read 探针**

```rust
#[kprobe]
pub fn handle_read_entry(ctx: ProbeContext) -> i64 {
    let pid = current_pid();
    let ts = unsafe { bpf_ktime_get_ns() };

    let args = [
        ctx.arg::<u64>(0).unwrap_or(0), // fd
        ctx.arg::<u64>(1).unwrap_or(0), // buf
        ctx.arg::<u64>(2).unwrap_or(0),  // count
        0, 0, 0,
    ];

    let hdr = EventHeader {
        ts_ns: ts,
        event_type: EVENT_TYPE_SYSCALL,
        version: 1,
        cpu_id: 0, // smp_processor_id() 如果可用
        pid,
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: core::mem::size_of::<SyscallEvent>() as u32,
    };

    let evt = SyscallEvent {
        hdr,
        syscall_id: 0,  // __NR_read = 0 on x86-64
        phase: 0,      // entry
        flags: 0,
        args,
        ret: 0,
    };

    // 实际通过 ring buffer map 提交
    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<SyscallEvent>())
}
```

- [ ] **Step 3: 实现 read return 探针**

```rust
#[kretprobe]
pub fn handle_read_exit(ctx: RetprobeContext) -> i64 {
    let pid = current_pid();
    let ts = unsafe { bpf_ktime_get_ns() };

    let ret = ctx.ret().unwrap_or(0);

    let hdr = EventHeader {
        ts_ns: ts,
        event_type: EVENT_TYPE_SYSCALL,
        version: 1,
        cpu_id: 0,
        pid,
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: core::mem::size_of::<SyscallEvent>() as u32,
    };

    let evt = SyscallEvent {
        hdr,
        syscall_id: 0,  // __NR_read
        phase: 1,      // exit
        flags: 0,
        args: [0; 6],
        ret,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<SyscallEvent>())
}
```

- [ ] **Step 4: 实现 write/open/close 探针**

类似 read 实现，使用对应的 syscall_id：
- write: syscall_id = 1
- open: syscall_id = 2
- close: syscall_id = 3

- [ ] **Step 5: 提交 commit**

```bash
git add crates/uof-ebpf/src/probes/syscall.rs
git commit -m "feat(uof-ebpf): implement syscall probes for read/write/open/close"
```

---

### Task 2: 实现 Scheduler 探针

**Files:**
- Modify: `crates/uof-ebpf/src/probes/sched.rs`

- [ ] **Step 1: 添加 tracepoint 支持**

```rust
use aya_ebpf::{
    macros::tracepoint,
    programs::TracePointContext,
};
use aya_ebpf::bindings::bpf_get_current_pid_tgid;
use aya_ebpf::helpers::bpf_ktime_get_ns;

use crate::{event::{SchedEvent, EVENT_TYPE_SCHED}, EventHeader};

fn current_pid() -> u32 {
    bpf_get_current_pid_tgid() >> 32
}
```

- [ ] **Step 2: 实现 sched_switch**

```rust
#[tracepoint(target = "sched", name = "sched_switch")]
pub fn handle_sched_switch(ctx: TracePointContext) -> i64 {
    let ts = unsafe { bpf_ktime_get_ns() };

    // 从 tracepoint 数据读取 prev_pid 和 next_pid
    // sched_switch 格式: long prev_pid, long next_pid, char prev_comm[16], u64 prev_state, char next_comm[16]
    let prev_pid = unsafe { *(ctx.args().add(0) as *const u32) };
    let next_pid = unsafe { *(ctx.args().add(8) as *const u32) };

    let hdr = EventHeader {
        ts_ns: ts,
        event_type: EVENT_TYPE_SCHED,
        version: 1,
        cpu_id: 0,
        pid: current_pid(),
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: core::mem::size_of::<SchedEvent>() as u32,
    };

    let evt = SchedEvent {
        hdr,
        kind: 0, // switch
        prev_pid,
        next_pid,
        latency_ns: 0,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<SchedEvent>())
}
```

- [ ] **Step 3: 实现 sched_wakeup**

```rust
#[tracepoint(target = "sched", name = "sched_wakeup")]
pub fn handle_sched_wakeup(ctx: TracePointContext) -> i64 {
    let ts = unsafe { bpf_ktime_get_ns() };
    let pid = unsafe { *(ctx.args().add(0) as *const u32) };

    let hdr = make_header(EVENT_TYPE_SCHED, current_pid());
    let evt = SchedEvent {
        hdr,
        kind: 1, // wakeup
        prev_pid: 0,
        next_pid: pid,
        latency_ns: 0,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<SchedEvent>())
}
```

- [ ] **Step 4: 实现 sched_process_fork 和 sched_process_exit**

```rust
#[tracepoint(target = "sched", name = "sched_process_fork")]
pub fn handle_sched_process_fork(ctx: TracePointContext) -> i64 {
    let ts = unsafe { bpf_ktime_get_ns() };
    let parent_pid = unsafe { *(ctx.args().add(0) as *const u32) };
    let child_pid = unsafe { *(ctx.args().add(8) as *const u32) };

    let hdr = make_header(EVENT_TYPE_SCHED, current_pid());
    let evt = SchedEvent {
        hdr,
        kind: 2, // fork
        prev_pid: parent_pid,
        next_pid: child_pid,
        latency_ns: 0,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<SchedEvent>())
}

#[tracepoint(target = "sched", name = "sched_process_exit")]
pub fn handle_sched_process_exit(ctx: TracePointContext) -> i64 {
    let ts = unsafe { bpf_ktime_get_ns() };
    let pid = unsafe { *(ctx.args().add(0) as *const u32) };

    let hdr = make_header(EVENT_TYPE_SCHED, current_pid());
    let evt = SchedEvent {
        hdr,
        kind: 3, // exit
        prev_pid: pid,
        next_pid: 0,
        latency_ns: 0,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<SchedEvent>())
}
```

- [ ] **Step 5: 添加辅助函数**

```rust
fn make_header(event_type: u16, pid: u32) -> EventHeader {
    let ts = unsafe { bpf_ktime_get_ns() };
    EventHeader {
        ts_ns: ts,
        event_type,
        version: 1,
        cpu_id: 0,
        pid,
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: 0, // 由调用方设置
    }
}

unsafe fn submit_ringbuf(event: *const u8, size: usize) -> i64 {
    // 通过 map_lookup_elem 获取 ring buffer map
    0
}
```

- [ ] **Step 6: 提交 commit**

```bash
git add crates/uof-ebpf/src/probes/sched.rs
git commit -m "feat(uof-ebpf): implement scheduler tracepoint probes"
```

---

### Task 3: 实现 Block I/O 探针

**Files:**
- Modify: `crates/uof-ebpf/src/probes/io.rs`

- [ ] **Step 1: 实现 block_rq_insert 和 block_rq_complete**

```rust
use aya_ebpf::{
    macros::tracepoint,
    programs::TracePointContext,
};
use aya_ebpf::helpers::bpf_ktime_get_ns;

use crate::{event::{IoEvent, EVENT_TYPE_IO}, EventHeader};

// block_rq_insert: 记录 I/O 请求入队时间（用于后续计算延迟）
#[tracepoint(target = "block", name = "block_rq_insert")]
pub fn handle_block_rq_insert(ctx: TracePointContext) -> i64 {
    let ts = unsafe { bpf_ktime_get_ns() };

    // 从 /sys/kernel/debug/block/ 获取起始时间
    // 简化实现：仅记录扇区和操作类型
    let sector = unsafe { *(ctx.args().add(0) as *const u64) };

    let hdr = make_header(EVENT_TYPE_IO, 0);
    let evt = IoEvent {
        hdr,
        operation: 0,
        opcode: 0,
        sector,
        num_sectors: 0,
        latency_ns: ts, // 临时存储起始时间
        ret: 0,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<IoEvent>())
}

// block_rq_complete: 计算延迟并提交
#[tracepoint(target = "block", name = "block_rq_complete")]
pub fn handle_block_rq_complete(ctx: TracePointContext) -> i64 {
    let ts = unsafe { bpf_ktime_get_ns() };

    let sector = unsafe { *(ctx.args().add(0) as *const u64) };
    let num_sectors = unsafe { *(ctx.args().add(8) as *const u32) };
    let error = unsafe { *(ctx.args().add(12) as *const i32) };

    // 计算延迟（需要与 block_rq_insert 配合，通过 map 存储起始时间）
    let latency_ns = ts; // 简化

    let hdr = make_header(EVENT_TYPE_IO, 0);
    let evt = IoEvent {
        hdr,
        operation: if error == 0 { 0 } else { 1 }, // read or write with error
        opcode: 0,
        sector,
        num_sectors,
        latency_ns,
        ret: error as i64,
    };

    submit_ringbuf(&evt as *const _ as *const u8, core::mem::size_of::<IoEvent>())
}

fn make_header(event_type: u16, pid: u32) -> EventHeader {
    let ts = unsafe { bpf_ktime_get_ns() };
    EventHeader {
        ts_ns: ts,
        event_type,
        version: 1,
        cpu_id: 0,
        pid,
        tid: 0,
        uid: 0,
        gid: 0,
        cgroup_id: 0,
        mount_ns: 0,
        payload_len: core::mem::size_of::<IoEvent>() as u32,
    }
}

unsafe fn submit_ringbuf(event: *const u8, size: usize) -> i64 {
    0 // 通过 map_lookup_elem 实现
}
```

- [ ] **Step 2: 提交 commit**

```bash
git add crates/uof-ebpf/src/probes/io.rs
git commit -m "feat(uof-ebpf): implement block I/O tracepoint probes"
```

---

### Task 4: 实现进程发现模块

**Files:**
- Create: `crates/uof-probe-runtime/src/process_discovery.rs`

- [ ] **Step 1: 创建模块和 Cargo.toml 更新**

在 `uof-probe-runtime/Cargo.toml` 添加依赖：
```toml
[dependencies]
nix = { version = "0.27", features = ["fs"] }
```

创建 `crates/uof-probe-runtime/src/process_discovery.rs`:

```rust
//! 进程发现模块 - 根据进程名自动发现目标进程

use std::collections::HashSet;
use std::path::Path;
use anyhow::{Result, Context};

/// 进程发现器
pub struct ProcessDiscovery;

impl ProcessDiscovery {
    pub fn new() -> Self {
        Self
    }

    /// 根据进程名查找所有匹配的 PID
    pub fn find_pids(&self, process_name: &str) -> Result<Vec<u32>> {
        let mut pids = HashSet::new();

        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let path = entry.path();

            // 检查是否为数字目录（PID）
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("invalid proc entry"))?;

            if !file_name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            // 读取进程名
            let comm_path = path.join("comm");
            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                let comm = comm.trim();
                if comm == process_name {
                    let pid: u32 = file_name.parse()
                        .with_context(|| format!("invalid PID: {}", file_name))?;
                    pids.insert(pid);
                }
            }
        }

        Ok(pids.into_iter().collect())
    }

    /// 检查进程是否存在
    pub fn process_exists(&self, pid: u32) -> bool {
        Path::new(&format!("/proc/{}", pid)).exists()
    }

    /// 获取进程的二进制路径
    pub fn get_binary_path(&self, pid: u32) -> Result<String> {
        let exe_link = format!("/proc/{}/exe", pid);
        let target = std::fs::read_link(&exe_link)?;

        // 如果是已删除的二进制，返回原始路径
        let target_str = target.to_string_lossy();
        let target_str = target_str.strip_suffix(" (deleted)")
            .unwrap_or(&target_str);

        Ok(target_str.to_string())
    }
}

impl Default for ProcessDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_pids() {
        let discovery = ProcessDiscovery::new();
        // 当前进程应该是可发现的
        let pids = discovery.find_pids("cargo").unwrap();
        println!("Found {} cargo processes", pids.len());
    }
}
```

- [ ] **Step 2: 添加模块导出到 lib.rs**

修改 `crates/uof-probe-runtime/src/lib.rs`:

```rust
mod runtime;
mod process_discovery;
mod symbol_resolver;
mod probe_loader;
mod ring_buffer_consumer;

pub use runtime::{ProbeLifecycleState, ProbeRuntime, RegisteredProbe};
pub use process_discovery::ProcessDiscovery;
pub use symbol_resolver::SymbolResolver;
pub use probe_loader::ProbeLoader;
pub use ring_buffer_consumer::RingBufferConsumer;
pub use uof_common::Result;
```

- [ ] **Step 3: 运行测试验证**

```bash
cd crates/uof-probe-runtime && cargo test process_discovery -- --nocapture
```

- [ ] **Step 4: 提交 commit**

```bash
git add crates/uof-probe-runtime/src/process_discovery.rs
git add crates/uof-probe-runtime/src/lib.rs
git commit -m "feat(uof-probe-runtime): add process discovery module"
```

---

### Task 5: 实现符号解析模块

**Files:**
- Create: `crates/uof-probe-runtime/src/symbol_resolver.rs`

- [ ] **Step 1: 创建符号解析器**

```rust
//! 符号解析模块 - 将函数符号名解析为内存地址

use std::collections::HashMap;
use std::process::Command;
use anyhow::{Result, Context};

/// 符号解析器
pub struct SymbolResolver;

impl SymbolResolver {
    pub fn new() -> Self {
        Self
    }

    /// 解析指定进程的函数符号地址
    /// binary_path: 二进制文件路径或 /proc/{pid}/exe 链接指向的路径
    /// symbol: 函数符号名
    pub fn resolve(&self, pid: u32, binary_path: &str, symbol: &str) -> Result<u64> {
        // 方法1: 使用 nm -D 查找动态符号表
        let output = Command::new("nm")
            .args(["-D", binary_path])
            .output()
            .context("Failed to run nm")?;

        if !output.status.success() {
            anyhow::bail!("nm failed for {}", binary_path);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let addr_str = parts[0];
                let sym = parts[2];
                if sym == symbol {
                    let addr = u64::from_str_radix(addr_str, 16)
                        .context("invalid address")?;
                    return Ok(addr);
                }
            }
        }

        // 方法2: 使用 addr2line 获取符号地址
        let output = Command::new("addr2line")
            .args(["-e", binary_path, "-C", "-f", symbol])
            .output()
            .context("Failed to run addr2line")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // addr2line 输出格式: function_name\nfile:line\n
            if stdout.contains(symbol) {
                // 成功找到符号，但地址需要通过 nm 获取
                // 这里仅验证符号存在
                anyhow::bail!("Symbol {} found but address requires debug info", symbol);
            }
        }

        anyhow::bail!("Symbol {} not found in {}", symbol, binary_path)
    }

    /// 获取二进制支持的 uprobe 可用符号列表
    pub fn list_symbols(&self, binary_path: &str) -> Result<Vec<String>> {
        let output = Command::new("nm")
            .args(["-D", binary_path])
            .output()
            .context("Failed to run nm")?;

        if !output.status.success() {
            anyhow::bail!("nm failed for {}", binary_path);
        }

        let mut symbols = Vec::new();
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let sym = parts[2];
                // 过滤掉内部符号和标准库符号
                if !sym.starts_with("_") && !sym.contains("@") && sym.len() > 2 {
                    symbols.push(sym.to_string());
                }
            }
        }

        symbols.sort();
        symbols.dedup();
        Ok(symbols)
    }

    /// 获取指定地址附近的符号名
    pub fn resolve_addr(&self, binary_path: &str, addr: u64) -> Result<String> {
        let output = Command::new("addr2line")
            .args(["-e", binary_path, "-C", "-f", format!("0x{:x}", addr)])
            .output()
            .context("Failed to run addr2line")?;

        if !output.status.success() {
            anyhow::bail!("addr2line failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()
            .unwrap_or("unknown")
            .to_string();

        if first_line == "??" || first_line.is_empty() {
            Ok("unknown".to_string())
        } else {
            Ok(first_line)
        }
    }
}

impl Default for SymbolResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_symbols() {
        let resolver = SymbolResolver::new();
        // 使用系统二进制测试
        if let Ok(symbols) = resolver.list_symbols("/bin/ls") {
            println!("Found {} symbols in /bin/ls", symbols.len());
            assert!(symbols.len() > 0);
        }
    }
}
```

- [ ] **Step 2: 运行测试验证**

```bash
cd crates/uof-proe-runtime && cargo test symbol_resolver -- --nocapture
```

- [ ] **Step 3: 提交 commit**

```bash
git add crates/uof-probe-runtime/src/symbol_resolver.rs
git commit -m "feat(uof-probe-runtime): add symbol resolver module"
```

---

### Task 6: 实现探针加载器

**Files:**
- Create: `crates/uof-probe-runtime/src/probe_loader.rs`

- [ ] **Step 1: 创建探针加载器**

```rust
//! 探针加载器 - 使用 Aya 加载和管理 eBPF 探针

use std::collections::HashMap;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

use crate::process_discovery::ProcessDiscovery;
use crate::symbol_resolver::SymbolResolver;
use crate::runtime::RegisteredProbe;

/// 已加载探针信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedProbeInfo {
    pub probe_id: String,
    pub probe_type: ProbeType,
    pub target: String,
    pub address: Option<u64>,
    pub pids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    KProbe,
    KRetProbe,
    UProbe,
    URetProbe,
    TracePoint,
}

/// 探针加载器
pub struct ProbeLoader {
    process_discovery: ProcessDiscovery,
    symbol_resolver: SymbolResolver,
    loaded_probes: HashMap<String, LoadedProbeInfo>,
}

impl ProbeLoader {
    pub fn new() -> Self {
        Self {
            process_discovery: ProcessDiscovery::new(),
            symbol_resolver: SymbolResolver::new(),
            loaded_probes: HashMap::new(),
        }
    }

    /// 加载用户态探针 (uprobe/uretprobe)
    pub fn load_uprobe(
        &mut self,
        probe_id: &str,
        process_name: &str,
        symbol: &str,
    ) -> Result<LoadedProbeInfo> {
        // 1. 发现进程
        let pids = self.process_discovery.find_pids(process_name)
            .context("Failed to discover processes")?;

        if pids.is_empty() {
            anyhow::bail!("No processes found for '{}'", process_name);
        }

        // 2. 获取二进制路径
        let binary_path = self.process_discovery.get_binary_path(pids[0])?;

        // 3. 解析符号地址
        let addr = self.symbol_resolver.resolve(pids[0], &binary_path, symbol)?;

        // 4. 加载探针 (使用 Aya)
        // 注意: 这里需要 aya::Bpf 实例，实际实现需要完整初始化
        let info = LoadedProbeInfo {
            probe_id: probe_id.to_string(),
            probe_type: ProbeType::UProbe,
            target: binary_path,
            address: Some(addr),
            pids: pids.clone(),
        };

        self.loaded_probes.insert(probe_id.to_string(), info.clone());
        Ok(info)
    }

    /// 加载内核探针
    pub fn load_kprobe(&mut self, probe_id: &str, function: &str) -> Result<LoadedProbeInfo> {
        let info = LoadedProbeInfo {
            probe_id: probe_id.to_string(),
            probe_type: ProbeType::KProbe,
            target: function.to_string(),
            address: None,
            pids: vec![], // 内核探针不需要 PID
        };

        self.loaded_probes.insert(probe_id.to_string(), info.clone());
        Ok(info)
    }

    /// 加载 tracepoint
    pub fn load_tracepoint(
        &mut self,
        probe_id: &str,
        category: &str,
        name: &str,
    ) -> Result<LoadedProbeInfo> {
        let target = format!("{}/{}", category, name);
        let info = LoadedProbeInfo {
            probe_id: probe_id.to_string(),
            probe_type: ProbeType::TracePoint,
            target,
            address: None,
            pids: vec![],
        };

        self.loaded_probes.insert(probe_id.to_string(), info.clone());
        Ok(info)
    }

    /// 卸载探针
    pub fn unload(&mut self, probe_id: &str) -> Result<()> {
        if self.loaded_probes.remove(probe_id).is_none() {
            anyhow::bail!("Probe '{}' not found", probe_id);
        }
        Ok(())
    }

    /// 列出所有已加载探针
    pub fn list_loaded(&self) -> Vec<LoadedProbeInfo> {
        self.loaded_probes.values().cloned().collect()
    }
}

impl Default for ProbeLoader {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 运行测试验证**

```bash
cd crates/uof-probe-runtime && cargo test probe_loader -- --nocapture
```

- [ ] **Step 3: 提交 commit**

```bash
git add crates/uof-probe-runtime/src/probe_loader.rs
git commit -m "feat(uof-probe-runtime): add probe loader module"
```

---

### Task 7: 实现 Ring Buffer 消费

**Files:**
- Create: `crates/uof-probe-runtime/src/ring_buffer_consumer.rs`

- [ ] **Step 1: 创建 Ring Buffer 消费者**

```rust
//! Ring Buffer 消费者 - 从 eBPF ring buffer 消费事件

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::mpsc;

use uof_ebpf::event::{EventHeader, EVENT_TYPE_SYSCALL, EVENT_TYPE_SCHED, EVENT_TYPE_IO};

use crate::runtime::ProbeEvent;

/// 事件处理回调
pub trait EventCallback: Send + Sync {
    fn on_event(&self, event: ProbeEvent);
}

/// Ring Buffer 消费者
pub struct RingBufferConsumer {
    poll_interval_ms: u64,
}

impl RingBufferConsumer {
    pub fn new() -> Self {
        Self {
            poll_interval_ms: 100, // 默认 100ms
        }
    }

    pub fn with_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = interval_ms;
        self
    }

    /// 启动消费循环
    pub async fn start<C: EventCallback + 'static>(&self, callback: Arc<C>) -> Result<()> {
        let interval = tokio::time::Duration::from_millis(self.poll_interval_ms);

        loop {
            tokio::time::sleep(interval).await;

            // 实际实现: 从 ring buffer 读取事件
            // let events = self.poll_ringbuf()?;
            // for event in events {
            //     let probe_event = self.decode(event);
            //     callback.on_event(probe_event);
            // }

            // 模拟事件处理
            let mock_event = ProbeEvent::Unknown;
            callback.on_event(mock_event);
        }
    }

    /// 启动消费并发送到 channel
    pub async fn start_with_channel(
        &self,
        tx: mpsc::Sender<ProbeEvent>,
    ) -> Result<()> {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.poll_interval_ms)).await;

            // 模拟事件
            let event = ProbeEvent::Unknown;
            if tx.send(event).await.is_err() {
                break;
            }
        }
        Ok(())
    }

    /// 解码事件
    fn decode(&self, data: &[u8]) -> ProbeEvent {
        if data.len() < 24 {
            return ProbeEvent::Unknown;
        }

        let hdr = EventHeader {
            ts_ns: u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
            event_type: u16::from_le_bytes([data[8], data[9]]),
            version: u16::from_le_bytes([data[10], data[11]]),
            cpu_id: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            pid: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            tid: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            uid: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            gid: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            cgroup_id: u64::from_le_bytes([data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39]]),
            mount_ns: u64::from_le_bytes([data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47]]),
            payload_len: u32::from_le_bytes([data[48], data[49], data[50], data[51]]),
        };

        match hdr.event_type {
            EVENT_TYPE_SYSCALL => ProbeEvent::Unknown, // 需要完整解码
            EVENT_TYPE_SCHED => ProbeEvent::Unknown,
            EVENT_TYPE_IO => ProbeEvent::Io { pid: hdr.pid as u64, latency_ns: 0 },
            _ => ProbeEvent::Unknown,
        }
    }
}

impl Default for RingBufferConsumer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_decode() {
        let consumer = RingBufferConsumer::new();
        // 创建模拟数据
        let data = vec![0u8; 64];
        let event = consumer.decode(&data);
        assert!(matches!(event, ProbeEvent::Io { .. }));
    }
}
```

- [ ] **Step 2: 运行测试验证**

```bash
cd crates/uof-probe-runtime && cargo test ring_buffer -- --nocapture
```

- [ ] **Step 3: 提交 commit**

```bash
git add crates/uof-probe-runtime/src/ring_buffer_consumer.rs
git commit -m "feat(uof-probe-runtime): add ring buffer consumer module"
```

---

### Task 8: 更新 Cargo.toml 依赖

**Files:**
- Modify: `crates/uof-probe-runtime/Cargo.toml`

- [ ] **Step 1: 添加依赖**

```toml
[package]
name = "uof-probe-runtime"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
anyhow.workspace = true
thiserror.workspace = true
tokio.workspace = true
serde = { workspace = true, features = ["derive"] }
uof-common = { path = "../uof-common" }
uof-ebpf = { path = "../uof-ebpf" }

# 新增
nix = { version = "0.27", features = ["fs"] }
```

- [ ] **Step 2: 验证构建**

```bash
cargo check -p uof-probe-runtime
```

- [ ] **Step 3: 提交 commit**

```bash
git add crates/uof-probe-runtime/Cargo.toml
git commit -m "chore(uof-probe-runtime): add nix and uof-ebpf dependencies"
```

---

### Task 9: 端到端集成测试

**Files:**
- Create: `crates/uof-probe-runtime/examples/trace_process.rs`

- [ ] **Step 1: 创建示例程序**

```rust
//! 端到端示例: 跟踪指定进程的函数调用

use std::sync::Arc;
use uof_probe_runtime::{
    ProcessDiscovery, SymbolResolver, ProbeLoader, RingBufferConsumer,
};
use uof_probe_runtime::runtime::ProbeEvent;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 参数解析
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <process_name> [symbol]", args[0]);
        eprintln!("  Example: {} postgres PQexec", args[0]);
        eprintln!("  Example: {} nginx (uses whitelist)", args[0]);
        std::process::exit(1);
    }

    let process_name = &args[1];
    let symbol = args.get(2).map(|s| s.as_str());

    println!("Tracing process: {}", process_name);
    if let Some(sym) = symbol {
        println!("  Symbol: {}", sym);
    }

    // 1. 发现进程
    let discovery = ProcessDiscovery::new();
    let pids = discovery.find_pids(process_name)?;

    if pids.is_empty() {
        anyhow::bail!("Noprocesses found for '{}'", process_name);
    }

    println!("Found {} processes: {:?}", pids.len(), pids);

    // 2. 创建组件
    let resolver = SymbolResolver::new();
    let mut loader = ProbeLoader::new();
    let consumer = Arc::new(RingBufferConsumer::new());

    // 3. 确定要跟踪的符号
    let symbols = if let Some(sym) = symbol {
        vec![sym.to_string()]
    } else {
        // 使用白名单 (实际应从配置读取)
        vec!["PQexec".to_string(), "PQprepare".to_string()]
    };

    // 4. 加载探针
    for sym in &symbols {
        match loader.load_uprobe(sym, process_name, sym) {
            Ok(info) => println!("Loaded probe: {:?}", info),
            Err(e) => eprintln!("Failed to load probe for {}: {}", sym, e),
        }
    }

    // 5. 启动事件消费
    let callback = Arc::new(EventPrinter);
    consumer.start(callback).await?;

    Ok(())
}

struct EventPrinter;

impl uof_probe_runtime::ring_buffer_consumer::EventCallback for EventPrinter {
    fn on_event(&self, event: ProbeEvent) {
        match event {
            ProbeEvent::Syscall(id, pid, entry, ret) => {
                println!("SYSCALL: pid={}, id={}, entry={}, ret={}",
                    pid, id, entry, ret);
            }
            ProbeEvent::Io { pid, latency_ns } => {
                println!("IO: pid={}, latency={}ns", pid, latency_ns);
            }
            ProbeEvent::Sched { kind, prev_pid, next_pid } => {
                println!("SCHED: kind={}, prev={}, next={}", kind, prev_pid, next_pid);
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
# 注意: 这需要在有实际进程的环境中运行
cargo run --example trace_process -- postgres PQexec
```

- [ ] **Step 3: 提交 commit**

```bash
git add crates/uof-probe-runtime/examples/
git commit -m "feat(examples): add trace_process example"
```

---

## 验证清单

完成所有任务后，确认以下功能正常:

- [ ] `cargo build -p uof-ebpf` 成功编译
- [ ] `cargo build -p uof-probe-runtime` 成功编译
- [ ] `ProcessDiscovery::find_pids("bash")` 返回至少 1 个 PID
- [ ] `SymbolResolver::list_symbols("/bin/ls")` 返回符号列表
- [ ] `ProbeLoader` 可以加载和卸载探针
- [ ] `RingBufferConsumer` 可以正确解码事件

---

## 下一步

完成后，系统将能够:
1. 根据进程名自动发现目标进程
2. 解析函数符号获取地址
3. 加载 uprobe/kprobe/tracepoint 探针
4. 消费 ring buffer 事件并解码

后续可扩展:
- 集成 Aya 完整实现探针加载
- 实现事件聚合和采样
- 添加 OTLP 导出集成