# M6 market_webserver 接入点

本文档给 `market_webserver`（以及任何 WS 消费方）的接入者看：帧分拣、
反序列化、fan-out、断线续推和 heartbeat/重连退避配置。

## 1. 帧分拣

WS text frame 只有两类：

- **消息帧**：`MarketMessage` 的 JSON，顶层 `type` 为 `quote` / `tick` /
  `kline` / `quote_snapshot`。
- **控制帧**：heartbeat / resume，不能反序列化为 `MarketMessage`。

Rust 侧先用辅助函数分拣：

```rust
use market_protocol::frame::{is_control_frame, heartbeat_json, resume_json};

match is_control_frame(&frame) {
    Some("heartbeat") => { /* 重置连接超时 */ }
    Some("resume") => { /* 解析 since_ts / since_serial 并续推 */ }
    _ => {
        let msg: market_protocol::message::MarketMessage =
            serde_json::from_str(&frame)?;
        fanout(&msg);
    }
}
```

- 未知 `type` 不是控制帧，`from_str::<MarketMessage>` 会 fail loud；收到后
  记日志并丢弃，不要当已知类型处理。
- `heartbeat_json()` / `resume_json()` 只用于生成控制帧。

## 2. 反序列化与 fan-out

webserver 收到消息帧后统一反序列化为 `MarketMessage`，然后分发给订阅者：

- 订阅者是异构处理器集合时，用 `&dyn Message` 统一消费：
  `msg.kind()` / `msg.symbol()` / `msg.timestamp()`。
- 需要具体类型做业务（例如 Tick 的 serial）时，再 match `MarketMessage`。
- M3 裁决：`Message` trait 保留，同时提供 enum 方法；两者不冲突。

## 3. 断线续推（`(ts, serial)` 双键游标）

### 客户端侧

- 持续记录最后收到的 `(ts, serial)`。
- 断线重连后先发 resume 帧：

```text
{"type":"resume","since_ts":123,"since_serial":456}
```

### 服务器侧

- **流全序约束（服务器责任）**：服务器必须按 `(ts, serial)` 全序推送；
  非 Tick 消息视为位于 `(ts, 0)`。即同一 `ts` 内，Quote/Kline/QuoteSnapshot
  必须先于该秒的 Tick 发送。
- 收到 resume 后，从游标之后续推：
  - `ts > since_ts` 的全部消息；
  - `ts == since_ts && serial > since_serial` 的 Tick。
- 容错：如果接入方无法保证上述全序，宁可把同 `ts` 的非 Tick 也重推
  （幂等无害，Kline 按 `ts` 去重），也不要让它们成为缺口。
- 语义分界：
  - **Tick**：`(ts, serial)` 精确定位，保证不丢不重。
  - **Quote / QuoteSnapshot**：最新快照，重复推送幂等无害（覆盖即可）。
  - **Kline**：按 `ts` 去重。

### 边界

- `since_ts = 0`：全量重推。
- `since_serial` 只对 Tick 有意义；非 Tick 消息不使用它。
- 游标已到流尾：不推新消息，保持连接等 heartbeat/新数据。

## 4. heartbeat / 重连退避配置点

协议只定帧格式，不定死参数。推荐配置项：

| 参数 | 默认值 | 说明 |
|---|---|---|
| `heartbeat_interval_secs` | 15 | webserver 发送 heartbeat 的间隔 |
| `client_timeout_secs` | 45 | 客户端超过该时间未收到任何帧则断开 |
| `reconnect_base_secs` | 1 | 客户端重连初始退避 |
| `reconnect_max_secs` | 60 | 重连退避上限 |

Rust 常量默认值见 `market_protocol::frame::DEFAULT_HEARTBEAT_INTERVAL_SECS`；
线上实现应允许从配置文件覆盖，不要写死在业务代码里。

## 5. 最小可运行示例

```bash
# 内置样本
cargo run --example mock_webserver

# 管道接收 mock_spider 帧流
cargo run --example mock_spider | cargo run --example mock_webserver
```
