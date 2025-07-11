use anyhow::{Result, Context};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use tracing::{info, debug, error};
use chrono::Utc;

use crate::clients::CoinGeckoClient;
use crate::models::{AggregatedMetric, MetricBuilder, DataSource};
use crate::storage::PostgresRepository;
use super::{Task, TaskStatus};

/// CoinGecko数据获取任务
/// 
/// 负责从CoinGecko API获取代币价格、市值等市场数据
pub struct CoinGeckoTask {
    /// CoinGecko客户端
    client: Arc<CoinGeckoClient>,
    /// 任务状态
    status: AtomicU8,
    /// 执行间隔
    interval: Duration,
    /// 要获取的代币列表
    coin_ids: Vec<String>,
}

impl CoinGeckoTask {
    /// 创建新的CoinGecko任务
    /// 
    /// # 参数
    /// * `client` - CoinGecko客户端
    /// * `interval` - 执行间隔
    /// * `coin_ids` - 要获取的代币ID列表
    /// 
    /// # 返回
    /// * `Self` - 创建的任务
    pub fn new(client: Arc<CoinGeckoClient>, interval: Duration, coin_ids: Vec<String>) -> Self {
        Self {
            client,
            status: AtomicU8::new(TaskStatus::Idle as u8),
            interval,
            coin_ids,
        }
    }
    
    /// 获取热门代币价格
    async fn fetch_trending_prices(&self) -> Result<Vec<AggregatedMetric>> {
        debug!("🔥 正在获取热门代币价格");
        
        // 获取热门代币列表
        let trending_ids = self.client.get_trending_coins().await
            .context("获取热门代币列表失败")?;
        
        // 获取价格信息
        let prices = self.client.get_coin_prices(&trending_ids, "usd").await
            .context("获取热门代币价格失败")?;
        
        let mut metrics = Vec::new();
        
        for price in prices {
            let metric = MetricBuilder::new(DataSource::CoinGecko, "trending_coin_price")
                .value(serde_json::to_value(&price)?)
                .timestamp(Utc::now())
                .build();
            
            metrics.push(metric);
        }
        
        info!("✅ 获取到 {} 个热门代币价格", metrics.len());
        
        Ok(metrics)
    }
    
    /// 获取配置的代币价格
    async fn fetch_configured_prices(&self) -> Result<Vec<AggregatedMetric>> {
        if self.coin_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        debug!("💰 正在获取配置的代币价格: {:?}", self.coin_ids);
        
        // 获取价格信息
        let prices = self.client.get_coin_prices(&self.coin_ids, "usd").await
            .context("获取配置代币价格失败")?;
        
        let mut metrics = Vec::new();
        
        for price in prices {
            let metric = MetricBuilder::new(DataSource::CoinGecko, "coin_price")
                .value(serde_json::to_value(&price)?)
                .timestamp(Utc::now())
                .build();
            
            metrics.push(metric);
        }
        
        info!("✅ 获取到 {} 个配置代币价格", metrics.len());
        
        Ok(metrics)
    }
    
    /// 获取全球市场数据
    async fn fetch_global_data(&self) -> Result<Vec<AggregatedMetric>> {
        debug!("🌍 正在获取全球市场数据");
        
        let global_data = self.client.get_global_data().await
            .context("获取全球市场数据失败")?;
        
        let metric = MetricBuilder::new(DataSource::CoinGecko, "global_market_data")
            .value(serde_json::to_value(&global_data)?)
            .timestamp(Utc::now())
            .build();
        
        info!("✅ 获取全球市场数据成功");
        
        Ok(vec![metric])
    }
}

#[async_trait]
impl Task for CoinGeckoTask {
    fn name(&self) -> &str {
        "coingecko"
    }
    
    fn description(&self) -> &str {
        "获取CoinGecko市场数据，包括代币价格、市值和全球市场统计"
    }
    
    fn interval(&self) -> Duration {
        self.interval
    }
    
    async fn execute(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行CoinGecko任务");
        
        let mut all_metrics = Vec::new();
        
        // 获取热门代币价格
        match self.fetch_trending_prices().await {
            Ok(mut metrics) => {
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                error!("❌ 获取热门代币价格失败: {}", e);
                // 继续执行其他任务，不中断
            }
        }
        
        // 获取配置的代币价格
        match self.fetch_configured_prices().await {
            Ok(mut metrics) => {
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                error!("❌ 获取配置代币价格失败: {}", e);
                // 继续执行其他任务，不中断
            }
        }
        
        // 获取全球市场数据
        match self.fetch_global_data().await {
            Ok(mut metrics) => {
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                error!("❌ 获取全球市场数据失败: {}", e);
                // 继续执行其他任务，不中断
            }
        }
        
        // 保存到数据库
        if !all_metrics.is_empty() {
            storage.save_metrics(&all_metrics).await
                .context("保存CoinGecko数据到数据库失败")?;
        }
        
        info!("✅ CoinGecko任务执行完成，共获取 {} 条数据", all_metrics.len());
        
        Ok(all_metrics)
    }
    
    async fn health_check(&self) -> Result<bool> {
        debug!("🏥 正在检查CoinGecko任务健康状态");
        
        match self.client.check_api_key().await {
            Ok(is_valid) => {
                if is_valid {
                    info!("✅ CoinGecko任务健康状态良好");
                } else {
                    error!("❌ CoinGecko API密钥无效");
                }
                Ok(is_valid)
            }
            Err(e) => {
                error!("❌ CoinGecko健康检查失败: {}", e);
                Ok(false)
            }
        }
    }
    
    fn status(&self) -> TaskStatus {
        let status_value = self.status.load(Ordering::Relaxed);
        match status_value {
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

/// CoinGecko任务构建器
/// 
/// 用于方便地创建CoinGecko任务实例
pub struct CoinGeckoTaskBuilder {
    client: Option<Arc<CoinGeckoClient>>,
    interval: Duration,
    coin_ids: Vec<String>,
}

impl CoinGeckoTaskBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            client: None,
            interval: Duration::from_secs(300), // 默认5分钟
            coin_ids: Vec::new(),
        }
    }
    
    /// 设置客户端
    pub fn client(mut self, client: Arc<CoinGeckoClient>) -> Self {
        self.client = Some(client);
        self
    }
    
    /// 设置执行间隔
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
    
    /// 设置要获取的代币ID列表
    pub fn coin_ids(mut self, coin_ids: Vec<String>) -> Self {
        self.coin_ids = coin_ids;
        self
    }
    
    /// 添加单个代币ID
    pub fn add_coin_id(mut self, coin_id: impl Into<String>) -> Self {
        self.coin_ids.push(coin_id.into());
        self
    }
    
    /// 构建任务
    pub fn build(self) -> Result<CoinGeckoTask> {
        let client = self.client.ok_or_else(|| anyhow::anyhow!("客户端未设置"))?;
        
        Ok(CoinGeckoTask::new(client, self.interval, self.coin_ids))
    }
}

impl Default for CoinGeckoTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
} 