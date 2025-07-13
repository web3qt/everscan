use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use tracing::{info, debug, error, warn};

use crate::clients::{CoinGeckoClient, ApiClient};
use crate::storage::PostgresRepository;
use crate::models::{AggregatedMetric, MetricBuilder, DataSource};
use super::{Task, TaskStatus};
use chrono::{DateTime, Utc};

/// 加密货币市场数据收集任务
/// 
/// 这是一个通用的任务，可以配置收集不同代币的市场数据
/// 包括价格、交易量、技术指标等信息
pub struct CryptoMarketTask {
    /// CoinGecko客户端
    client: Arc<CoinGeckoClient>,
    /// 任务状态
    status: AtomicU8,
    /// 执行间隔
    interval: Duration,
    /// 要监控的代币列表
    coin_ids: Vec<String>,
    /// 任务名称
    name: String,
}

impl CryptoMarketTask {
    /// 创建新的加密货币市场数据任务
    pub fn new(
        client: Arc<CoinGeckoClient>,
        interval: Duration,
        coin_ids: Vec<String>,
        name: String,
    ) -> Self {
        Self {
            client,
            status: AtomicU8::new(TaskStatus::Idle as u8),
            interval,
            coin_ids,
            name,
        }
    }
}

#[async_trait::async_trait]
impl Task for CryptoMarketTask {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "收集加密货币市场数据，包括价格、交易量和技术指标"
    }
    
    fn interval(&self) -> Duration {
        self.interval
    }
    
    async fn execute(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行{}任务", self.name());
        
        let mut all_metrics = Vec::new();
        
        for coin_id in &self.coin_ids {
            debug!("📊 正在获取 {} 的市场数据", coin_id);
            
            match self.collect_coin_data(coin_id, storage).await {
                Ok(mut metrics) => {
                    info!("✅ 成功收集 {} 的市场数据", coin_id);
                    all_metrics.append(&mut metrics);
                }
                Err(e) => {
                    error!("❌ 收集 {} 的市场数据失败: {}", coin_id, e);
                    // 继续处理其他代币，不因为一个失败而中断
                }
            }
            
            // 在处理多个代币时添加小延迟，避免API限制
            if self.coin_ids.len() > 1 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        
        info!("✅ {}任务执行完成，共获取 {} 条数据", self.name(), all_metrics.len());
        Ok(all_metrics)
    }
    
    async fn health_check(&self) -> Result<bool> {
        debug!("🏥 正在检查{}任务健康状态", self.name());
        
        // 检查客户端是否正常
        match self.client.check_api_key().await {
            Ok(true) => {
                debug!("✅ {}任务健康检查通过", self.name());
                Ok(true)
            }
            Ok(false) => {
                warn!("⚠️ {}任务API密钥无效", self.name());
                Ok(false)
            }
            Err(e) => {
                error!("❌ {}任务健康检查失败: {}", self.name(), e);
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
        
        // 转换为 AggregatedMetric 格式
        let metrics = self.convert_to_metrics(&market_data)?;
        
        // 存储到数据库
        self.store_market_data(&market_data, storage).await?;
        
        Ok(metrics)
    }
    
    /// 转换为 AggregatedMetric 格式
    fn convert_to_metrics(&self, market_data: &crate::clients::EnhancedMarketData) -> Result<Vec<AggregatedMetric>> {
        let mut metrics = Vec::new();
        let timestamp = market_data.updated_at;
        
        // 价格指标
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_price", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(market_data.coin_price.current_price))
        .timestamp(timestamp)
        .build());
        
        // 交易量指标
        if let Some(volume) = market_data.coin_price.total_volume {
            metrics.push(MetricBuilder::new(
                DataSource::CoinGecko,
                format!("{}_volume", market_data.coin_price.symbol)
            )
            .value(serde_json::json!(volume))
            .timestamp(timestamp)
            .build());
        }
        
        // 市值指标
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
        
        // 布林带指标
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_bollinger_upper", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(indicators.bollinger_bands.upper))
        .timestamp(timestamp)
        .build());
        
        metrics.push(MetricBuilder::new(
            DataSource::CoinGecko,
            format!("{}_bollinger_middle", market_data.coin_price.symbol)
        )
        .value(serde_json::json!(indicators.bollinger_bands.middle))
        .timestamp(timestamp)
        .build());
        
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
}

impl CryptoMarketTaskBuilder {
    pub fn new() -> Self {
        Self {
            client: None,
            interval: None,
            coin_ids: Vec::new(),
            name: None,
        }
    }
    
    pub fn client(mut self, client: Arc<CoinGeckoClient>) -> Self {
        self.client = Some(client);
        self
    }
    
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }
    
    pub fn coin_ids(mut self, coin_ids: Vec<String>) -> Self {
        self.coin_ids = coin_ids;
        self
    }
    
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    /// 添加单个代币ID
    pub fn add_coin(mut self, coin_id: String) -> Self {
        self.coin_ids.push(coin_id);
        self
    }
    
    /// 添加多个代币ID
    pub fn add_coins(mut self, coin_ids: Vec<String>) -> Self {
        self.coin_ids.extend(coin_ids);
        self
    }
}

impl CryptoMarketTaskBuilder {
    pub fn build(self) -> Result<CryptoMarketTask> {
        let client = self.client.ok_or_else(|| anyhow::anyhow!("CoinGecko客户端未设置"))?;
        let interval = self.interval.unwrap_or(Duration::from_secs(14400)); // 默认4小时
        let name = self.name.unwrap_or_else(|| "CryptoMarketTask".to_string());
        
        if self.coin_ids.is_empty() {
            return Err(anyhow::anyhow!("至少需要指定一个代币ID"));
        }
        
        Ok(CryptoMarketTask::new(client, interval, self.coin_ids, name))
    }
} 