# eBPF 探针实现设计

## 1. 概述

UOF 通过 eBPF 探针实现对 Linux 系统的深度可观测性，支持：
- **系统级观测**：syscall、I/O、scheduler、network、lock 等内核事件
- **应用级观测**：任意用户态进程的函数调用（通过函数符号名）

### 1.1 设计目标

- 用户只需提供**进程名**和**函数符号名**，系统自动完成探针加载和观测
- 支持全量函数观测（需显式开启）和预定义函数白名单
- 探针按类型组织为独立模块，支持按需加载

---

## 2. 架构概览

```
uof-ebpf/                          # eBPF 探针程序包
├── src/
│   ├── lib.rs                     # 模块导出
│   ├── event.rs                   # 事件类型定义
│   ├── maps.rs                    # Map 定义
│   └── probes/
│       ├── mod.rs                 # 统一探针加载接口
│       ├── syscall.rs             # 系统调用探针
│       ├── io.rs                  # 块设备 I/O 探针
│       ├── sched.rs               # 调度器探针
│       ├── net.rs                 # 网络探针
│       ├── lock.rs                # 锁探针
│       └── uprobe.rs              # 用户态动态探针

uof-probe-runtime/                 # 用户态运行时
└── src/
    ├── lib.rs
    ├── probe_loader.rs            # 探针加载器
    ├── process_discovery.rs       # 进程发现
    ├── symbol_resolver.rs         # 符号解析
    └── ring_buffer_consumer.rs    # Ring buffer 消费
```

---

## 3. 探针类型

| 类型 | 描述 | 观测点 |
|------|------|--------|
| **kprobe** | 内核函数入口探针 | 函数参数、返回值 |
| **kretprobe** | 内核函数返回探针 | 返回值、执行时长 |
| **tracepoint** | 内核静态跟踪点 | sched_switch、block_rq_complete 等 |
| **uprobe** | 用户态函数入口探针 | 函数参数 |
| **uretprobe** | 用户态函数返回探针 | 返回值、执行时长 |

---

## 4. 核心模块

### 4.1 进程发现 (`process_discovery.rs`)

根据进程名自动发现目标进程。

```rust
pub struct ProcessDiscovery;

impl ProcessDiscovery {
    /// 根据进程名查找所有匹配的 PID
    pub fn find_pids(&self, process_name: &str) -> Result<Vec<u32>>;

    /// 过滤容器内进程（可选，配合 cgroup_id）
    pub fn find_pids_in_cgroup(&self, process_name: &str, cgroup_id: u64) -> Result<Vec<u32>>;
}
```

**实现方式**：
- 扫描 `/proc/*/comm` 获取进程名
- 或使用 `nix::sys::ps::PsindFlags` 遍历进程

### 4.2 符号解析 (`symbol_resolver.rs`)

将函数符号名解析为内存地址。

```rust
pub struct SymbolResolver;

impl SymbolResolver {
    /// 解析指定进程的函数符号地址
    pub fn resolve(&self, pid: u32, binary_path: &str, symbol: &str) -> Result<u64>;

    /// 获取二进制支持的 uprobe 可用符号列表
    pub fn list_symbols(&self, binary_path: &str) -> Result<Vec<String>>;
}
```

**实现方式**：
1. 读取 `/proc/{pid}/maps` 获取二进制文件路径
2. 使用 `addr2line` 或 `nm` 解析符号
3. 对于动态库，需解析加载基址后计算绝对偏移

### 4.3 探针加载器 (`probe_loader.rs`)

```rust
pub struct ProbeLoader {
    aya: Aya,
    probes: HashMap<String, LoadedProbe>,
}

impl ProbeLoader {
    /// 加载内核探针
    pub fn load_kprobe(&mut self, name: &str, fn_name: &str) -> Result<()>;

    /// 加载用户态探针
    pub fn load_uprobe(&mut self, name: &str, pid: u32, path: &str, offset: u64) -> Result<()>;

    /// 卸载探针
    pub fn unload(&mut self, name: &str) -> Result<()>;

    /// 列出所有已加载探针
    pub fn list_loaded(&self) -> Vec<ProbeInfo>;
}
```

### 4.4 Ring Buffer 消费 (`ring_buffer_consumer.rs`)

```rust
pub struct RingBufferConsumer {
    ringbuf: RingBuf,
}

impl RingBufferConsumer {
    /// 启动消费循环
    pub async fn start(&mut self, handler: impl FnMut EventHeader) -> Result<()> {
        loop {
            let events = self.ringbuf.poll().await?;
            for event in events {
                handler(event)?;
            }
        }
    }
}
```

---

## 5. 事件定义

### 5.1 事件头

所有事件共享 `EventHeader`：

```rust
#[repr(C)]
pub struct EventHeader {
    pub ts_ns: u64,           // 时间戳
    pub event_type: u16,      // 事件类型
    pub version: u16,         // 版本
    pub cpu_id: u32,
    pub pid: u32,             // 进程 ID
    pub tid: u32,            // 线程 ID
    pub uid: u32,
    pub gid: u32,
    pub cgroup_id: u64,
    pub mount_ns: u64,
    pub payload_len: u32,
}
```

### 5.2 事件类型常量

```rust
pub const EVENT_TYPE_SYSCALL: u16 = 1;
pub const EVENT_TYPE_IO: u16 = 2;
pub const EVENT_TYPE_SCHED: u16 = 3;
pub const EVENT_TYPE_NET: u16 = 4;
pub const EVENT_TYPE_LOCK: u16 = 5;
pub const EVENT_TYPE_UPROBE: u16 = 6;
```

---

## 6. 用户态探针 (Uprobe) 实现

### 6.1 探针数据结构

```rust
#[repr(C)]
pub struct UprobeEvent {
    pub hdr: EventHeader,
    pub func_addr: u64,        // 函数地址
    pub ret_addr: u64,         // 返回地址
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
}
```

### 6.2 uprobe.rs 实现

```rust
use aya_ebpf::{macros::{uprobe, uretprobe}, programs::UProbeCtx};

#[uprobe]
pub fn handle_pqexec_entry(ctx: UProbeCtx) -> i64 {
    let arg0 = ctx.arg::<u64>(0)?;
    let event = UprobeEvent {
        hdr: make_header(EVENT_TYPE_UPROBE),
        func_addr: 0, // 运行时填充
        ret_addr: ctx.ret_probe_regs().map(|r| r.ip).unwrap_or(0),
        arg0,
        ..Default::default()
    };
    submit_ringbuf(&event)
}

#[uretprobe]
pub fn handle_pqexec_return(ctx: UProbeCtx) -> i64 {
    let ret = ctx.ret_probe_regs().map(|r| r.ax).unwrap_or(0);
    let event = make_return_event(ret);
    submit_ringbuf(&event)
}
```

---

## 7. 使用流程

### 7.1 典型场景：观测 PostgreSQL SQL 执行

```rust
// 1. 发现 postgres 进程
let pids = process_discovery.find_pids("postgres")?;

// 2. 解析 PQexec 函数地址
let addr = symbol_resolver.resolve(pids[0], "/usr/lib/postgresql/15/bin/postgres", "PQexec")?;

// 3. 加载探针
probe_loader.load_uprobe("pg_sql", pids[0], binary_path, addr)?;

// 4. 消费事件
ringbuf_consumer.start(|hdr| {
    if hdr.event_type == EVENT_TYPE_UPROBE {
        let event = decode::<UprobeEvent>(hdr);
        println!("SQL executed, ret={}", event.ret);
    }
}).await?;
```

### 7.2 全量观测模式

```rust
// 列出所有可用符号
let symbols = symbol_resolver.list_symbols("/usr/lib/postgresql/15/bin/postgres")?;

// 对每个符号加载 uprobe
for symbol in symbols {
    probe_loader.load_uprobe(&symbol, pid, binary_path, addr)?;
}
```

### 7.3 预定义函数白名单

```yaml
# postgres-plugin/manifest.yaml
default_probes:
  - name: PQexec
    description: "执行 SQL 查询"
  - name: PQprepare
    description: "预处理语句"
  - name: Begin
    description: "开始事务"
  - name: Commit
    description: "提交事务"
```

---

## 8. 错误处理

| 错误场景 | 处理方式 |
|----------|----------|
| 进程不存在 | 返回错误，用户需确认进程名 |
| 符号解析失败 | 尝试 fallback 地址，或提示用户检查符号表 |
| 探针加载失败 | 内核验证器拒绝，输出详细错误信息 |
| 进程退出 | 自动 detach 探针，清理资源 |

---

## 9. 限制与约束

1. **内核版本**：需要 Linux 4.18+
2. **权限**：需要 `CAP_BPF` + `CAP_PERFMON`（或 `CAP_SYS_ADMIN`）
3. **符号表**：目标二进制需要包含 debug symbols（`nm -D` 可用）
4. **采样率**：全量观测时必须设置采样率上限，防止数据风暴
5. **函数重载**：共享库多版本共存时需精确指定路径

---

## 10. 后续扩展

- [ ] 支持用户自定义 arg 提取规则（如解析 SQL 字符串）
- [ ] 支持 tracepoint 动态探针（需要 rootfs 内核源码）
- [ ] 支持多进程联合观测（fork/exec 后的子进程追踪）
- [ ] 内核探针与 uprobe 的关联（如记录 sys_open 的调用栈）

---

## 11. 验收标准

1. 可对 postgres 进程加载 `PQexec` uprobe，成功捕获函数调用
2. 可对 nginx 进程加载 `ngx_http_handler` uprobe
3. 仅提供进程名时，使用白名单函数进行观测
4. 提供 "trace all" 标志时，可对目标进程所有符号加载探针
5. 探针可卸载，进程不受影响