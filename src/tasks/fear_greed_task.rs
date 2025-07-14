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

/// 贪婪恐惧指数任务
pub struct FearGreedTask {
    /// 任务名称
    name: String,
    /// CoinMarketCap客户端
    client: Arc<CoinMarketCapClient>,
    /// 任务执行间隔（秒）
    interval_seconds: u64,
}

impl FearGreedTask {
    /// 创建新的贪婪恐惧指数任务
    pub fn new(
        name: String,
        client: Arc<CoinMarketCapClient>,
        interval_seconds: u64,
    ) -> Self {
        info!("🚀 创建贪婪恐惧指数任务: {}", name);
        info!("⏰ 执行间隔: {}s", interval_seconds);
        
        Self {
            name,
            client,
            interval_seconds,
        }
    }
    
    /// 收集贪婪恐惧指数数据
    async fn collect_fear_greed_data(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>> {
        info!("📊 开始收集贪婪恐惧指数数据");
        
        // 获取真实的贪婪恐惧指数数据
        match self.client.get_fear_greed_index().await {
            Ok(fear_greed_data) => {
                info!("✅ 贪婪恐惧指数获取成功: {} - {}", fear_greed_data.value, fear_greed_data.value_classification);
                
                // 获取中文分类和投资建议
                let chinese_classification = CoinMarketCapClient::get_chinese_classification(&fear_greed_data.value_classification);
                let sentiment_description = CoinMarketCapClient::get_sentiment_description(fear_greed_data.value);
                let investment_advice = CoinMarketCapClient::get_investment_advice(fear_greed_data.value);
                
                // 缓存数据
                let cached_data = serde_json::json!({
                    "value": fear_greed_data.value,
                    "value_classification": fear_greed_data.value_classification,
                    "value_classification_zh": chinese_classification,
                    "sentiment_description": sentiment_description,
                    "investment_advice": investment_advice,
                    "timestamp": fear_greed_data.timestamp,
                    "time_until_update": fear_greed_data.time_until_update
                });
                cache.set_fear_greed_index(cached_data).await;
                
                // 转换为指标格式
                let mut metrics = Vec::new();
                let timestamp = Utc::now();
                
                // 贪婪恐惧指数值
                metrics.push(MetricBuilder::new(
                    DataSource::CoinMarketCap,
                    "fear_greed_index".to_string()
                )
                .value(serde_json::json!(fear_greed_data.value))
                .timestamp(timestamp)
                .metadata(serde_json::json!({
                    "classification": fear_greed_data.value_classification,
                    "classification_zh": chinese_classification,
                    "sentiment_description": sentiment_description,
                    "investment_advice": investment_advice,
                    "time_until_update": fear_greed_data.time_until_update
                }))
                .build());
                
                info!("📦 贪婪恐惧指数数据已缓存");
                info!("🎯 贪婪恐惧指数: {} - {} ({})", fear_greed_data.value, chinese_classification, investment_advice);
                
                Ok(metrics)
            }
            Err(e) => {
                error!("❌ 获取贪婪恐惧指数失败: {}", e);
                Err(e)
            }
        }
    }
}

#[async_trait]
impl Task for FearGreedTask {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "收集加密货币市场贪婪恐惧指数，分析市场情绪状态"
    }
    
    fn id(&self) -> &str {
        "fear_greed"
    }
    
    fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }
    
    async fn execute(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行贪婪恐惧指数任务: {}", self.name);
        
        match self.collect_fear_greed_data(cache).await {
            Ok(metrics) => {
                info!("✅ 贪婪恐惧指数数据收集完成，共 {} 条指标", metrics.len());
                Ok(metrics)
            }
            Err(e) => {
                error!("❌ 贪婪恐惧指数任务执行失败: {}", e);
                Err(e)
            }
        }
    }
}

/// 贪婪恐惧指数任务构建器
pub struct FearGreedTaskBuilder {
    client: Option<Arc<CoinMarketCapClient>>,
    interval_seconds: Option<u64>,
    name: Option<String>,
}

impl FearGreedTaskBuilder {
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
    pub fn build(self) -> Result<FearGreedTask> {
        let client = self.client.ok_or_else(|| anyhow::anyhow!("缺少CoinMarketCap客户端"))?;
        let interval_seconds = self.interval_seconds.unwrap_or(3600); // 默认1小时
        let name = self.name.unwrap_or_else(|| "贪婪恐惧指数采集".to_string());
        
        Ok(FearGreedTask::new(name, client, interval_seconds))
    }
}

impl Default for FearGreedTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
} 