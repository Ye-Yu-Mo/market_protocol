# market_protocol

`market_protocol` 是行情数据的 **Protobuf-only wire protocol**。协议模型只维护一份：

```text
proto/market_protocol/v1/market_protocol.proto
        ├── Rust prost generated types
        └── Python protobuf generated module
```

不保留 JSON data frame、JSON control frame 或 JSON 兼容层。Rust 和 Python 消费方直接使用 generated Protobuf types；`market_data` 的离线数据标准化和存储由其自身项目负责。

## 协议消息

当前只包含 Protobuf 开发前已经存在的四种行情消息：

| Message | 用途 |
|---|---|
| `Quote` | 最新成交快照 |
| `Tick` | 逐笔成交，`serial` 用于游标 |
| `Kline` | OHLCV 与周期 |
| `QuoteSnapshot` | 盘口档位 |

传输控制消息：

| Message | 用途 |
|---|---|
| `Heartbeat` | 连接保活 |
| `Resume` | 按 `(since_ts, since_serial)` 请求续推 |

`MarketEnvelope` 使用 `oneof` 携带四种行情消息，`TransportFrame` 使用 `oneof` 区分行情、heartbeat 和 resume。

## 协议约定

- `ts`：epoch milliseconds，`int64`；
- `volume`：股；
- `amount`：元；
- `Symbol.code`：字符串，保留股票代码前导零；
- `Period` enum 数值固定，业务周期为 `1m`、`5m`、`15m`、`30m`、`60m`、`1d`、`1w`、`1M`；
- 需要区分缺失和零值的字段使用 `proto3 optional`；
- 已发布 field number 和 enum number 不得复用；删除字段必须使用 `reserved`。

Protobuf bytes 是单个 `TransportFrame`，不经过 JSON。未知字段按 protobuf runtime 的兼容规则处理；缺失 payload、损坏 wire、非法 varint 等输入必须由消费者拒绝。

## Rust

Rust 类型由 `build.rs` 在构建时从 `.proto` 生成：

```rust
use market_protocol::v1;
use prost::Message;

let quote = v1::Quote {
    ts: Some(1_787_904_896_000),
    price: Some(3.208),
    open: Some(3.211),
    high: Some(3.245),
    low: Some(3.204),
    pre_close: Some(3.220),
    volume: Some(132_886_000.0),
    amount: Some(428_272_326.0),
    in_vol: Some(69_090_700.0),
    out_vol: Some(63_795_300.0),
};
```

完整 envelope：

```rust
use market_protocol::v1;
use prost::Message;

let frame = v1::TransportFrame {
    payload: Some(v1::transport_frame::Payload::Market(
        v1::MarketEnvelope {
            symbol: Some(v1::Symbol {
                market: Some(v1::Market::A as i32),
                code: Some("560010".to_owned()),
            }),
            payload: Some(v1::market_envelope::Payload::Quote(quote)),
        },
    )),
};

let bytes = frame.encode_to_vec();
let decoded = v1::TransportFrame::decode(bytes.as_slice())?;
```

## Python

Python generated module 位于 `python/market_protocol/v1/market_protocol_pb2.py`：

```python
from market_protocol.v1 import market_protocol_pb2 as pb

frame = pb.TransportFrame(
    market=pb.MarketEnvelope(
        symbol=pb.Symbol(market=pb.MARKET_A, code="560010"),
        quote=pb.Quote(
            ts=1_787_904_896_000,
            price=3.208,
            volume=132_886_000.0,
            amount=428_272_326.0,
        ),
    ),
)

wire = frame.SerializeToString()
parsed = pb.TransportFrame.FromString(wire)
assert parsed.market.quote.HasField("price")
```

生成 Python module：

```bash
uv sync --locked
uv run python scripts/generate_python.py
```

运行环境只需要 `protobuf` runtime，不需要安装 `protoc`。

## 历史行情服务协议

`market_protocol` 只定义历史服务的 Binary message，不连接数据库。`market_data`
负责从 `market_v2.duckdb`、`free-stockdb` 读取和标准化数据，再使用这些 generated types
对外提供历史行情服务。

```python
from market_protocol.v1 import market_protocol_pb2 as pb

request = pb.HistoryRequest(
    request_id="req-1",
    symbols=[pb.Symbol(market=pb.MARKET_A, code="000001")],
    period=pb.PERIOD_D1,
    start_date="2026-01-01",
    end_date="2026-01-31",
    adjustment=pb.ADJUSTMENT_RAW,
    page_size=500,
)
wire = pb.TransportFrame(history_request=request).SerializeToString()
```

服务端返回 `HistoryResponse`，其中 `HistoryChunk.records` 复用同一个 `Kline` 数据模型；
`trade_date` 保留离线交易日的精确日历键，`next_page_token` 用于分页，`snapshot_id`
用于标识本次数据快照。请求不携带底层数据库名称，数据源选择由 `market_data` 内部负责。

`Adjustment` 必须显式指定 `raw`、`qfq` 或 `hfq`。特别是 `free-stockdb` 的 SDK 默认
返回前复权数据，adapter 不得把默认值伪装成原始行情。

## 示例

```bash
cargo run --example to_binary | xargs cargo run --example from_binary
```

两个示例都直接使用 generated `TransportFrame`，没有 JSON 中间层。

## 开发检查

```bash
python3 scripts/check_proto_schema.py
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo doc --no-deps
uv run python scripts/generate_python.py
uv run pytest -q
```

## 项目边界

本仓库只负责 `.proto`、Rust/Python generated protocol types、Binary 编解码和 wire contract。不负责：

- 数据源访问和字段标准化；
- quality gate、Parquet/DuckDB、manifest、快照发布；
- 真实 WebSocket server、鉴权和订阅管理；
- ACK、outbox、可靠重放和交易执行。

这些工作由对应项目维护，不能反向增加本仓库的第二套数据模型。

## License

MIT
