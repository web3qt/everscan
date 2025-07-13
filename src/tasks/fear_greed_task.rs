use anyhow::{Result, Context};
use async_trait::async_trait;
use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
use std::time::Duration;
use tracing::{info, debug, error};
use chrono::Utc;

use crate::clients::{CoinMarketCapClient, FearGreedIndex};
use crate::storage::PostgresRepository;
use crate::models::{AggregatedMetric, MetricBuilder, DataSource};
use crate::web::cache::DataCache;
use super::{Task, TaskStatus};

/// 贪婪恐惧指数任务
/// 
/// 负责定期获取市场贪婪恐惧指数数据
/// 提供市场情绪分析和投资建议
pub struct FearGreedTask {
    /// 任务名称
    name: String,
    /// CoinMarketCap客户端
    client: Arc<CoinMarketCapClient>,
    /// 任务执行间隔
    interval: Duration,
    /// 任务状态
    status: AtomicU8,
    /// 数据缓存
    cache: Option<Arc<DataCache>>,
}

impl FearGreedTask {
    /// 创建新的贪婪恐惧指数任务
    /// 
    /// # 参数
    /// * `name` - 任务名称
    /// * `client` - CoinMarketCap客户端
    /// * `interval` - 执行间隔
    /// * `cache` - 数据缓存（可选）
    /// 
    /// # 返回
    /// * `Self` - 任务实例
    pub fn new(
        name: String,
        client: Arc<CoinMarketCapClient>,
        interval: Duration,
        cache: Option<Arc<DataCache>>,
    ) -> Self {
        Self {
            name,
            client,
            interval,
            status: AtomicU8::new(TaskStatus::Idle as u8),
            cache,
        }
    }

    /// 设置数据缓存
    /// 
    /// # 参数
    /// * `cache` - 数据缓存
    /// 
    /// # 返回
    /// * `Self` - 更新后的任务实例
    pub fn with_cache(mut self, cache: Arc<DataCache>) -> Self {
        self.cache = Some(cache);
        self
    }
}

#[async_trait]
impl Task for FearGreedTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "获取市场贪婪恐惧指数，分析市场情绪"
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn execute(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行贪婪恐惧指数任务: {}", self.name);
        self.set_status(TaskStatus::Running);

        let start_time = std::time::Instant::now();
        let mut metrics = Vec::new();

        match self.collect_fear_greed_data(storage).await {
            Ok(mut task_metrics) => {
                metrics.append(&mut task_metrics);
                self.set_status(TaskStatus::Completed);
                
                let execution_time = start_time.elapsed();
                info!("✅ 贪婪恐惧指数任务完成，耗时: {:?}, 获取 {} 条指标", 
                      execution_time, metrics.len());
            }
            Err(e) => {
                error!("❌ 贪婪恐惧指数任务执行失败: {}", e);
                self.set_status(TaskStatus::Failed);
                return Err(e);
            }
        }

        Ok(metrics)
    }

    async fn health_check(&self) -> Result<bool> {
        debug!("🏥 执行贪婪恐惧指数任务健康检查");
        
        // 检查客户端连接
        self.client.health_check().await
    }

    fn status(&self) -> TaskStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => TaskStatus::Idle,
            1 => TaskStatus::Running,
            2 => TaskStatus::Completed,
            3 => TaskStatus::Failed,
            4 => TaskStatus::Disabled,
            _ => TaskStatus::Idle,
        }
    }

    fn set_status(&self, status: TaskStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

impl FearGreedTask {
    /// 收集贪婪恐惧指数数据
    /// 
    /// # 参数
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<Vec<AggregatedMetric>>` - 指标数据或错误
    async fn collect_fear_greed_data(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>> {
        info!("📊 开始收集贪婪恐惧指数数据");

        // 获取贪婪恐惧指数
        let fear_greed_data = self.client.get_fear_greed_index().await
            .context("获取贪婪恐惧指数失败")?;

        // 记录详细信息
        let chinese_classification = CoinMarketCapClient::get_chinese_classification(&fear_greed_data.value_classification);
        let _sentiment_description = CoinMarketCapClient::get_sentiment_description(fear_greed_data.value);
        let investment_advice = CoinMarketCapClient::get_investment_advice(fear_greed_data.value);

        info!("📈 贪婪恐惧指数: {} - {} ({})", 
              fear_greed_data.value, 
              chinese_classification,
              fear_greed_data.value_classification);
        info!("💡 投资建议: {}", investment_advice);

        // 转换为指标数据
        let metrics = self.convert_to_metrics(&fear_greed_data)?;

        // 存储到缓存
        if let Some(cache) = &self.cache {
            self.store_to_cache(&fear_greed_data, cache).await?;
        }

        // 存储到数据库（如果需要）
        self.store_fear_greed_data(&fear_greed_data, storage).await?;

        info!("✅ 贪婪恐惧指数数据收集完成，生成 {} 条指标", metrics.len());
        Ok(metrics)
    }

    /// 将贪婪恐惧指数数据转换为指标
    /// 
    /// # 参数
    /// * `fear_greed_data` - 贪婪恐惧指数数据
    /// 
    /// # 返回
    /// * `Result<Vec<AggregatedMetric>>` - 指标列表或错误
    fn convert_to_metrics(&self, fear_greed_data: &FearGreedIndex) -> Result<Vec<AggregatedMetric>> {
        let mut metrics = Vec::new();
        let timestamp = Utc::now();

        // 贪婪恐惧指数值指标
        let fear_greed_metric = MetricBuilder::new(DataSource::CoinMarketCap, "fear_greed_index")
            .value(serde_json::json!(fear_greed_data.value))
            .timestamp(timestamp)
            .metadata(serde_json::json!({
                "classification": fear_greed_data.value_classification,
                "chinese_classification": CoinMarketCapClient::get_chinese_classification(&fear_greed_data.value_classification),
                "sentiment_description": CoinMarketCapClient::get_sentiment_description(fear_greed_data.value),
                "investment_advice": CoinMarketCapClient::get_investment_advice(fear_greed_data.value),
                "timestamp": fear_greed_data.timestamp,
                "time_until_update": fear_greed_data.time_until_update
            }))
            .build();

        metrics.push(fear_greed_metric);

        // 情绪分类指标（数值化）
        let sentiment_value = match fear_greed_data.value {
            0..=24 => 1.0,   // 极度恐惧
            25..=44 => 2.0,  // 恐惧
            45..=55 => 3.0,  // 中性
            56..=75 => 4.0,  // 贪婪
            76..=100 => 5.0, // 极度贪婪
            _ => 3.0,        // 默认为中性
        };

        let sentiment_metric = MetricBuilder::new(DataSource::CoinMarketCap, "market_sentiment")
            .value(serde_json::json!(sentiment_value))
            .timestamp(timestamp)
            .metadata(serde_json::json!({
                "category": CoinMarketCapClient::get_sentiment_description(fear_greed_data.value),
                "raw_value": fear_greed_data.value
            }))
            .build();

        metrics.push(sentiment_metric);

        Ok(metrics)
    }

    /// 存储贪婪恐惧指数数据到缓存
    /// 
    /// # 参数
    /// * `fear_greed_data` - 贪婪恐惧指数数据
    /// * `cache` - 数据缓存
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn store_to_cache(&self, fear_greed_data: &FearGreedIndex, cache: &Arc<DataCache>) -> Result<()> {
        debug!("💾 存储贪婪恐惧指数数据到缓存");

        // 创建缓存数据结构
        let cache_data = serde_json::json!({
            "value": fear_greed_data.value,
            "classification": fear_greed_data.value_classification,
            "chinese_classification": CoinMarketCapClient::get_chinese_classification(&fear_greed_data.value_classification),
            "sentiment_description": CoinMarketCapClient::get_sentiment_description(fear_greed_data.value),
            "investment_advice": CoinMarketCapClient::get_investment_advice(fear_greed_data.value),
            "timestamp": fear_greed_data.timestamp,
            "time_until_update": fear_greed_data.time_until_update,
            "updated_at": Utc::now().to_rfc3339()
        });

        // 存储到缓存（使用特殊的键名）
        cache.set_fear_greed_index(cache_data).await;

        debug!("✅ 贪婪恐惧指数数据已存储到缓存");
        Ok(())
    }

    /// 存储贪婪恐惧指数数据到数据库
    /// 
    /// # 参数
    /// * `fear_greed_data` - 贪婪恐惧指数数据
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn store_fear_greed_data(&self, _fear_greed_data: &FearGreedIndex, _storage: &PostgresRepository) -> Result<()> {
        // 这里可以实现数据库存储逻辑
        // 由于当前主要使用内存缓存，暂时不实现数据库存储
        debug!("📝 贪婪恐惧指数数据存储到数据库（暂未实现）");
        Ok(())
    }
}

/// 贪婪恐惧指数任务构建器
pub struct FearGreedTaskBuilder {
    client: Option<Arc<CoinMarketCapClient>>,
    interval: Option<Duration>,
    name: Option<String>,
    cache: Option<Arc<DataCache>>,
}

impl FearGreedTaskBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            client: None,
            interval: None,
            name: None,
            cache: None,
        }
    }

    /// 设置客户端
    pub fn client(mut self, client: Arc<CoinMarketCapClient>) -> Self {
        self.client = Some(client);
        self
    }

    /// 设置执行间隔
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// 设置任务名称
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// 设置数据缓存
    pub fn cache(mut self, cache: Arc<DataCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// 构建任务
    pub fn build(self) -> Result<FearGreedTask> {
        let client = self.client.ok_or_else(|| anyhow::anyhow!("缺少CoinMarketCap客户端"))?;
        let interval = self.interval.unwrap_or_else(|| Duration::from_secs(3600)); // 默认1小时
        let name = self.name.unwrap_or_else(|| "贪婪恐惧指数任务".to_string());

        Ok(FearGreedTask::new(name, client, interval, self.cache))
    }
}

impl Default for FearGreedTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
} 