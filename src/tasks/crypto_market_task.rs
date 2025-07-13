use anyhow::{Result, Context};
use async_trait::async_trait;
use std::sync::{Arc, atomic::{AtomicU8, Ordering}};
use std::time::Duration;
use tracing::{info, debug, error, warn};
use chrono::Utc;

use crate::clients::{CoinGeckoClient, EnhancedMarketData};
use crate::storage::PostgresRepository;
use crate::models::{AggregatedMetric, MetricBuilder, DataSource};
use crate::web::cache::DataCache; // 新增：导入数据缓存
use super::{Task, TaskStatus};

/// 加密货币市场数据任务
/// 
/// 负责定期获取配置的加密货币市场数据和技术指标
/// 支持多币种监控和实时数据缓存
pub struct CryptoMarketTask {
    /// 任务名称
    name: String,
    /// CoinGecko客户端
    client: Arc<CoinGeckoClient>,
    /// 要监控的币种ID列表
    coin_ids: Vec<String>,
    /// 任务执行间隔
    interval: Duration,
    /// 任务状态
    status: AtomicU8,
    /// 数据缓存（新增）
    cache: Option<Arc<DataCache>>,
}

impl CryptoMarketTask {
    /// 创建新的加密货币市场数据任务
    /// 
    /// # 参数
    /// * `name` - 任务名称
    /// * `client` - CoinGecko客户端
    /// * `coin_ids` - 要监控的币种ID列表
    /// * `interval` - 执行间隔
    /// * `cache` - 数据缓存（可选）
    /// 
    /// # 返回
    /// * `Self` - 任务实例
    pub fn new(
        name: String,
        client: Arc<CoinGeckoClient>,
        coin_ids: Vec<String>,
        interval: Duration,
        cache: Option<Arc<DataCache>>,
    ) -> Self {
        info!("🚀 创建加密货币市场数据任务: {}", name);
        info!("📊 监控币种: {:?}", coin_ids);
        info!("⏰ 执行间隔: {:?}", interval);
        
        Self {
            name,
            client,
            coin_ids,
            interval,
            status: AtomicU8::new(TaskStatus::Idle as u8),
            cache,
        }
    }
    
    /// 设置数据缓存
    /// 
    /// # 参数
    /// * `cache` - 数据缓存
    pub fn with_cache(mut self, cache: Arc<DataCache>) -> Self {
        self.cache = Some(cache);
        self
    }
}

#[async_trait]
impl Task for CryptoMarketTask {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "获取加密货币市场数据和技术指标"
    }
    
    fn interval(&self) -> Duration {
        self.interval
    }
    
    async fn execute(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行加密货币市场数据任务: {}", self.name);
        self.set_status(TaskStatus::Running);
        
        let mut all_metrics = Vec::new();
        let mut successful_updates = 0;
        
        for (index, coin_id) in self.coin_ids.iter().enumerate() {
            info!("🔍 [{}/{}] 获取 {} 的市场数据", index + 1, self.coin_ids.len(), coin_id);
            
            match self.collect_coin_data(coin_id, storage).await {
                Ok(mut metrics) => {
                    all_metrics.append(&mut metrics);
                    successful_updates += 1;
                    info!("✅ 成功获取 {} 的数据", coin_id);
                }
                Err(e) => {
                    error!("❌ 获取 {} 数据失败: {}", coin_id, e);
                    // 继续处理其他币种，不中断整个任务
                }
            }
            
            // 在请求之间添加延迟，避免触发API限制
            if index < self.coin_ids.len() - 1 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        
        // 保存到数据库（如果需要）
        if !all_metrics.is_empty() {
            match storage.save_metrics(&all_metrics).await {
                Ok(saved_count) => {
                    info!("💾 成功保存 {} 条指标数据到数据库", saved_count);
                }
                Err(e) => {
                    warn!("⚠️ 保存数据到数据库失败: {}", e);
                    // 不影响任务成功状态，因为数据已缓存
                }
            }
        }
        
        self.set_status(TaskStatus::Completed);
        info!("✅ 加密货币市场数据任务完成: 成功更新 {}/{} 个币种", successful_updates, self.coin_ids.len());
        
        Ok(all_metrics)
    }
    
    async fn health_check(&self) -> Result<bool> {
        // 检查CoinGecko客户端是否正常
        match self.client.get_simple_price(&["bitcoin"], &["usd"]).await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("❌ CoinGecko健康检查失败: {}", e);
                Ok(false)
            }
        }
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

impl CryptoMarketTask {
    /// 收集单个代币的市场数据
    async fn collect_coin_data(&self, coin_id: &str, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>> {
        // 获取增强的市场数据（包含技术指标）
        let market_data = self.client.get_enhanced_market_data(coin_id, "usd").await?;
        
        // 记录获取到的数据
        info!("📈 {} 市场数据:", market_data.coin_price.name);
        info!("   当前价格: ${:.2}", market_data.coin_price.current_price);
        if let Some(volume) = market_data.coin_price.total_volume {
            info!("   24小时交易量: ${:.0}", volume);
        }
        if let Some(change) = market_data.coin_price.price_change_percentage_24h {
            info!("   24小时涨跌幅: {:.2}%", change);
        }
        
        // 技术指标
        let indicators = &market_data.technical_indicators;
        info!("📊 技术指标:");
        info!("   布林带上轨: ${:.2}", indicators.bollinger_bands.upper);
        info!("   布林带中轨: ${:.2}", indicators.bollinger_bands.middle);
        info!("   布林带下轨: ${:.2}", indicators.bollinger_bands.lower);
        info!("   RSI: {:.2}", indicators.rsi.value);
        
        // RSI信号分析
        if indicators.rsi.value >= indicators.rsi.overbought_threshold {
            warn!("⚠️ {} RSI超买信号 (RSI: {:.2})", market_data.coin_price.symbol.to_uppercase(), indicators.rsi.value);
        } else if indicators.rsi.value <= indicators.rsi.oversold_threshold {
            warn!("⚠️ {} RSI超卖信号 (RSI: {:.2})", market_data.coin_price.symbol.to_uppercase(), indicators.rsi.value);
        }
        
        // 更新缓存（新增）
        if let Some(cache) = &self.cache {
            match cache.update_market_data(coin_id, &market_data) {
                Ok(_) => {
                    debug!("💾 已更新 {} 的缓存数据", coin_id);
                }
                Err(e) => {
                    warn!("⚠️ 更新 {} 缓存失败: {}", coin_id, e);
                }
            }
        }
        
        // 转换为 AggregatedMetric 格式
        let metrics = self.convert_to_metrics(&market_data)?;
        
        // 存储到数据库
        self.store_market_data(&market_data, storage).await?;
        
        Ok(metrics)
    }
    
    /// 转换为 AggregatedMetric 格式
    fn convert_to_metrics(&self, market_data: &EnhancedMarketData) -> Result<Vec<AggregatedMetric>> {
        let mut metrics = Vec::new();
        let timestamp = Utc::now();
        
        // 基础价格数据
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_price", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(market_data.coin_price.current_price))
        .timestamp(timestamp)
        .build());
        
        // 24小时交易量
        if let Some(volume) = market_data.coin_price.total_volume {
            metrics.push(MetricBuilder::new(
                DataSource::CoinGecko,
                format!("{}_volume_24h", market_data.coin_price.symbol)
            )
            .value(serde_json::json!(volume))
            .timestamp(timestamp)
            .build());
        }
        
        // 24小时价格变化
        if let Some(change) = market_data.coin_price.price_change_percentage_24h {
            metrics.push(MetricBuilder::new(
                DataSource::CoinGecko,
                format!("{}_change_24h", market_data.coin_price.symbol)
            )
            .value(serde_json::json!(change))
            .timestamp(timestamp)
            .build());
        }
        
        // 市值
        if let Some(market_cap) = market_data.coin_price.market_cap {
            metrics.push(MetricBuilder::new(
                DataSource::CoinGecko,
                format!("{}_market_cap", market_data.coin_price.symbol)
            )
            .value(serde_json::json!(market_cap))
            .timestamp(timestamp)
            .build());
        }
        
        // 技术指标
        let indicators = &market_data.technical_indicators;
        
        // 布林带上轨
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_bollinger_upper", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(indicators.bollinger_bands.upper))
        .timestamp(timestamp)
        .build());
        
        // 布林带中轨
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_bollinger_middle", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(indicators.bollinger_bands.middle))
        .timestamp(timestamp)
        .build());
        
        // 布林带下轨
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_bollinger_lower", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(indicators.bollinger_bands.lower))
        .timestamp(timestamp)
        .build());
        
        // RSI指标
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_rsi", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(indicators.rsi.value))
        .timestamp(timestamp)
        .build());
        
        Ok(metrics)
    }
    
    /// 存储市场数据到数据库
    async fn store_market_data(&self, market_data: &crate::clients::EnhancedMarketData, _storage: &PostgresRepository) -> Result<()> {
        debug!("💾 正在存储 {} 的市场数据到数据库", market_data.coin_price.symbol);
        
        // 注意：这里我们不直接存储原始数据，而是通过 AggregatedMetric 系统存储
        // 实际的存储会在 Task::execute 方法中通过 storage.save_metrics 完成
        
        debug!("✅ 市场数据已准备存储: {}", market_data.coin_price.symbol);
        Ok(())
    }
}

/// 加密货币市场数据任务构建器
pub struct CryptoMarketTaskBuilder {
    client: Option<Arc<CoinGeckoClient>>,
    interval: Option<Duration>,
    coin_ids: Vec<String>,
    name: Option<String>,
    cache: Option<Arc<DataCache>>, // 新增：缓存字段
}

impl CryptoMarketTaskBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            client: None,
            interval: None,
            coin_ids: Vec::new(),
            name: None,
            cache: None,
        }
    }
    
    /// 设置CoinGecko客户端
    pub fn client(mut self, client: Arc<CoinGeckoClient>) -> Self {
        self.client = Some(client);
        self
    }
    
    /// 设置执行间隔
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }
    
    /// 添加要监控的币种
    pub fn add_coin(mut self, coin_id: String) -> Self {
        self.coin_ids.push(coin_id);
        self
    }
    
    /// 设置要监控的币种列表
    pub fn coin_ids(mut self, coin_ids: Vec<String>) -> Self {
        self.coin_ids = coin_ids;
        self
    }
    
    /// 设置任务名称
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    /// 设置数据缓存（新增）
    pub fn cache(mut self, cache: Arc<DataCache>) -> Self {
        self.cache = Some(cache);
        self
    }
    
    /// 构建任务
    pub fn build(self) -> Result<CryptoMarketTask> {
        let client = self.client.context("缺少CoinGecko客户端")?;
        let interval = self.interval.unwrap_or(Duration::from_secs(14400)); // 默认4小时
        let name = self.name.unwrap_or_else(|| "CryptoMarketTask".to_string());
        
        if self.coin_ids.is_empty() {
            return Err(anyhow::anyhow!("至少需要指定一个要监控的币种"));
        }
        
        Ok(CryptoMarketTask::new(
            name,
            client,
            self.coin_ids,
            interval,
            self.cache,
        ))
    }
}

impl Default for CryptoMarketTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
} 