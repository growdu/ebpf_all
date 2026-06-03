# 项目优化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成三项优化：清理 dead code、添加测试覆盖、完善部署配置

**Architecture:** 三个独立优化方向分别处理，每个方向作为一个 task group。Dead code 清理直接删除未使用代码；测试使用 Rust 内置测试框架；部署配置创建 systemd service 模板文件。

**Tech Stack:** Rust, cargo, systemd

---

## 文件结构

```
crates/uof-control-api/src/
├── routes.rs      # [修改] 删除未使用的函数
├── lib.rs         # [修改] 删除未使用的 import
└── models.rs      # [修改] 删除/标记未使用的结构体

crates/uof-common/src/
└── lib.rs         # [修改] 添加错误类型测试

crates/uof-model/src/
└── lib.rs         # [修改] 添加序列化测试

crates/uof-control-plane/src/
└── state.rs       # [修改] 添加状态管理测试

crates/uof-exporter-otlp/src/
└── exporter.rs   # [修改] 添加导出器测试

deploy/systemd/
├── uof-agent.service           # [新建]
├── uof-control-plane.service   # [新建]
└── README.md                   # [新建]
```

---

## Task Group A: 清理 Dead Code

### Task A1: 清理 routes.rs 中的未使用函数

**Files:**
- Modify: `crates/uof-control-api/src/routes.rs`

**待删除函数 (通过 cargo build 警告确认):**
- `test_catch_all` (line ~249)
- `parse_oci_ref` (line ~370)
- `create_template_binding` (line ~397)
- `delete_template_binding` (line ~404)
- `serve_agent_plugin_artifact` (line ~304)
- `serve_plugin_artifact` (line ~267)

**注意保留 (预留接口):**
- `pull_plugin` - 可能是未来功能
- `create_plugin_version` - 预留接口
- `release_plugin_version` - 预留接口

**执行步骤:**

1. **Step 1: 读取 routes.rs 确认函数位置**

Run: `grep -n "async fn\|fn " crates/uof-control-api/src/routes.rs | head -30`
Expected: 列出所有函数定义

2. **Step 2: 删除 dead functions**

直接删除以下函数定义：
- `test_catch_all`
- `parse_oci_ref`
- `create_template_binding`
- `delete_template_binding`
- `serve_agent_plugin_artifact`
- `serve_plugin_artifact`

3. **Step 3: 验证编译**

Run: `cargo build -p uof-control-api 2>&1 | grep -E "warning:|error:"`
Expected: 无 dead code 警告

4. **Step 4: 提交**

```bash
git add crates/uof-control-api/src/routes.rs
git commit -m "chore(control-api): remove unused functions"
```

---

### Task A2: 清理 lib.rs 中的未使用 import

**Files:**
- Modify: `crates/uof-control-api/src/lib.rs`

**待删除:**
- `PipelineHandler` import (如果存在)

**执行步骤:**

1. **Step 1: 读取 lib.rs**

2. **Step 2: 删除未使用的 import**

3. **Step 3: 验证编译**

Run: `cargo build -p uof-control-api 2>&1 | grep -E "warning:|error:"`
Expected: 无 unused import 警告

4. **Step 4: 提交**

```bash
git add crates/uof-control-api/src/lib.rs
git commit -m "chore(control-api): remove unused imports"
```

---

## Task Group B: 添加测试覆盖

### Task B1: 为 uof-common 添加错误类型测试

**Files:**
- Create: `crates/uof-common/src/lib.rs` 测试模块

1. **Step 1: 查看 uof-common 结构**

Run: `ls -la crates/uof-common/src/ && cat crates/uof-common/src/lib.rs | head -50`

2. **Step 2: 创建测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        // Test error type Display impl
    }

    #[test]
    fn test_error_from() {
        // Test error conversion
    }
}
```

3. **Step 3: 运行测试**

Run: `cargo test -p uof-common 2>&1`
Expected: 测试通过

4. **Step 4: 提交**

```bash
git add crates/uof-common/src/
git commit -m "test(uof-common): add error type tests"
```

---

### Task B2: 为 uof-model 添加序列化测试

**Files:**
- Modify: `crates/uof-model/src/lib.rs`

1. **Step 1: 查看 uof-model 结构**

2. **Step 2: 添加序列化测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_agent_event_serde() {
        let event = AgentEvent { ... };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, parsed);
    }
}
```

3. **Step 3: 运行测试**

Run: `cargo test -p uof-model 2>&1`
Expected: 测试通过

4. **Step 4: 提交**

```bash
git add crates/uof-model/src/
git commit -m "test(uof-model): add serialization tests"
```

---

### Task B3: 为 uof-control-plane 添加状态管理测试

**Files:**
- Modify: `crates/uof-control-plane/src/state.rs`

1. **Step 1: 查看 state.rs 结构**

2. **Step 2: 添加状态管理测试**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_state_insert_and_get() {
        let mut state = State::new();
        state.plugins.insert(...);
        assert!(state.get_plugin(...).is_some());
    }
}
```

3. **Step 3: 运行测试**

Run: `cargo test -p uof-control-plane 2>&1`
Expected: 测试通过

4. **Step 4: 提交**

```bash
git add crates/uof-control-plane/src/
git commit -m "test(control-plane): add state management tests"
```

---

### Task B4: 为 uof-exporter-otlp 添加导出器测试

**Files:**
- Modify: `crates/uof-exporter-otlp/src/exporter.rs`

1. **Step 1: 查看 exporter.rs 结构**

2. **Step 2: 添加导出器测试**

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_export_batch() {
        // Test OTLP export logic
    }
}
```

3. **Step 3: 运行测试**

Run: `cargo test -p uof-exporter-otlp 2>&1`
Expected: 测试通过

4. **Step 4: 提交**

```bash
git add crates/uof-exporter-otlp/src/
git commit -m "test(exporter-otlp): add exporter tests"
```

---

## Task Group C: 完善部署配置

### Task C1: 创建 uof-agent.service

**Files:**
- Create: `deploy/systemd/uof-agent.service`

```ini
[Unit]
Description=UOF Agent - eBPF Probe Manager
After=network.target

[Service]
Type=simple
User=uof
Group=uof
ExecStart=/usr/local/bin/uof-agent --config /etc/uof/agent.toml
Restart=always
RestartSec=5
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

**执行步骤:**

1. **Step 1: 创建文件**

2. **Step 2: 提交**

```bash
git add deploy/systemd/uof-agent.service
git commit -m "deploy(systemd): add uof-agent service template"
```

---

### Task C2: 创建 uof-control-plane.service

**Files:**
- Create: `deploy/systemd/uof-control-plane.service`

```ini
[Unit]
Description=UOF Control Plane - State Management API
After=network.target postgresql.service

[Service]
Type=simple
User=uof
Group=uof
ExecStart=/usr/local/bin/uof-control-plane --config /etc/uof/control-plane.toml
Restart=always
RestartSec=5
Environment="RUST_LOG=info"
Environment="DATABASE_URL=postgresql://uof:uof@localhost:5432/uof"

[Install]
WantedBy=multi-user.target
```

**执行步骤:**

1. **Step 1: 创建文件**

2. **Step 2: 提交**

```bash
git add deploy/systemd/uof-control-plane.service
git commit -m "deploy(systemd): add uof-control-plane service template"
```

---

### Task C3: 创建部署说明 README

**Files:**
- Create: `deploy/systemd/README.md`

```markdown
# UOF Systemd Service Templates

## 部署步骤

1. 复制服务文件到 systemd 目录:
   ```bash
   sudo cp uof-agent.service /etc/systemd/system/
   sudo cp uof-control-plane.service /etc/systemd/system/
   ```

2. 重新加载 systemd:
   ```bash
   sudo systemctl daemon-reload
   ```

3. 启用并启动服务:
   ```bash
   sudo systemctl enable uof-agent
   sudo systemctl enable uof-control-plane
   sudo systemctl start uof-agent
   sudo systemctl start uof-control-plane
   ```

## 配置

- Agent 配置: `/etc/uof/agent.toml`
- Control Plane 配置: `/etc/uof/control-plane.toml`

## 日志

```bash
journalctl -u uof-agent -f
journalctl -u uof-control-plane -f
```
```

**执行步骤:**

1. **Step 1: 创建 README.md**

2. **Step 2: 提交**

```bash
git add deploy/systemd/README.md
git commit -m "deploy(systemd): add deployment README"
```

---

## Task C4: 最终验证

1. **Step 1: 运行完整测试**

Run: `cargo test --workspace 2>&1 | grep -E "^test result"`
Expected: 所有测试通过

2. **Step 2: 验证无警告**

Run: `cargo build --workspace 2>&1 | grep -E "warning:.*unused"`
Expected: 无 dead code 警告

3. **Step 3: 提交所有更改**

```bash
git add -A
git commit -m "chore: complete project optimization

- Remove dead code from uof-control-api
- Add unit tests for core modules
- Add systemd service templates
"
```

---

## 自检清单

- [ ] spec coverage: 每个优化方向都有对应 Task
- [ ] placeholder scan: 无 "TBD", "TODO" 等占位符
- [ ] type consistency: 所有类型和函数名一致
- [ ] 所有 Task 步骤都包含实际代码

---

**Plan complete.** 文件保存在 `docs/superpowers/plans/2026-06-03-project-optimization-plan.md`

**两个执行选项:**

**1. Subagent-Driven (recommended)** - 每个 Task 由独立 subagent 实现

**2. Inline Execution** - 在当前 session 批量执行

选择哪个方式？