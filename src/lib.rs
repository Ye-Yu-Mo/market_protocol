//! `market_protocol` —— 行情数据在管道间传输的 wire protocol。
//!
//! 定位：`spider → market_webserver → factor_maker / frontend` 之间，
//! 行情消息的统一类型、统一序列化/反序列化与传输帧约定。
//!
//! 设计决策与分阶段计划见同目录 [`PLAN.md`](./PLAN.md)。
//!
//! # 协议契约
//!
//! - **时间戳**：所有行情消息的 `ts` 一律为 epoch 毫秒 `i64`。
//! - **量单位**：所有行情消息的 `volume` 一律以「股」为协议单位。
//!
//! 详细说明见 [`types`](crate::types) 模块文档。

pub mod frame;
pub mod message;
// `trait` 是关键字，模块文件仍是 `src/trait.rs`。
pub mod r#trait;
pub mod types;

pub use r#trait::Message;

use serde::{Deserialize, Serialize};

/// 市场。
///
/// `A` = A 股，`Tw` = 台股，`Crypto` = 加密货币。
/// crypto 行情类型本阶段只预留，spider 实现延后到系统稳定后（根 PLAN M11）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Market {
    A,
    Tw,
    Crypto,
}

/// 标的代码。
///
/// 三市场差异（A 股整手 100、台股繁体编码、crypto 24/7 小数份额）
/// 由消息类型在字段层面表达，这里只做稳定的市场 + 代码标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub market: Market,
    pub code: String,
}

impl Symbol {
    pub fn new(market: Market, code: impl Into<String>) -> Self {
        Self {
            market,
            code: code.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_json_round_trip() {
        let s = Symbol::new(Market::A, "560010");
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("560010"));
        let back: Symbol = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn market_serializes_as_plain_string() {
        assert_eq!(serde_json::to_string(&Market::Tw).unwrap(), r#""Tw""#);
    }
}
