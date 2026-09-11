# PLAN

## 背景

### 当前问题

- 当前项目曾同时维护 Rust domain 类型、JSON envelope 和 Protobuf 类型，重复模型让协议边界变得复杂。
- 项目的真实目标是：从 Protobuf 开发前的四种行情 JSON 语义出发，提取唯一 `.proto`，再生成 Rust 和 Python 版本。
- Protobuf 开发前的基线是提交 `fbb3b22`，包含 `Quote`、`Tick`、`Kline`、`QuoteSnapshot` 以及 heartbeat/resume 传输约定。
- 后续加入的 `OrderEvent`、订单消息、JSON adapter 和双协议 pipeline 不属于本次协议范围。
- `market_data` 是独立项目；它的来源适配、标准化、质量门禁、Parquet/DuckDB、快照发布和 CLI 由 `market_data/PLAN.md` 负责，不在这里重复规划。

### 为什么这是个真实问题

- 同一份行情语义维护多套模型，会造成字段、单位、缺失值、周期和 enum 的解释漂移。
- `protoc` 生成代码只能保证结构和 wire encoding，不能自动保证跨语言对 optional、oneof、单位和控制帧的理解一致；这些必须在 schema 中一次冻结。
- 旧 JSON 代码和测试会让项目继续验证已经放弃的运行路径，也会误导消费者把 JSON 当成正式协议。
- Rust 和 Python 如果不直接共享同一份 `.proto`，跨语言协议仍然依赖人工同步，最终会回到原来的问题。

### 为什么现在要做

- 当前没有稳定的 Protobuf 外部消费者，正适合一次性删除旧 JSON 运行时和重复 domain 层。
- 现在保留四种既有行情语义，可以避免把订单扩展和协议迁移混为一个项目。
- Rust 和 Python 两端都需要协议代码，必须先确定 `.proto` 是唯一来源，再建立两端生成和互读验证。

## 目标

### 本次要解决什么

- 恢复并锁定 Protobuf 开发前四种行情消息的语义范围。
- 从旧 JSON 版本提取 `Market`、`Symbol`、`Period`、`Quote`、`Tick`、`Kline`、`Level`、`QuoteSnapshot` 的唯一 v1 `.proto` schema。
- 将 heartbeat/resume 纳入 Protobuf `TransportFrame`，不保留 JSON data/control frame。
- 从同一份 `.proto` 生成 Rust `prost` types 和 Python `protobuf` module。
- 删除所有旧 JSON 代码、JSON 测试、JSON 示例、订单扩展和重复 domain adapter。
- 保留 Binary 编解码、optional presence、oneof、enum、malformed input 和 Rust/Python 语义互读测试。

### 不解决什么

- 不保留 JSON producer、consumer、envelope、fallback、转换器或运行时解析器。
- 不保留 `MarketMessage`、`Message` trait、手写 Rust domain struct 或订单事件模型。
- 不实现 `market_data` 的数据源、DataFrame、质量门禁、Parquet/DuckDB、manifest、快照、复制和 CLI。
- 不实现真实 WebSocket server、鉴权、订阅隔离、ACK、outbox、可靠重放或交易执行。
- 不将 Protobuf 作为离线存储格式，也不引入 gRPC 或消息队列。

## 约束

### 相容性要求

- `.proto` 是 Rust 和 Python 的唯一协议来源，禁止手工维护第二套字段定义。
- 四种行情的字段语义、类型、单位和缺失值含义必须从 `fbb3b22` 的 JSON 版本提取并固定。
- `ts` 使用 epoch milliseconds；`volume` 使用股；`amount` 使用元；`Symbol.code` 使用字符串并保留前导零。
- 需要区分缺失和显式零值的 scalar 使用 `proto3 optional`；盘口 `repeated` 字段保留空数组语义。
- 已发布 field number 和 enum number 不得复用；删除字段必须使用 `reserved`；breaking change 进入新 package version。
- `MarketEnvelope` 只允许四种行情 payload；`TransportFrame` 只允许 market、heartbeat、resume 三种 oneof 分支。
- 本次是全量迁移，不提供旧 JSON 兼容承诺；历史 JSON 只作为一次性 schema 提取和迁移输入。

### 性能要求

- Rust 和 Python 直接对 Protobuf bytes 编解码，不经过 JSON 中间转换。
- Rust generated code 在 Cargo build 阶段生成；Python generated code 使用固定版本的 generator 生成，运行环境只依赖 protobuf runtime。
- 以普通实时行情和跨进程传输为目标，不做没有实际需求的 HFT 优化。
- 不承诺不同语言或不同 runtime 产生完全相同的 canonical bytes；exact bytes 只用于固定实现的回归 fixture。

### 可维护性要求

- 固定 schema 路径、Rust 生成入口、Python 生成入口和 generated module 路径。
- 生成类型直接作为协议 API；手写辅助代码只能解决明确的 wire-level 问题。
- schema checker 必须检查 package、message、field number、enum、optional 和 oneof。
- 跨语言测试比较字段、presence、enum、oneof、时间、单位和代码字符串语义。
- schema、生成配置、测试、CI、README 和接入文档必须保持同步。
- `market_data` 只消费稳定的 generated protocol，不反向修改本仓库的离线流程。

### 明确不能 break userspace 的点

- `fbb3b22` 中四种行情的字段语义、单位、Period、代码字符串和 Tick serial 不能改变。
- 不得将缺失值变成零值、将字符串代码变成整数、或将毫秒时间戳变成秒。
- 不得在 Rust 和 Python 中引入各自独立的字段映射和默认值规则。
- 不得在 Rust/Python 生成、Binary 联调和回滚条件未满足前切换外部 producer/consumer。
- 旧 JSON 不再作为运行时兼容接口；若确实需要读取历史 JSON，只能使用独立的一次性迁移工具。

## 里程碑

### Milestone 1: 从旧 JSON 基线冻结行情 Protobuf schema

#### 目标

- 以 `fbb3b22` 为语义基线，提取四种行情和传输控制帧，冻结 v1 schema。

#### 交付物

- `proto/market_protocol/v1/market_protocol.proto`。
- `Market`、`Period`、`Symbol`、`Quote`、`Tick`、`Kline`、`Level`、`QuoteSnapshot`。
- `MarketEnvelope.oneof`，包含四种行情 payload。
- `Heartbeat`、`Resume` 和 `TransportFrame.oneof`。
- schema 注释明确时间、单位、optional、盘口空列表和 Tick 游标语义。
- schema checker，覆盖 package、声明清单、field number、enum 数值、optional 和 oneof。

#### 验收标准

- schema 不包含 `OrderEvent`、订单 enum、JSON envelope 或 JSON type 字段。
- 四种行情字段与 `fbb3b22` JSON 语义一致，股票代码仍为字符串。
- heartbeat/resume 可以作为 `TransportFrame` 的 Protobuf oneof 分支表达。
- 缺少 optional、field number、enum 或 oneof 成员变化时，schema checker 失败。
- schema 是 Rust 和 Python 生成代码的唯一输入。

#### 风险

- JSON 字段顺序不是 schema 语义，不能把序列化顺序误当成字段含义。
- `Option` 缺失与显式零值容易在设计时被合并。
- control frame 若不纳入统一 transport model，就会重新留下双协议边界。
- schema 一旦发布，field number 和 enum 数值不能靠重排修正。

---

### Milestone 2: 由同一 schema 生成 Rust 与 Python 协议

#### 目标

- Rust 和 Python 直接使用同一份 generated Protobuf types，不再维护手写 domain 镜像。

#### 交付物

- Rust `build.rs`、pinned `prost`/`prost-build`/vendored `protoc` 和公开的 `v1` generated module。
- Python `pyproject.toml`、固定 generator/runtime 版本、`scripts/generate_python.py`。
- `python/market_protocol/v1/market_protocol_pb2.py` 及最小 package 入口。
- `examples/to_binary.rs`、`examples/from_binary.rs`，直接构造和解码 `TransportFrame`。
- Rust 测试和 Python 测试，覆盖四种行情、heartbeat、resume、optional presence、oneof 和 malformed wire。
- 删除 `serde`、`serde_json`、JSON 示例、JSON pipeline、订单扩展、旧 domain adapter 和旧 frame classifier。

#### 验收标准

- `cargo build` 能从 `.proto` 生成 Rust 类型并完成 Binary 编解码。
- Python 生成命令能从同一 `.proto` 生成 module，Python 能导入并编解码四种行情和控制帧。
- Rust/Python 对同一 fixture 的字段、enum、oneof、optional presence、时间、单位和代码字符串解释一致。
- `cargo tree` 和源码搜索不再显示旧 JSON 运行时依赖或 `MarketMessage`。
- 运行环境只需要 generated Python module 和 protobuf runtime，不需要运行时 `protoc`。

#### 风险

- Python generator 和 Rust prost 的版本漂移可能导致 generated code 或 presence 行为差异。
- 直接公开 generated types 会牺牲 domain wrapper 的便利性，但消除了重复模型。
- 删除旧 Rust API 是本次全量迁移的预期 breaking change，必须在外部消费者切换前完成。
- Protobuf 合法编码不保证不同 runtime 的 bytes 完全相同，跨语言测试不能错误要求 byte equality。

---

### Milestone 3: Protobuf-only 验证与迁移收口

#### 目标

- 证明 `.proto`、Rust generated code 和 Python generated code 是一条可复现、可互读的协议链路，并清除旧 JSON 残留。

#### 交付物

- Rust/Python 双向 golden vectors，覆盖四种行情和三种 transport frame 类型。
- optional 缺失/显式零值、空盘口、未知字段、未知 enum、截断 bytes、非法 varint 和缺失 payload 测试。
- Binary-only producer/consumer 文档、迁移顺序和失败处理说明。
- CI 中的 schema 检查、Rust 生成构建、Python 生成检查、Rust 测试、Python 测试、format、lint 和 docs。
- 明确外部 producer/consumer 切换要求；实际 spider/webserver 代码留在对应仓库。

#### 验收标准

- Rust encode 的 fixture 可以被 Python 解码，Python encode 的 fixture 可以被 Rust 解码。
- 两端对 optional、enum、oneof、时间戳、单位和股票代码的语义完全一致。
- 旧 JSON 源码、JSON fixture、JSON 示例、JSON parser 和 JSON 运行时依赖均已移除。
- schema 变化能产生明确的 compatibility check 失败信号。
- Binary 解码失败只拒绝当前 frame，不将损坏数据当作有效行情或控制帧。
- 外部业务接入前可以使用固定 Binary fixture 完成联调；本仓库不宣称提供真实 WebSocket 或可靠消息系统。

#### 风险

- 没有真实 Python 消费者时，互读测试可能只验证形式而非实际使用场景。
- 旧 JSON API 删除会影响尚未迁移的外部调用方，必须先完成调用方盘点。
- 将 `market_data` 的 offline bridge 或数据质量流程加入本仓库会重新制造职责耦合。
- 如果迁移过程中同时发送 JSON 和 Binary，会造成重复行情和错误去重。

### Milestone 4: 历史行情服务协议

#### 目标

- 在不暴露 `market_v2.duckdb` 或 `free-stockdb` 内部结构的前提下，为统一历史行情服务定义 Protobuf request/response。
- 让在线和离线数据继续复用 `Quote`、`Tick`、`Kline`、`QuoteSnapshot`，只增加查询、分页、快照和复权元数据。

#### 交付物

- `Adjustment` enum：`RAW`、`QFQ`、`HFQ`。
- `HistoryRequest`：request id、标的列表、周期、明确的交易日期范围、复权方式、分页大小和 page token。
- `HistoryRecord`：`Symbol`、精确 `trade_date` 和现有 `Kline`。
- `HistoryChunk`：records、next page token、完成标记、snapshot id 和实际复权方式。
- `HistoryError` 与 `HistoryResponse.oneof`。
- `TransportFrame` 的历史请求/响应分支（如果服务采用统一 Binary frame）。
- Rust/Python generated types、共享 Binary fixture、presence/分页/错误测试和服务边界文档。

#### 验收标准

- Rust 和 Python 可以从同一 `.proto` 生成并编解码历史 request、chunk 和 error。
- `trade_date`、snapshot id、adjustment、page token、end 和 request id 往返不丢失。
- `market_data` 的两个 source adapter 都可以映射到同一个 `HistoryRecord`，协议不出现 `MarketV2Kline` 或 `FreeStockDbKline`。
- `free-stockdb` 的默认 qfq 行为不会被误当成 raw；复权方式必须显式传递。
- 历史协议不依赖 DuckDB、free-stockdb SDK 或任何 source-specific runtime。

#### 风险

- 两个源的日期格式、复权默认值、缺失值和周期聚合规则不同，不能直接拼接。
- 一次返回全部历史数据会造成内存和网络压力，必须支持分页或 chunk。
- `market_v2` 的辅助表（复权因子、资金流、融资融券、财务数据）不能强行塞进 Kline；有真实消费者后再定义独立 dataset。
- 历史服务的网络实现属于 `market_data`，不能把数据库访问或服务状态塞进 `market_protocol`。

---

## 兼容性检查

- **必须保持的行为**：`fbb3b22` 四种行情的字段语义、时间戳毫秒单位、成交量/金额单位、前导零代码、Period、盘口空数组和 Tick serial。
- **不能被破坏的接口**：v1 `.proto` 的 package、field number、enum number、optional presence、`MarketEnvelope.oneof`、`TransportFrame.oneof` 和 Rust/Python generated module 路径。
- **明确删除的接口**：`MarketMessage`、JSON envelope、serde/serde_json、JSON control frame、订单消息、旧 domain adapter 和测试内伪 server；它们不属于新协议公共 API。
- **schema 调整路径**：新增字段只能追加；删除字段必须 reserved；字段类型、含义或 package 发生 breaking change 时创建新版本 namespace。
- **工具链调整路径**：Rust/Python generator 或 runtime 升级必须同步更新锁定版本、generated output、golden vectors、互读测试和迁移说明。
- **外部消费者迁移路径**：先完成 Rust/Python 生成和 Binary fixture 联调，再切换 producer/consumer；历史 JSON 只能作为一次性迁移输入。
- **项目边界**：`market_data` 的离线标准化和 Protobuf bridge 在其自己的 `PLAN.md` 中维护；本仓库只提供 generated schema 和 wire contract。

## 退出条件

- **什么算完成**：
  - 仓库只保留唯一 `.proto`、Rust/Python generated protocol、Binary 示例、Binary fixture、schema checker 和对应测试文档。
  - 四种行情、heartbeat、resume 都能使用 Protobuf `TransportFrame` 表达。
  - Rust/Python 均可从同一 `.proto` 生成、导入、encode/decode。
  - 所有旧 JSON 代码、JSON 数据、JSON 测试、订单扩展和重复 domain 模型已清理。
  - Rust/Python 语义互读、optional presence、oneof、enum、malformed input 和 schema compatibility 检查全部通过。
  - `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`、`cargo doc --no-deps`、Python generation check 和 Python tests 全部通过。

- **什么情况下应停止继续扩展**：
  - 仍需要 JSON 运行时兼容层，说明外部迁移尚未完成，应先解决消费者切换。
  - Rust/Python 对 optional、enum、oneof、时间或单位的解释不一致。
  - schema 变更可能复用旧 field number、改变旧字段含义或破坏 generated module 路径。
  - 为了弥补 generated types 不方便，又重新引入平行 domain 模型。
  - 将 `market_data` 离线流程、真实 WebSocket、可靠重放或交易执行塞回本仓库。
  - 没有真实调用方却引入复杂 runtime、RPC、消息队列或跨服务状态。
