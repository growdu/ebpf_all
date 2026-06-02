# eBPF 探针完整实现设计

**日期:** 2026-06-02
**项目:** UOF (Universal Observability Framework)
**目标:** 完成 eBPF 探针的完整实现

## 1. 概述

当前 `uof-ebpf-programs` crate 中的探针实现不完整：
- `sched.rs`: 字段硬编码为 0
- `io.rs`: 字段硬编码
- `lock.rs`: 仅返回 0
- `net.rs`: 仅返回 0
- `uprobe.rs`: 仅返回 0

目标：完整实现所有探针，从 tracepoint 正确读取 kernel 数据并写入 ringbuf。

## 2. 架构

```
uof-ebpf-programs/src/
├── event.rs      # 事件类型定义 ✅ 已完成
├── maps.rs       # RingBuf map ✅ 已完成
├── common.rs     # [新增] 通用辅助函数
├── sched.rs      # [完善] 调度器探针
├── io.rs         # [完善] 块设备 I/O 探针
├── syscall.rs    # [已完整] ✅
├── lock.rs       # [完善] 锁探针
├── net.rs        # [完善] 网络探针
└── uprobe.rs     # [完善] 用户空间探针
```

## 3. 通用框架 (common.rs)

### 3.1 辅助函数

```rust
/// 初始化 tracepoint 事件的通用头部
#[inline(always)]
unsafe fn init_event_header(ctx: &TracePointContext, event_type: u16) -> EventHeader {
    let mut hdr = EventHeader::default();
    hdr.ts_ns = bpf_ktime_get_ns();
    hdr.event_type = event_type;
    hdr.version = 1;
    hdr.cpu_id = bpf_get_smp_processor_id();

    let pid_tgid = bpf_get_current_pid_tgid();
    hdr.pid = (pid_tgid >> 32) as u32;
    hdr.tid = pid_tgid as u32;

    let uid_gid = bpf_get_current_uid_gid();
    hdr.uid = (uid_gid >> 32) as u32;
    hdr.gid = uid_gid as u32;

    hdr
}

/// 发射事件到 ringbuf
#[inline(always)]
unsafe fn emit_to_ringbuf<T>(event: &T) -> i64
where
    T: Sized,
{
    ringbuf().output(event, 0)
}
```

### 3.2 bpf_probe_read 包装

```rust
/// 安全读取 kernel 数据的辅助宏
macro_rules! probe_read {
    ($ptr:expr) => {
        bpf_probe_read($ptr as *const _, core::mem::size_of_val($ptr), $ptr as *const _)
    };
}
```

## 4. 各模块实现

### 4.1 sched.rs - 调度器探针

| Tracepoint | 字段 |
|------------|------|
| `sched:sched_switch` | prev_pid, prev_state, next_pid, next_prio, next_cpu |
| `sched:sched_wakeup` | pid, prio, success, target_cpu |
| `sched:sched_process_fork` | pid, child_pid, clone_flags |
| `sched:sched_process_exit` | pid, exit_code, exit_signal |

```rust
#[tracepoint]
pub fn handle_sched_switch(ctx: TracePointContext) -> u32 {
    unsafe {
        // 从 ctx 读取 sched_switch 字段
        let prev_pid = bpf_probe_read(...);
        let prev_state = bpf_probe_read(...);
        let next_pid = bpf_probe_read(...);

        let mut event = SchedEvent {
            hdr: init_event_header(&ctx, EVENT_TYPE_SCHED),
            kind: 0,
            prev_pid,
            next_pid,
            latency_ns: 0,
        };

        emit_to_ringbuf(&event)
    }
    0
}
```

### 4.2 io.rs - 块设备 I/O 探针

| Tracepoint | 字段 |
|------------|------|
| `block:block_rq_insert` | sector, num_sectors, dev, operation |
| `block:block_rq_complete` | sector, num_sectors, errors, latency_ns |
| `block:block_rq_issue` | sector, num_sectors, cmd_type |

### 4.3 lock.rs - 锁探针

| Tracepoint | 字段 |
|------------|------|
| `lock:lock_acquire` | lock_addr, ret, contended |
| `lock:lock_release` | lock_addr, wait_time, hold_time |

### 4.4 net.rs - 网络探针

| Tracepoint | 字段 |
|------------|------|
| `sock:inet_sock_set_state` | family, protocol, saddr, daddr, sport, dport, old_state, new_state |
| `net:netif_receive_skb` | len, protocol |

### 4.5 uprobe.rs - 用户空间探针

用户态探针需要用户指定函数地址，框架提供基础结构：
- `handle_uprobe`: 捕获函数入口参数
- `handle_uretprobe`: 捕获函数返回值

## 5. 数据流

```
Kernel Tracepoint
       │
       ▼
  eBPF Program (tracepoint handler)
       │
       ▼
  bpf_probe_read() 读取 tracepoint 字段
       │
       ▼
  填充事件结构体
       │
       ▼
  ringbuf.output() 写入 ring buffer
       │
       ▼
  User Space (probe-runtime) 读取
       │
       ▼
  转发到 OTLP Collector
```

## 6. 构建和测试

### 6.1 构建命令
```bash
cargo build-bpf -p uof-ebpf-programs
```

### 6.2 验证
- 编译成功无错误
- 所有 tracepoint 字段正确映射
- ringbuf 写入不丢失事件

## 7. 依赖

- `aya-ebpf` 0.1
- `aya-ebpf-bindings` 0.1
- Nightly Rust (见 rust-toolchain.toml)

## 8. 其他优化任务

完成 eBPF 探针后，继续：
1. **清理 dead code** - 消除 control-api 中未使用的函数警告
2. **添加测试覆盖** - 为核心模块添加 unit tests
3. **完善部署配置** - 完成 systemd 服务模板