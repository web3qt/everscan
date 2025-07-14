use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tracing::{info, debug, warn};

use crate::clients::AltcoinSeasonIndex;

/// 缓存的市场数据
/// 
/// 包含加密货币的完整市场信息和技术指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMarketData {
    /// 币种ID
    pub coin_id: String,
    /// 币种名称
    pub name: String,
    /// 币种符号
    pub symbol: String,
    /// 当前价格（美元）
    pub current_price: f64,
    /// 24小时交易量
    pub volume_24h: Option<f64>,
    /// 24小时价格变化百分比
    pub price_change_24h: Option<f64>,
    /// 市值
    pub market_cap: Option<f64>,
    /// 技术指标
    pub technical_indicators: TechnicalIndicatorsData,
    /// 数据更新时间
    pub updated_at: DateTime<Utc>,
    /// 数据来源
    pub source: String,
}

/// 技术指标数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicatorsData {
    /// 布林带
    pub bollinger_bands: BollingerBandsData,
    /// RSI指标
    pub rsi: RSIData,
}

/// 布林带数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBandsData {
    /// 上轨
    pub upper: f64,
    /// 中轨（移动平均线）
    pub middle: f64,
    /// 下轨
    pub lower: f64,
    /// 计算周期
    pub period: u32,
    /// 标准差倍数
    pub std_dev_multiplier: f64,
}

/// RSI指标数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSIData {
    /// RSI值
    pub value: f64,
    /// 计算周期
    pub period: u32,
    /// 超买阈值
    pub overbought_threshold: f64,
    /// 超卖阈值
    pub oversold_threshold: f64,
    /// 信号状态
    pub signal: RSISignal,
}

/// RSI信号枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RSISignal {
    /// 正常
    Normal,
    /// 超买
    Overbought,
    /// 超卖
    Oversold,
}

/// 数据缓存管理器
/// 
/// 提供高效的读写操作和数据过期管理
pub struct DataCache {
    /// 市场数据缓存
    /// key: 币种ID, value: 缓存的市场数据
    market_data: RwLock<HashMap<String, CachedMarketData>>,
    /// 贪婪恐惧指数缓存
    fear_greed_index: RwLock<Option<serde_json::Value>>,
    /// 山寨币季节指数缓存
    altcoin_season_index: RwLock<Option<AltcoinSeasonIndex>>,
    /// 缓存统计信息
    stats: RwLock<CacheStats>,
}

/// 缓存统计信息
#[derive(Debug, Default, Serialize, Clone)] // 添加Clone trait
pub struct CacheStats {
    /// 总缓存项目数
    pub total_items: usize,
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 最后更新时间
    pub last_updated: Option<DateTime<Utc>>,
    /// 数据来源统计
    pub sources: HashMap<String, u64>,
}

impl DataCache {
    /// 创建新的数据缓存
    /// 
    /// # 返回
    /// * `Self` - 数据缓存实例
    pub fn new() -> Self {
        info!("💾 初始化数据缓存管理器");
        Self {
            market_data: RwLock::new(HashMap::new()),
            fear_greed_index: RwLock::new(None),
            altcoin_season_index: RwLock::new(None),
            stats: RwLock::new(CacheStats::default()),
        }
    }
    

    
    /// 获取市场数据
    /// 
    /// # 参数
    /// * `coin_id` - 币种ID
    /// 
    /// # 返回
    /// * `Option<CachedMarketData>` - 缓存的市场数据或None
    pub fn get_market_data(&self, coin_id: &str) -> Option<CachedMarketData> {
        let cache = self.market_data.read().unwrap();
        let result = cache.get(coin_id).cloned();
        
        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            if result.is_some() {
                stats.hits += 1;
            } else {
                stats.misses += 1;
            }
        }
        
        result
    }
    
    /// 获取所有市场数据
    /// 
    /// # 返回
    /// * `Vec<CachedMarketData>` - 所有缓存的市场数据
    pub fn get_all_market_data(&self) -> Vec<CachedMarketData> {
        let cache = self.market_data.read().unwrap();
        cache.values().cloned().collect()
    }
    
    /// 获取指定币种列表的市场数据
    /// 
    /// # 参数
    /// * `coin_ids` - 币种ID列表
    /// 
    /// # 返回
    /// * `HashMap<String, CachedMarketData>` - 币种ID到市场数据的映射
    pub fn get_multiple_market_data(&self, coin_ids: &[String]) -> HashMap<String, CachedMarketData> {
        let cache = self.market_data.read().unwrap();
        let mut result = HashMap::new();
        
        for coin_id in coin_ids {
            if let Some(data) = cache.get(coin_id) {
                result.insert(coin_id.clone(), data.clone());
            }
        }
        
        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.hits += result.len() as u64;
            stats.misses += (coin_ids.len() - result.len()) as u64;
        }
        
        result
    }
    
    /// 清理过期数据
    /// 
    /// # 参数
    /// * `max_age_hours` - 最大数据年龄（小时）
    /// 
    /// # 返回
    /// * `usize` - 清理的数据项数量
    pub fn cleanup_expired_data(&self, max_age_hours: i64) -> usize {
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut cache = self.market_data.write().unwrap();
        
        let initial_count = cache.len();
        cache.retain(|_, data| data.updated_at > cutoff_time);
        let removed_count = initial_count - cache.len();
        
        if removed_count > 0 {
            info!("🧹 清理了 {} 条过期数据 (超过 {} 小时)", removed_count, max_age_hours);
            
            // 更新统计信息
            let mut stats = self.stats.write().unwrap();
            stats.total_items = cache.len();
        }
        
        removed_count
    }
    
    /// 获取支持的币种列表
    /// 
    /// # 返回
    /// * `Vec<String>` - 币种ID列表
    pub fn get_supported_coins(&self) -> Vec<String> {
        let cache = self.market_data.read().unwrap();
        cache.keys().cloned().collect()
    }

    /// 获取缓存统计信息
    /// 
    /// # 返回
    /// * `CacheStats` - 缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }
    
    /// 清空所有缓存
    pub fn clear_all(&self) {
        let mut cache = self.market_data.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        let cleared_count = cache.len();
        cache.clear();
        *stats = CacheStats::default();
        
        warn!("🗑️ 已清空所有缓存数据 ({} 项)", cleared_count);
    }
    
    /// 获取缓存大小
    /// 
    /// # 返回
    /// * `usize` - 缓存中的数据项数量
    pub fn size(&self) -> usize {
        let cache = self.market_data.read().unwrap();
        cache.len()
    }
    
    /// 检查是否包含指定币种的数据
    /// 
    /// # 参数
    /// * `coin_id` - 币种ID
    /// 
    /// # 返回
    /// * `bool` - 是否包含数据
    pub fn contains(&self, coin_id: &str) -> bool {
        let cache = self.market_data.read().unwrap();
        cache.contains_key(coin_id)
    }

    /// 设置贪婪恐惧指数数据
    /// 
    /// # 参数
    /// * `data` - 贪婪恐惧指数数据
    pub async fn set_fear_greed_index(&self, data: serde_json::Value) {
        debug!("💾 更新贪婪恐惧指数缓存");
        
        {
            let mut cache = self.fear_greed_index.write().unwrap();
            *cache = Some(data);
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.last_updated = Some(Utc::now());
            *stats.sources.entry("CoinMarketCap".to_string()).or_insert(0) += 1;
        }

        info!("✅ 贪婪恐惧指数缓存已更新");
    }

    /// 获取贪婪恐惧指数数据
    /// 
    /// # 返回
    /// * `Option<serde_json::Value>` - 贪婪恐惧指数数据
    pub fn get_fear_greed_index(&self) -> Option<serde_json::Value> {
        debug!("📖 读取贪婪恐惧指数缓存");
        
        let cache = self.fear_greed_index.read().unwrap();
        
        if cache.is_some() {
            // 更新命中统计
            {
                let mut stats = self.stats.write().unwrap();
                stats.hits += 1;
            }
            debug!("✅ 贪婪恐惧指数缓存命中");
        } else {
            // 更新未命中统计
            {
                let mut stats = self.stats.write().unwrap();
                stats.misses += 1;
            }
            debug!("❌ 贪婪恐惧指数缓存未命中");
        }
        
        cache.clone()
    }

    /// 设置山寨币季节指数数据
    /// 
    /// # 参数
    /// * `data` - 山寨币季节指数数据（JSON格式）
    pub async fn set_altcoin_season_index(&self, data: serde_json::Value) {
        debug!("💾 更新山寨币季节指数缓存");
        
        {
            let mut cache = self.altcoin_season_index.write().unwrap();
            // 尝试解析为AltcoinSeasonIndex，如果失败就存储JSON
            if let Ok(parsed_data) = serde_json::from_value::<AltcoinSeasonIndex>(data.clone()) {
                *cache = Some(parsed_data);
            } else {
                // 对于模拟数据，我们需要创建一个AltcoinSeasonIndex结构
                if let (Some(value), Some(classification), Some(classification_zh), Some(timestamp), Some(advice)) = (
                    data.get("value").and_then(|v| v.as_u64()).map(|v| v as u8),
                    data.get("classification").and_then(|v| v.as_str()),
                    data.get("classification_zh").and_then(|v| v.as_str()),
                    data.get("timestamp").and_then(|v| v.as_str()),
                    data.get("market_advice").and_then(|v| v.as_str()),
                ) {
                    let altcoin_data = AltcoinSeasonIndex {
                        value,
                        classification: classification.to_string(),
                        classification_zh: classification_zh.to_string(),
                        timestamp: timestamp.to_string(),
                        outperforming_count: data.get("outperforming_count").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                        total_count: data.get("total_count").and_then(|v| v.as_u64()).unwrap_or(100) as u8,
                        outperforming_percentage: data.get("outperforming_percentage").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        market_advice: advice.to_string(),
                    };
                    *cache = Some(altcoin_data);
                }
            }
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.last_updated = Some(Utc::now());
            *stats.sources.entry("CoinMarketCap".to_string()).or_insert(0) += 1;
        }

        info!("✅ 山寨币季节指数缓存已更新");
    }

    /// 获取山寨币季节指数数据
    /// 
    /// # 返回
    /// * `Option<AltcoinSeasonIndex>` - 山寨币季节指数数据
    pub fn get_altcoin_season_index(&self) -> Option<AltcoinSeasonIndex> {
        debug!("📖 读取山寨币季节指数缓存");
        
        let cache = self.altcoin_season_index.read().unwrap();
        
        if cache.is_some() {
            // 更新命中统计
            {
                let mut stats = self.stats.write().unwrap();
                stats.hits += 1;
            }
            debug!("✅ 山寨币季节指数缓存命中");
        } else {
            // 更新未命中统计
            {
                let mut stats = self.stats.write().unwrap();
                stats.misses += 1;
            }
            debug!("❌ 山寨币季节指数缓存未命中");
        }
        
        cache.clone()
    }

    /// 设置币种数据（简化版本）
    /// 
    /// # 参数
    /// * `coin_id` - 币种ID
    /// * `data` - 币种数据（JSON格式）
    pub async fn set_coin_data(&self, coin_id: &str, data: serde_json::Value) {
        debug!("💾 更新币种数据缓存: {}", coin_id);
        
        // 创建简化的缓存数据
        if let (Some(current_price), Some(symbol), Some(name)) = (
            data.get("current_price").and_then(|v| v.as_f64()),
            data.get("symbol").and_then(|v| v.as_str()),
            data.get("name").and_then(|v| v.as_str()),
        ) {
            let cached_data = CachedMarketData {
                coin_id: coin_id.to_string(),
                name: name.to_string(),
                symbol: symbol.to_string(),
                current_price,
                volume_24h: data.get("total_volume").and_then(|v| v.as_f64()),
                price_change_24h: data.get("price_change_percentage_24h").and_then(|v| v.as_f64()),
                market_cap: data.get("market_cap").and_then(|v| v.as_f64()),
                technical_indicators: TechnicalIndicatorsData {
                    bollinger_bands: BollingerBandsData {
                        upper: current_price * 1.02, // 模拟数据
                        middle: current_price,
                        lower: current_price * 0.98,
                        period: 20,
                        std_dev_multiplier: 2.0,
                    },
                    rsi: RSIData {
                        value: 50.0, // 模拟中性RSI
                        period: 14,
                        overbought_threshold: 70.0,
                        oversold_threshold: 30.0,
                        signal: RSISignal::Normal,
                    },
                },
                updated_at: Utc::now(),
                source: if data.get("mock_data").is_some() { "Mock" } else { "CoinGecko" }.to_string(),
            };

            {
                let mut cache = self.market_data.write().unwrap();
                cache.insert(coin_id.to_string(), cached_data);
            }

            // 更新统计信息
            {
                let mut stats = self.stats.write().unwrap();
                stats.last_updated = Some(Utc::now());
                stats.total_items = self.market_data.read().unwrap().len();
                let source = if data.get("mock_data").is_some() { "Mock" } else { "CoinGecko" };
                *stats.sources.entry(source.to_string()).or_insert(0) += 1;
            }

            info!("✅ 币种数据缓存已更新: {}", coin_id);
        } else {
            warn!("⚠️ 无法解析币种数据: {}", coin_id);
        }
    }
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
} 