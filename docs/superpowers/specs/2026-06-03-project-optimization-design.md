# 项目优化设计

**日期:** 2026-06-03
**项目:** UOF (Universal Observability Framework)
**目标:** 优化项目：清理 dead code、添加测试、完善部署配置

## 1. 概述

当前项目存在三个优化点：
1. Dead code 警告（10+ 个未使用的函数/结构体）
2. 缺少测试覆盖
3. systemd 部署模板为空

## 2. 三个优化方向

### 2.1 清理 Dead Code (uof-control-api)

**目标:** 消除 `cargo build` 产生的未使用代码警告

**待删除/修复项:**
| 文件 | 函数/类型 | 原因 |
|------|----------|------|
| `routes.rs:249` | `test_catch_all` | 测试用的 catch-all handler，未使用 |
| `routes.rs:370` | `parse_oci_ref` | OCI 引用解析，未被调用 |
| `routes.rs:397` | `create_template_binding` | 未使用 |
| `routes.rs:404` | `delete_template_binding` | 未使用 |
| `routes.rs:304` | `serve_agent_plugin_artifact` | 未使用 |
| `routes.rs:267` | `serve_plugin_artifact` | 未使用 |
| `routes.rs:190` | `pull_plugin` | 预留接口，考虑保留 |
| `routes.rs:153` | `create_plugin_version` | 预留接口，考虑保留 |
| `routes.rs:176` | `release_plugin_version` | 预留接口，考虑保留 |
| `models.rs` | `PullPluginBody` | 未使用 |
| lib.rs | `PipelineHandler` import | 未使用 |

**策略:**
- 明显 dead code 直接删除
- 预留接口（如 `pull_plugin`）如果可能是未来功能则用 `#[allow(dead_code)]` 标记

### 2.2 添加测试覆盖

**目标:** 为核心模块添加基本单元测试

**测试范围:**
- `uof-common` - 错误类型测试
- `uof-model` - 数据结构序列化/反序列化测试
- `uof-control-plane` - 状态管理基本测试
- `uof-exporter-otlp` - OTLP 导出逻辑测试

**不包括:**
- eBPF 探针代码（需要 `bpfel-unknown-none` target，未安装）
- 集成测试（需要运行时环境）

**测试框架:** 使用 Rust 内置 `#[test]` 和 `#[tokio::test]`

### 2.3 完善部署配置

**目标:** 创建可用的 systemd 服务模板

**文件清单:**
```
deploy/systemd/
├── uof-agent.service      # Agent 节点守护进程
├── uof-control-plane.service  # Control Plane 服务
└── README.md              # 部署说明
```

**服务模板内容:**
- `uof-agent.service`: Type=simple, EnvironmentFile, ExecStart, Restart=always
- `uof-control-plane.service`: Type=simple, Port=8080, ExecStart, Restart=always

## 3. 执行顺序

三个优化方向独立并行执行，无依赖关系。

## 4. 验收标准

1. **Dead Code:**
   - `cargo build --workspace` 无 dead code 警告

2. **测试覆盖:**
   - 每个目标 crate 至少有 3-5 个单元测试
   - `cargo test --workspace` 全部通过

3. **部署配置:**
   - `deploy/systemd/` 包含可用的 service 文件
   - 服务文件格式符合 systemd 规范