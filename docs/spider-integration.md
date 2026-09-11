# Protobuf producer 接入

本文档说明行情 producer 如何使用 `market_protocol` 的 generated Protobuf 类型。旧 JSON producer 已删除，不是兼容路径。

## 1. 构造行情消息

所有消息来自：

```text
proto/market_protocol/v1/market_protocol.proto
```

Rust producer 直接构造 generated 类型：

```rust
use market_protocol::v1;

let frame = v1::TransportFrame {
    payload: Some(v1::transport_frame::Payload::Market(
        v1::MarketEnvelope {
            symbol: Some(v1::Symbol {
                market: Some(v1::Market::A as i32),
                code: Some("560010".to_owned()),
            }),
            payload: Some(v1::market_envelope::Payload::Quote(v1::Quote {
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
            })),
        },
    )),
};
```

Python producer 使用 `market_protocol.v1.market_protocol_pb2` 中的同名 generated 类型。

## 2. 单位与时间

- `ts` 使用 epoch milliseconds；
- `volume` 使用股；
- `amount` 使用元；
- 股票代码始终使用字符串，保留前导零；
- 来源格式和来源单位必须在 producer 的 source adapter 中转换，不能写入协议层。

## 3. Tick 游标

`Tick.serial` 是同一标的交易日内的单调游标：

- 同一秒的多笔成交依靠 `serial` 区分；
- `serial = -1` 表示来源清盘/恢复；
- producer 必须按 `(ts, serial)` 顺序发送；
- resume 的实际存储和重放由消费服务负责，本 crate 不提供持久化。

## 4. 发送 Binary frame

使用 `prost::Message` 直接编码：

```rust
use prost::Message;

let bytes = frame.encode_to_vec();
// 将 bytes 作为 WebSocket Binary message 发送。
```

不要：

- 先把消息转 JSON；
- 手工拼 Protobuf bytes；
- 把 `TransportFrame` 转成字符串；
- 同时发送同一事件的 JSON 和 Binary 两份。

## 5. 控制帧

heartbeat 和 resume 也是 `TransportFrame` 的 Protobuf oneof：

```rust
let heartbeat = v1::TransportFrame {
    payload: Some(v1::transport_frame::Payload::Heartbeat(v1::Heartbeat {})),
};

let resume = v1::TransportFrame {
    payload: Some(v1::transport_frame::Payload::Resume(v1::Resume {
        since_ts: Some(123),
        since_serial: Some(456),
    })),
};
```

producer 只发送业务数据；heartbeat/resume 的方向和生命周期由 WebSocket 服务负责。

## 6. Python 生成代码

```bash
uv sync --locked
uv run python scripts/generate_python.py
```

运行时只需要 `protobuf`，不需要安装 `protoc`。生成脚本和 generated module 必须始终以仓库内 `.proto` 为输入。

## 7. 能力边界

本协议只定义消息结构和 Binary 编码，不实现：

- 真实 WebSocket 客户端或服务端；
- 鉴权、订阅隔离和连接管理；
- ACK、outbox、持久化和可靠重放；
- 数据源访问、质量门禁和离线存储；
- 交易执行。
