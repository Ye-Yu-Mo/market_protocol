//! 演示：解析真实样本 envelope JSON 为 `MarketMessage`。
//!
//! 运行：`cargo run --example from_json`

use market_protocol::message::MarketMessage;

fn main() {
    // A 股 560010 腾讯实测报价 envelope。
    let json = r#"{
        "type": "quote",
        "symbol": {"market": "A", "code": "560010"},
        "data": {
            "ts": 1787904896000,
            "price": 3.208,
            "open": 3.211,
            "high": 3.245,
            "low": 3.204,
            "pre_close": 3.220,
            "volume": 132886000.0,
            "amount": 428272326.0,
            "in_vol": 69090700.0,
            "out_vol": 63795300.0
        }
    }"#;

    let msg: MarketMessage =
        serde_json::from_str(json).expect("parse real A-share sample envelope");

    match &msg {
        MarketMessage::Quote { symbol, data } => {
            println!(
                "type=quote symbol={:?} ts={} price={} volume={}",
                symbol, data.ts, data.price, data.volume
            );
        }
        MarketMessage::Tick { symbol, data } => {
            println!(
                "type=tick symbol={:?} ts={} price={} volume={}",
                symbol, data.ts, data.price, data.volume
            );
        }
        MarketMessage::Kline { symbol, data } => {
            println!(
                "type=kline symbol={:?} ts={} period={:?} close={}",
                symbol, data.ts, data.period, data.close
            );
        }
        MarketMessage::QuoteSnapshot { symbol, data } => {
            println!(
                "type=quote_snapshot symbol={:?} ts={} bids={} asks={}",
                symbol,
                data.ts,
                data.bids.len(),
                data.asks.len()
            );
        }
    }
}
