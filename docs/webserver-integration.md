# Protobuf consumer 接入

本文档说明 WebSocket consumer 如何处理 `market_protocol` 的 Binary Protobuf frame。当前协议不提供 JSON 输入路径。

## 1. 解码传输帧

WebSocket Binary message 直接解码为 generated `TransportFrame`：

```rust
use market_protocol::v1;
use prost::Message;

let frame = v1::TransportFrame::decode(bytes.as_slice())?;
match frame.payload {
    Some(v1::transport_frame::Payload::Market(envelope)) => {
        match envelope.payload {
            Some(v1::market_envelope::Payload::Quote(quote)) => { /* quote */ }
            Some(v1::market_envelope::Payload::Tick(tick)) => { /* tick */ }
            Some(v1::market_envelope::Payload::Kline(kline)) => { /* kline */ }
            Some(v1::market_envelope::Payload::QuoteSnapshot(snapshot)) => { /* snapshot */ }
            None => { /* reject missing payload */ }
        }
    }
    Some(v1::transport_frame::Payload::Heartbeat(_)) => { /* keepalive */ }
    Some(v1::transport_frame::Payload::Resume(resume)) => { /* resume request */ }
    None => { /* reject empty transport frame */ }
}
```

Python consumer 使用 `TransportFrame.FromString(bytes)`，再用 `WhichOneof("payload")` 判断帧类型。

Binary decode 失败只能丢弃当前 frame，不能阻断后续独立 frame。

## 2. optional presence

Protobuf scalar 使用 `optional` 时，缺失和显式零值不同：

```python
quote = pb.Quote(price=0.0)
assert quote.HasField("price")

empty = pb.Quote()
assert not empty.HasField("price")
```

Rust generated 类型对应 `Option<T>`。消费方不得用默认零值替代缺失字段。

## 3. 断线续推

客户端记录最后收到的：

```text
(ts, serial)
```

然后发送 Binary `TransportFrame.resume`：

```rust
let frame = v1::TransportFrame {
    payload: Some(v1::transport_frame::Payload::Resume(v1::Resume {
        since_ts: Some(last_ts),
        since_serial: Some(last_serial),
    })),
};
```

服务器侧约定：

- Tick 使用 `(ts, serial)` 精确续推；
- Quote、QuoteSnapshot 允许同一时间点幂等重推；
- Kline 按 `ts` 去重；
- 游标持久化、历史重放和重复消费处理由服务自身实现。

## 4. WebSocket 边界

- 所有协议 payload 都是 Binary Protobuf；
- heartbeat、resume 和行情消息都通过 `TransportFrame.oneof` 区分；
- 不调用 JSON parser；
- 不把 Protobuf 数字 enum 当成业务字符串；
- 不把 Binary codec 当作 ACK、outbox 或可靠消息系统。

本仓库不实现真实 WebSocket 生命周期、鉴权、订阅隔离、持久化或可靠重放。

## 5. Python generated module

```bash
uv sync --locked
uv run python scripts/generate_python.py
```

生成代码位于：

```text
python/market_protocol/v1/market_protocol_pb2.py
```

运行时只需要 `protobuf`，不需要安装 `protoc`。
