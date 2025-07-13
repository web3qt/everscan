use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tracing::{info, debug, warn};

use crate::clients::EnhancedMarketData;

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

/// RSI数据
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

/// RSI信号类型
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
/// 负责管理内存中的加密货币市场数据
/// 提供高效的读写操作和数据过期管理
pub struct DataCache {
    /// 市场数据缓存
    /// key: 币种ID, value: 缓存的市场数据
    market_data: RwLock<HashMap<String, CachedMarketData>>,
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
            stats: RwLock::new(CacheStats::default()),
        }
    }
    
    /// 更新市场数据
    /// 
    /// # 参数
    /// * `coin_id` - 币种ID
    /// * `market_data` - 增强的市场数据
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub fn update_market_data(&self, coin_id: &str, market_data: &EnhancedMarketData) -> Result<()> {
        let cached_data = CachedMarketData {
            coin_id: coin_id.to_string(),
            name: market_data.coin_price.name.clone(),
            symbol: market_data.coin_price.symbol.clone(),
            current_price: market_data.coin_price.current_price,
            volume_24h: market_data.coin_price.total_volume,
            price_change_24h: market_data.coin_price.price_change_percentage_24h,
            market_cap: market_data.coin_price.market_cap,
            technical_indicators: TechnicalIndicatorsData {
                bollinger_bands: BollingerBandsData {
                    upper: market_data.technical_indicators.bollinger_bands.upper,
                    middle: market_data.technical_indicators.bollinger_bands.middle,
                    lower: market_data.technical_indicators.bollinger_bands.lower,
                    period: market_data.technical_indicators.bollinger_bands.period,
                    std_dev_multiplier: market_data.technical_indicators.bollinger_bands.std_dev_multiplier,
                },
                rsi: RSIData {
                    value: market_data.technical_indicators.rsi.value,
                    period: market_data.technical_indicators.rsi.period,
                    overbought_threshold: market_data.technical_indicators.rsi.overbought_threshold,
                    oversold_threshold: market_data.technical_indicators.rsi.oversold_threshold,
                    signal: if market_data.technical_indicators.rsi.value >= market_data.technical_indicators.rsi.overbought_threshold {
                        RSISignal::Overbought
                    } else if market_data.technical_indicators.rsi.value <= market_data.technical_indicators.rsi.oversold_threshold {
                        RSISignal::Oversold
                    } else {
                        RSISignal::Normal
                    },
                },
            },
            updated_at: Utc::now(),
            source: "CoinGecko".to_string(),
        };
        
        // 更新缓存
        {
            let mut cache = self.market_data.write().unwrap();
            cache.insert(coin_id.to_string(), cached_data);
        }
        
        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_items = {
                let cache = self.market_data.read().unwrap();
                cache.len()
            };
            stats.last_updated = Some(Utc::now());
            *stats.sources.entry("CoinGecko".to_string()).or_insert(0) += 1;
        }
        
        debug!("💾 已更新 {} 的市场数据缓存", coin_id);
        Ok(())
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
}

impl Default for DataCache {
    fn default() -> Self {
        Self::new()
    }
} 