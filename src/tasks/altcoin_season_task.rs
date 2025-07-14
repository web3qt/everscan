use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, error, warn};
use chrono::Utc;

use crate::clients::CoinMarketCapClient;
use crate::models::{AggregatedMetric, MetricBuilder, DataSource};
use crate::tasks::Task;
use crate::web::cache::DataCache;

/// 山寨币季节指数任务
pub struct AltcoinSeasonTask {
    /// 任务名称
    name: String,
    /// CoinMarketCap客户端
    client: Arc<CoinMarketCapClient>,
    /// 任务执行间隔（秒）
    interval_seconds: u64,
}

impl AltcoinSeasonTask {
    /// 创建新的山寨币季节指数任务
    pub fn new(
        name: String,
        client: Arc<CoinMarketCapClient>,
        interval_seconds: u64,
    ) -> Self {
        info!("🚀 创建山寨币季节指数任务: {}", name);
        info!("⏰ 执行间隔: {}s", interval_seconds);
        
        Self {
            name,
            client,
            interval_seconds,
        }
    }
    
    /// 收集山寨币季节指数数据
    async fn collect_altcoin_season_data(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>> {
        info!("📊 开始收集山寨币季节指数数据");
        
        // 获取真实的山寨币季节指数数据
        match self.client.get_altcoin_season_index().await {
            Ok(altcoin_data) => {
                info!("✅ 山寨币季节指数获取成功: {} - {}", altcoin_data.value, altcoin_data.classification_zh);
                
                // 缓存数据
                let json_data = serde_json::json!({
                    "value": altcoin_data.value,
                    "classification": altcoin_data.classification,
                    "classification_zh": altcoin_data.classification_zh,
                    "timestamp": altcoin_data.timestamp,
                    "outperforming_count": altcoin_data.outperforming_count,
                    "total_count": altcoin_data.total_count,
                    "outperforming_percentage": altcoin_data.outperforming_percentage,
                    "market_advice": altcoin_data.market_advice
                });
                cache.set_altcoin_season_index(json_data).await;
                
                // 转换为指标格式
                let mut metrics = Vec::new();
                let timestamp = Utc::now();
                
                // 山寨币季节指数值
                metrics.push(MetricBuilder::new(
                    DataSource::CoinMarketCap,
                    "altcoin_season_index".to_string()
                )
                .value(serde_json::json!(altcoin_data.value))
                .timestamp(timestamp)
                .metadata(serde_json::json!({
                    "classification": altcoin_data.classification,
                    "classification_zh": altcoin_data.classification_zh,
                    "outperforming_count": altcoin_data.outperforming_count,
                    "total_count": altcoin_data.total_count,
                    "outperforming_percentage": altcoin_data.outperforming_percentage,
                    "market_advice": altcoin_data.market_advice
                }))
                .build());
                
                info!("📦 山寨币季节指数数据已缓存");
                info!("🎯 山寨币季节指数: {} - {} ({})", altcoin_data.value, altcoin_data.classification_zh, altcoin_data.market_advice);
                
                Ok(metrics)
            }
            Err(e) => {
                error!("❌ 获取山寨币季节指数失败: {}", e);
                Err(e)
            }
        }
    }
}

#[async_trait]
impl Task for AltcoinSeasonTask {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "收集山寨币季节指数数据，基于CMC 100指数分析山寨币相对于比特币的表现"
    }
    
    fn id(&self) -> &str {
        "altcoin_season"
    }
    
    fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }
    
    async fn execute(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行山寨币季节指数任务: {}", self.name);
        
        match self.collect_altcoin_season_data(cache).await {
            Ok(metrics) => {
                info!("✅ 山寨币季节指数数据收集完成，共 {} 条指标", metrics.len());
                Ok(metrics)
            }
            Err(e) => {
                error!("❌ 山寨币季节指数任务执行失败: {}", e);
                Err(e)
            }
        }
    }
}

/// 山寨币季节指数任务构建器
pub struct AltcoinSeasonTaskBuilder {
    client: Option<Arc<CoinMarketCapClient>>,
    interval_seconds: Option<u64>,
    name: Option<String>,
}

impl AltcoinSeasonTaskBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            client: None,
            interval_seconds: None,
            name: None,
        }
    }
    
    /// 设置CoinMarketCap客户端
    pub fn client(mut self, client: Arc<CoinMarketCapClient>) -> Self {
        self.client = Some(client);
        self
    }
    
    /// 设置任务执行间隔
    pub fn interval_seconds(mut self, seconds: u64) -> Self {
        self.interval_seconds = Some(seconds);
        self
    }
    
    /// 设置任务名称
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    /// 构建任务
    pub fn build(self) -> Result<AltcoinSeasonTask> {
        let client = self.client.ok_or_else(|| anyhow::anyhow!("缺少CoinMarketCap客户端"))?;
        let interval_seconds = self.interval_seconds.unwrap_or(3600); // 默认1小时
        let name = self.name.unwrap_or_else(|| "山寨币季节指数采集".to_string());
        
        Ok(AltcoinSeasonTask::new(name, client, interval_seconds))
    }
}

impl Default for AltcoinSeasonTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
} 