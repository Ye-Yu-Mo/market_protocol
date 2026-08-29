# M4 spider 接入点

本文档给 `stock_a_spider`（以及后续台股 spider）的接入者看：行情源解析后如何
构造 `MarketMessage`、序列化并按帧推送到 webserver。

## 1. 构造 `MarketMessage`

协议层统一 envelope：

```json
{
  "type": "quote",
  "symbol": {"market": "A", "code": "560010"},
  "data": { "ts": 1787904896000, "...": "..." }
}
```

Rust 侧构造：

```rust
use market_protocol::{Market, Symbol};
use market_protocol::message::MarketMessage;
use market_protocol::types::Quote;

let msg = MarketMessage::Quote {
    symbol: Symbol::new(Market::A, "560010"),
    data: Quote { /* 见 src/types.rs */ },
};
```

- `type` 只能是 `quote` / `tick` / `kline` / `quote_snapshot`。
- `symbol.market` 是 `A` / `Tw` / `Crypto`（Crypto 本阶段只预留）。
- 序列化用 `serde_json::to_string(&msg)`，不要手工拼 JSON。

## 2. 单位换算约定

- **volume 协议单位统一为“股”**。
  - 腾讯 A 股 `手`：`股 = 手 × 100`。
  - 台股若源给“千股/张”，同样先换算成股。
  - crypto 小数份额直接按原值进 `f64`，协议不表达整手。
- **amount 协议单位统一为“元”**。
- 其他源字段若为 `Option<T>`，源没有时给 `None`；不要填 0 冒充真实值。

## 3. 时间格式 → epoch 毫秒

协议时间戳一律是 **epoch 毫秒 `i64`**。

| 源格式 | 转换约定 |
|---|---|
| Unix 秒 | `× 1000` |
| `yyyy/MM/dd HH:mm:ss` | 按源时区解析后转 epoch ms |
| 8/14 位整数（free-stockdb） | 按源说明补足到 epoch ms |
| `YYYY-MM-DD`（FinMind） | 解析为当天 0 点 epoch ms |

不要让协议层出现 `"2024-..."` 字符串或秒级时间戳。

## 4. Tick 必须维护 `serial` 单调递增

`Tick.serial` 是断线续推游标：

- 同一标的、同一交易日的 Tick `serial` 必须严格单调递增。
- 同一秒多笔成交的 `ts` 可能相同，靠 `serial` 区分，不能丢。
- `serial = -1` 是清盘/恢复语义，按源约定使用。
- 服务器按 `(ts, serial)` 双键续推，任何 serial 回退都会造成重复/缺口。

## 5. 推送到 webserver

spider 侧每构造一条 `MarketMessage`，就序列化成一行 JSON，作为 WS text frame
发送：

```text
{"type":"quote","symbol":{"market":"A","code":"560010"},"data":{...}}
{"type":"tick","symbol":{"market":"Tw","code":"2330"},"data":{...}}
```

- 不要发送 heartbeat / resume 帧：这些是 webserver 和客户端之间的控制帧，
  spider 只发消息帧。
- 推荐按 `(ts, serial)` 全序发送：同一 `ts` 内，Quote/Kline/QuoteSnapshot
  先发，Tick 按 `serial` 递增后发。若无法保证，webserver 侧续推会以幂等
  重推同 `ts` 非 Tick 来兜底，但不应依赖这种兜底。
- 连接断开后 spider 按业务决定重连；行情游标续推由 webserver 对订阅者负责，
  spider 不需要自己实现 resume。

## 6. 最小可运行示例

```bash
cargo run --example mock_spider
cargo run --example mock_spider | cargo run --example mock_webserver
```
