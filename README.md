# market_protocol

[![CI](https://github.com/Ye-Yu-Mo/market_protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/Ye-Yu-Mo/market_protocol/actions/workflows/ci.yml)

行情数据在 `spider → market_webserver → factor_maker / frontend` 管道间传输的 **wire protocol**：
统一消息类型 + JSON 序列化/反序列化（前向兼容）+ WS 传输帧约定。

协议写成代码（crate）而非文档，消费方**编译期共享**同一份类型与 ser/de，从根上杜绝字段命名、时间格式、价格精度的各自为政。

```text
spider ──market_protocol (JSON over WS)──▶ market_webserver ──▶ factor_maker / frontend
```

## 协议契约

- **时间戳**：所有消息的 `ts` 统一为 **epoch 毫秒 `i64`**。源格式（元大 `yyyy/MM/dd`、free-stockdb 8/14 位整数、FinMind `YYYY-MM-DD`）一律在 spider 层转换，协议层无时区/格式歧义。
- **量单位**：`volume` 统一以「股」为单位；`amount` 为「元」。腾讯"手"×100、元大期货"千股"等源单位由 spider 换算。
- **envelope**：`{"type":"quote","symbol":{"market":"A","code":"560010"},"data":{...}}`——**顶层无 `ts`**（`ts` 属于 data）；`type` / `symbol` / `data` 字段名自 M2 起不可改。
- **前向兼容**：未知字段容忍 + 新增字段必须 `Option` / `#[serde(default)]` + 未知 `type` fail loud（不静默误解析）。

## 消息类型

| 类型 | `type` 标签 | 说明 |
|---|---|---|
| `Quote` | `quote` | 最新成交快照（price / OHLC / volume / amount / 内外盘） |
| `Tick` | `tick` | 逐笔成交（含 `serial` 断线续推游标） |
| `Kline` | `kline` | OHLCV + `period` + 离线字段（`amount` / `turnover` / `pre_close`） |
| `QuoteSnapshot` | `quote_snapshot` | 盘口五档（档数由源决定） |

三市场差异（A 股整手 100、台股繁体编码、crypto 小数份额）用字段表达，不造理论子类型。

## 快速开始

```rust
use market_protocol::message::MarketMessage;
use market_protocol::types::Quote;
use market_protocol::{Market, Symbol};

// 构造一条真实样本消息（A 股 560010，腾讯实测报价）。
let msg = MarketMessage::Quote {
    symbol: Symbol::new(Market::A, "560010"),
    data: Quote {
        ts: 1_787_904_896_000,     // epoch 毫秒
        price: 3.208,
        open: 3.211,
        high: 3.245,
        low: 3.204,
        pre_close: 3.220,
        volume: 132_886_000.0,     // 股（腾讯"手"已 ×100）
        amount: 428_272_326.0,     // 元
        in_vol: Some(69_090_700.0),
        out_vol: Some(63_795_300.0),
    },
};

let json = serde_json::to_string(&msg)?;
// {"type":"quote","symbol":{"market":"A","code":"560010"},"data":{"ts":...,"price":3.208,...}}

let parsed: MarketMessage = serde_json::from_str(&json)?;
println!("kind={} symbol={:?} ts={}", parsed.kind(), parsed.symbol(), parsed.timestamp());
```

`kind()` / `symbol()` / `timestamp()` 来自 [`Message`] trait，消费方可用 `&dyn Message` 统一处理任意消息。

## 传输帧约定

| 帧 | 格式 | 方向 |
|---|---|---|
| 消息帧 | `MarketMessage` JSON | spider → webserver |
| heartbeat | `{"type":"heartbeat"}` | webserver → 客户端（间隔可配置，默认 15s） |
| resume | `{"type":"resume","since_ts":...,"since_serial":...}` | 客户端 → webserver（断线重连续推） |

断线续推按 `(ts, serial)` 双键游标：`Tick` 精确定位（同一秒多笔成交靠 `serial` 不丢不重），快照类消息（Quote/QuoteSnapshot）允许 ts 粒度幂等重推，`Kline` 按 `ts` 去重。**服务器必须按 `(ts, serial)` 全序推送**（非 Tick 视为 `(ts, 0)`），否则游标后的同 `ts` 非 Tick 可能成为缺口。

```rust
use market_protocol::frame::{heartbeat_json, is_control_frame, resume_json};

assert_eq!(is_control_frame(&heartbeat_json()), Some("heartbeat"));
assert_eq!(is_control_frame(&resume_json(123, 456)), Some("resume"));
```

## 示例

```bash
cargo run --example to_json          # 序列化真实样本 envelope
cargo run --example from_json        # 解析真实样本 JSON
cargo run --example mock_spider      # 产出样本帧流
cargo run --example mock_webserver   # 收帧、分拣、fan-out 到两个订阅者
cargo run --example mock_spider | cargo run --example mock_webserver  # 管道串联
```

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
```

## 文档

- `docs/spider-integration.md` — M4 spider 接入点
- `docs/webserver-integration.md` — M6 webserver 接入点
- `CHANGELOG.md` — 变更记录

## License

MIT
