# Changelog

变更记录

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)

版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)

---

## 版本号说明

- **主版本号（Major）**：不兼容的 API 变更或架构重构
- **次版本号（Minor）**：向后兼容的功能新增（新模块、新页面、新接口）
- **修订号（Patch）**：向后兼容的问题修正、小优化、文档更新

---

## [Unreleased]

### Changed

- 将协议收敛为 Protobuf-only：`.proto` 是唯一 schema，Rust/Python 使用 generated types。
- 从 `fbb3b22` 的四种行情 JSON 语义提取 v1 Protobuf schema。
- 删除旧 JSON 运行时、JSON 示例、JSON pipeline、订单扩展和重复 domain adapter。
- heartbeat/resume 纳入 Protobuf `TransportFrame`，不再使用 JSON control frame。

---

## [0.1.0] - 2026-08-29

### Added

#### M1: crate 骨架 + 核心行情类型

- 新增 `Market`（A / Tw / Crypto）与 `Symbol`（market + code）
- 新增核心行情类型 `src/types.rs`：`Quote`（最新快照）、`Tick`（逐笔）、`Kline`（OHLCV + period + 离线字段）、`QuoteSnapshot`（盘口五档）、`Period`、`Level`
- 协议契约定死：时间戳统一 epoch 毫秒 `i64`；`volume` 统一「股」单位（spider 层换算，A 股整手 100 不进协议）
- 三市场差异用 `Option` 字段表达（`in_vol`/`out_vol`/`turnover`/`pre_close`）；`f64` 价格/量支持 crypto 小数份额（预留）
- `Kline` 承载离线源字段（`amount`/`turnover`/`pre_close`），满足历史查询与实时推送共用类型
- 单元测试：round-trip、真实 A/TW 样本 fixture（A 股 560010 / 台股 2330 / 1437）、三市场差异与边界

#### M2: 统一 envelope + JSON 序列化/反序列化

- 新增 `MarketMessage` enum（`src/message.rs`），internally-tagged，envelope `{"type":...,"symbol":{...},"data":{...}}`
- **envelope 顶层无 `ts`**，`ts` 由各消息类型自带（避免双时间戳歧义，且 `Kline` 数组元素自足）
- 前向兼容三原则：未知字段容忍、新增字段必须 `Option` / `#[serde(default)]`、未知 `type` fail loud
- 新增 `symbol()` / `timestamp()` 访问器
- 新增 `examples/to_json.rs` / `from_json.rs`

#### M3: Message trait + 传输帧约定 + 消费方验证

- 新增 `Message` trait（`kind()` / `symbol()` / `timestamp()`），由 `MarketMessage` 实现，fan-out 可用 `&dyn Message` 统一消费
- 新增 `src/frame.rs`：消息帧 / heartbeat / resume 三类帧约定 + 断线 `(ts, serial)` 双键续推 + 流全序约束
- 新增 `examples/mock_spider.rs` / `mock_webserver.rs` 与 `tests/pipeline.rs` 断线续推集成测试（无重复无缺口、快照幂等）
- 新增 `docs/spider-integration.md` / `docs/webserver-integration.md` 接入点文档

### Changed

- 无（首次发布）

### Fixed

- 无（首次发布）

### Security

- 无
