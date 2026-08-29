//! 核心行情类型。
//!
//! 本模块是 `market_protocol` 在 M1 阶段的交付物：只定义数据结构，
//! 不定义任何线上协议线格式（envelope/frame 属于后续里程碑）。
//!
//! # 协议契约
//!
//! - **时间戳**：所有消息类型的时间戳字段统一为 epoch 毫秒 `i64`。
//!   各源（元大 `yyyy/MM/dd`、free-stockdb 8/14 位整数、FinMind `YYYY-MM-DD`）
//!   的时间格式一律在 spider 层转换为 epoch 毫秒，协议层不再出现时区/格式歧义。
//! - **量单位**：`volume` 统一以「股」为协议单位；`amount` 为「元」。
//!   腾讯“手”×100、元大期货“千股”等源单位由 spider 换算，协议层只有“股”。
//!
//! 消费方只需要依赖这两个契约，不需要再猜测字段含义。

use serde::{Deserialize, Serialize};

/// 最新成交快照（逐笔成交的最新聚合）。
///
/// 字段取自腾讯 / 元大 IndexFlag29 / 东财真实数据的交集。
/// `in_vol`/`out_vol` 为 `Option`：部分源不提供内外盘量。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    /// epoch 毫秒，业务时间戳
    pub ts: i64,
    /// 最新成交价
    pub price: f64,
    /// 今开
    pub open: f64,
    /// 今高
    pub high: f64,
    /// 今低
    pub low: f64,
    /// 昨收
    pub pre_close: f64,
    /// 累计成交量（股）
    pub volume: f64,
    /// 累计成交额（元）
    pub amount: f64,
    /// 内盘量（元大 TotalInVol / 腾讯；部分源无）
    pub in_vol: Option<f64>,
    /// 外盘量（元大 TotalOutVol / 腾讯）
    pub out_vol: Option<f64>,
}

/// 逐笔成交。
///
/// 字段对照元大 StockTick 真实数据。`serial` 是断线续推游标：
/// 单调递增序号；`-1` 表示清盘/恢复。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    /// epoch 毫秒
    pub ts: i64,
    /// 成交价（DealPrice）
    pub price: f64,
    /// 单笔量（股）
    pub volume: f64,
    /// 当时买一价（BuyPrice）
    pub buy_price: Option<f64>,
    /// 当时卖一价（SellPrice）
    pub sell_price: Option<f64>,
    /// 内外盘/揭示状态：0 内盘 / 1 外盘 / 2 定盘内 / 3 定盘外 /
    /// 10 一般揭示 / 11 暂缓撮合瞬间趋跌 / 12 暂缓撮合瞬间趋涨 /
    /// 13 试算后延后收盘 / 14 暂停交易 / 15 恢复交易
    pub inout_flag: Option<u8>,
    /// 单调递增序号，断线续推游标；-1 = 清盘/恢复
    pub serial: i64,
}

/// K 线周期。
///
/// 周期是有限集合，非法周期在编译期无法表达；序列化格式定死为
/// `1m`/`5m`/`15m`/`30m`/`60m`/`1d`/`1w`/`1M`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Period {
    #[serde(rename = "1m")]
    M1,
    #[serde(rename = "5m")]
    M5,
    #[serde(rename = "15m")]
    M15,
    #[serde(rename = "30m")]
    M30,
    #[serde(rename = "60m")]
    M60,
    #[serde(rename = "1d")]
    D1,
    #[serde(rename = "1w")]
    W1,
    #[serde(rename = "1M")]
    Month1,
}

/// K 线（OHLCV + period + 离线字段）。
///
/// 历史查询（M6 REST）与实时推送（WS）共用同一协议类型，因此必须承载
/// 离线源字段（`amount`/`turnover`/`pre_close`），回测直查离线库不得丢信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kline {
    /// epoch 毫秒，K 线开始时间
    pub ts: i64,
    /// K 线周期
    pub period: Period,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// 成交量（股）
    pub volume: f64,
    /// 成交额（元）
    pub amount: f64,
    /// 换手率 %（离线库有；部分源无）
    pub turnover: Option<f64>,
    /// 前收（离线库周/月聚合需要；部分源无）
    pub pre_close: Option<f64>,
}

/// 盘口一档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub price: f64,
    /// 股
    pub volume: f64,
}

/// 盘口快照（买一..买五/卖一..卖五，档数由源决定）。
///
/// 用 `Vec` 而非固定 `[Level; 5]`：元大有 5 档与 10 档两种，
/// 档数可变是正常情况，不是特殊情况。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuoteSnapshot {
    /// epoch 毫秒
    pub ts: i64,
    /// 买一..买五，从高到低
    pub bids: Vec<Level>,
    /// 卖一..卖五，从低到高
    pub asks: Vec<Level>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_derives_and_json_round_trip() {
        let quote = Quote {
            ts: 1_728_447_294_000,
            price: 3.208,
            open: 3.211,
            high: 3.245,
            low: 3.204,
            pre_close: 3.220,
            volume: 132_886_000.0,
            amount: 428_272_326.0,
            in_vol: None,
            out_vol: None,
        };

        let json = serde_json::to_string(&quote).unwrap();
        let back: Quote = serde_json::from_str(&json).unwrap();
        assert_eq!(quote, back);

        // Debug/Clone/PartialEq/Serialize/Deserialize 均由派生提供，这里显式触发编译期验证。
        let _ = format!("{quote:?}");
        let _ = quote.clone();
        assert_eq!(quote, quote);
    }

    #[test]
    fn tick_derives_and_json_round_trip() {
        let tick = Tick {
            ts: 1_728_447_294_000,
            price: 1420.0,
            volume: 2_000.0,
            buy_price: Some(1420.0),
            sell_price: Some(1425.0),
            inout_flag: Some(1),
            serial: 42,
        };

        let json = serde_json::to_string(&tick).unwrap();
        let back: Tick = serde_json::from_str(&json).unwrap();
        assert_eq!(tick, back);

        let _ = format!("{tick:?}");
        let _ = tick.clone();
        assert_eq!(tick, tick);
    }

    #[test]
    fn kline_derives_and_json_round_trip() {
        let kline = Kline {
            ts: 1_728_447_294_000,
            period: Period::D1,
            open: 32.3,
            high: 32.30,
            low: 31.70,
            close: 31.8,
            volume: 70_951.0,
            amount: 2_260_790.0,
            turnover: Some(74.0),
            pre_close: Some(32.0),
        };

        let json = serde_json::to_string(&kline).unwrap();
        let back: Kline = serde_json::from_str(&json).unwrap();
        assert_eq!(kline, back);

        let _ = format!("{kline:?}");
        let _ = kline.clone();
        assert_eq!(kline, kline);
    }

    #[test]
    fn quote_snapshot_derives_and_json_round_trip() {
        let snapshot = QuoteSnapshot {
            ts: 1_728_447_294_000,
            bids: vec![
                Level {
                    price: 3.207,
                    volume: 10_000.0,
                },
                Level {
                    price: 3.206,
                    volume: 20_000.0,
                },
            ],
            asks: vec![
                Level {
                    price: 3.208,
                    volume: 5_000.0,
                },
                Level {
                    price: 3.209,
                    volume: 8_000.0,
                },
            ],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let back: QuoteSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, back);

        let _ = format!("{snapshot:?}");
        let _ = snapshot.clone();
        assert_eq!(snapshot, snapshot);
    }

    #[test]
    fn period_serializes_as_compact_string_and_round_trips() {
        let cases = [
            (Period::M1, "1m"),
            (Period::M5, "5m"),
            (Period::M15, "15m"),
            (Period::M30, "30m"),
            (Period::M60, "60m"),
            (Period::D1, "1d"),
            (Period::W1, "1w"),
            (Period::Month1, "1M"),
        ];

        for (period, expected) in cases {
            assert_eq!(
                serde_json::to_string(&period).unwrap(),
                format!("\"{expected}\"")
            );
            let back: Period = serde_json::from_str(&format!("\"{expected}\"")).unwrap();
            assert_eq!(period, back);
        }
    }

    #[test]
    fn real_a_share_560010_quote_fixture() {
        let quote = Quote {
            ts: 1_787_904_896_000,
            price: 3.208,
            open: 3.211,
            high: 3.245,
            low: 3.204,
            pre_close: 3.220,
            volume: 132_886_000.0,
            amount: 428_272_326.0,
            in_vol: Some(69_090_700.0),
            out_vol: Some(63_795_300.0),
        };

        assert_eq!(quote.ts, 1_787_904_896_000);
        assert_eq!(quote.price, 3.208);
        assert_eq!(quote.open, 3.211);
        assert_eq!(quote.high, 3.245);
        assert_eq!(quote.low, 3.204);
        assert_eq!(quote.pre_close, 3.220);
        assert_eq!(quote.volume, 132_886_000.0);
        assert_eq!(quote.amount, 428_272_326.0);
        assert_eq!(quote.in_vol, Some(69_090_700.0));
        assert_eq!(quote.out_vol, Some(63_795_300.0));
    }

    #[test]
    fn real_tw_2330_quote_fixture() {
        let quote = Quote {
            ts: 1_787_887_504_611,
            price: 1420.0,
            open: 1415.0,
            high: 1430.0,
            low: 1410.0,
            pre_close: 1410.0,
            volume: 14_992.0,
            amount: 21_289_000.0,
            in_vol: Some(7_997.0),
            out_vol: Some(6_995.0),
        };

        assert_eq!(quote.price, 1420.0);
        assert_eq!(quote.volume, 14_992.0);
        assert_eq!(quote.in_vol, Some(7_997.0));
        assert_eq!(quote.out_vol, Some(6_995.0));
    }

    #[test]
    fn real_tw_1437_kline_fixture() {
        let kline = Kline {
            ts: 1_787_500_800_000,
            period: Period::D1,
            open: 32.3,
            high: 32.30,
            low: 31.70,
            close: 31.8,
            volume: 70_951.0,
            amount: 2_260_790.0,
            turnover: Some(74.0),
            pre_close: Some(32.0),
        };

        assert_eq!(kline.period, Period::D1);
        assert_eq!(kline.open, 32.3);
        assert_eq!(kline.high, 32.30);
        assert_eq!(kline.low, 31.70);
        assert_eq!(kline.close, 31.8);
        assert_eq!(kline.volume, 70_951.0);
        assert_eq!(kline.amount, 2_260_790.0);
        assert_eq!(kline.turnover, Some(74.0));
        assert_eq!(kline.pre_close, Some(32.0));
    }

    #[test]
    fn three_market_differences_and_boundaries() {
        // 缺失源字段用 None 表达，序列化后仍是 null，反序列化回 None。
        let quote = Quote {
            ts: 0,
            price: 1.234,
            open: 1.2,
            high: 1.3,
            low: 1.1,
            pre_close: 1.0,
            volume: 0.001,
            amount: 0.0,
            in_vol: None,
            out_vol: None,
        };
        let json = serde_json::to_string(&quote).unwrap();
        let back: Quote = serde_json::from_str(&json).unwrap();
        assert_eq!(quote, back);
        assert!(json.contains("\"in_vol\":null"));

        // Option 字段也可以整体缺失：serde 对缺失的 Option 字段按 None 处理。
        let quote_with_missing_optional: Quote = serde_json::from_str(
            r#"{
                "ts": 0,
                "price": 1.0,
                "open": 1.0,
                "high": 1.0,
                "low": 1.0,
                "pre_close": 1.0,
                "volume": 0.0,
                "amount": 0.0
            }"#,
        )
        .unwrap();
        assert_eq!(quote_with_missing_optional.in_vol, None);
        assert_eq!(quote_with_missing_optional.out_vol, None);

        // Kline 的 turnover/pre_close 同样是 Option。
        let kline = Kline {
            ts: 0,
            period: Period::M1,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 0.0,
            amount: 0.0,
            turnover: None,
            pre_close: None,
        };
        let json = serde_json::to_string(&kline).unwrap();
        let back: Kline = serde_json::from_str(&json).unwrap();
        assert_eq!(kline, back);
        assert!(json.contains("\"turnover\":null"));
        assert!(json.contains("\"pre_close\":null"));

        // 盘口档数可变：空 bids/asks 是合法状态。
        let empty = QuoteSnapshot {
            ts: 0,
            bids: vec![],
            asks: vec![],
        };
        let json = serde_json::to_string(&empty).unwrap();
        let back: QuoteSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(empty, back);

        // Tick 的 serial = -1 表达清盘/恢复语义。
        let clear = Tick {
            ts: 0,
            price: 0.0,
            volume: 0.0,
            buy_price: None,
            sell_price: None,
            inout_flag: None,
            serial: -1,
        };
        let json = serde_json::to_string(&clear).unwrap();
        let back: Tick = serde_json::from_str(&json).unwrap();
        assert_eq!(clear, back);

        // 未知周期字符串反序列化必须失败。
        let err = serde_json::from_str::<Period>("\"3m\"");
        assert!(err.is_err());
    }
}
