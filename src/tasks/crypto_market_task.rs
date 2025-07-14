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

/// 加密货币市场数据任务
pub struct CryptoMarketTask {
    /// 任务名称
    name: String,
    /// CoinMarketCap客户端
    coinmarketcap_client: Arc<CoinMarketCapClient>,
    /// 任务执行间隔（秒）
    interval_seconds: u64,
}

impl CryptoMarketTask {
    /// 创建新的加密货币市场数据任务
    pub fn new(
        name: String,
        coinmarketcap_client: Arc<CoinMarketCapClient>,
        interval_seconds: u64,
    ) -> Self {
        Self {
            name,
            coinmarketcap_client,
            interval_seconds,
        }
    }

    /// 收集市场数据
    async fn collect_market_data(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>> {
        info!("📊 开始收集加密货币市场数据");

        let mut metrics = Vec::new();

        // 收集HYPE代币数据
        match self.collect_hype_data().await {
            Ok(coin_data) => {
                info!("✅ 成功获取HYPE代币数据");
                
                // 存储到缓存
                cache.set_coin_data("hype", serde_json::to_value(&coin_data)?).await;

                // 创建指标
                let metric = MetricBuilder::new(
                    DataSource::from_str(&coin_data.data_source),
                    "hype_market_data".to_string()
                )
                .value(serde_json::json!(coin_data.current_price))
                .metadata(serde_json::json!({
                    "coin_id": "hype",
                    "market_cap": coin_data.market_cap,
                    "volume_24h": coin_data.total_volume,
                    "price_change_24h": coin_data.price_change_percentage_24h,
                    "price_change_7d": coin_data.price_change_percentage_7d,
                    "market_cap_rank": coin_data.market_cap_rank,
                    "rsi": coin_data.rsi,
                    "bollinger_bands": coin_data.bollinger_bands,
                    "technical_analysis": coin_data.technical_analysis,
                    "investment_advice": coin_data.investment_advice,
                    "data_source": coin_data.data_source
                }))
                .build();

                metrics.push(metric);
            }
            Err(e) => {
                error!("❌ 获取HYPE代币数据失败: {}", e);
                return Err(e);
            }
        }

        info!("✅ 市场数据收集完成，共收集到 {} 个指标", metrics.len());
        Ok(metrics)
    }

    /// 收集HYPE代币数据
    async fn collect_hype_data(&self) -> Result<CoinData> {
        info!("💰 开始收集HYPE代币数据");

        // 直接使用CoinMarketCap API获取HYPE数据
        match self.coinmarketcap_client.get_cryptocurrency_data("HYPE").await {
            Ok(cmc_data) => {
                info!("✅ 从CoinMarketCap获取HYPE数据成功");
                Ok(CoinData::from_coinmarketcap(cmc_data))
            }
            Err(e) => {
                error!("❌ CoinMarketCap HYPE数据获取失败: {}", e);
                Err(anyhow::anyhow!("无法从CoinMarketCap获取HYPE数据: {}", e))
            }
        }
    }
}

/// 币种数据结构
#[derive(Debug, Clone, serde::Serialize)]
struct CoinData {
    name: String,
    symbol: String,
    current_price: f64,
    market_cap: f64,
    market_cap_rank: Option<u64>,
    total_volume: f64,
    price_change_24h: f64,
    price_change_percentage_24h: f64,
    price_change_percentage_7d: Option<f64>,
    data_source: String,
    bollinger_bands: serde_json::Value,
    rsi: f64,
    investment_advice: String,
    technical_analysis: String,
}

impl CoinData {
    /// 从CoinMarketCap数据创建CoinData
    fn from_coinmarketcap(data: crate::clients::CryptocurrencyData) -> Self {
        let rsi = Self::calculate_rsi(data.price);
        let bollinger_bands = Self::calculate_bollinger_bands(data.price);
        let technical_analysis = Self::generate_technical_analysis_cmc(rsi, &data);
        let investment_advice = Self::generate_investment_advice_cmc(&data);

        Self {
            name: data.name,
            symbol: data.symbol,
            current_price: data.price,
            market_cap: data.market_cap,
            market_cap_rank: data.cmc_rank,
            total_volume: data.volume_24h,
            price_change_24h: data.percent_change_24h,
            price_change_percentage_24h: data.percent_change_24h,
            price_change_percentage_7d: data.percent_change_7d,
            data_source: "CoinMarketCap".to_string(),
            bollinger_bands,
            rsi,
            investment_advice,
            technical_analysis,
        }
    }

    /// 计算RSI指标（简化版）
    fn calculate_rsi(price: f64) -> f64 {
        // 简化的RSI计算，实际应用中需要历史价格数据
        (price % 100.0).max(0.0).min(100.0)
    }

    /// 计算布林带指标（简化版）
    fn calculate_bollinger_bands(price: f64) -> serde_json::Value {
        let std_dev = price * 0.02; // 假设标准差为价格的2%
        serde_json::json!({
            "upper": price + (2.0 * std_dev),
            "middle": price,
            "lower": price - (2.0 * std_dev)
        })
    }

    /// 生成技术分析（CoinMarketCap版本）
    fn generate_technical_analysis_cmc(rsi: f64, data: &crate::clients::CryptocurrencyData) -> String {
        let mut analysis = Vec::new();
        
        // RSI分析
        if rsi > 70.0 {
            analysis.push("RSI显示超买状态");
        } else if rsi < 30.0 {
            analysis.push("RSI显示超卖状态");
        } else {
            analysis.push("RSI处于正常范围");
        }
        
        // 价格变化分析
        if data.percent_change_24h > 10.0 {
            analysis.push("24小时涨幅较大，需注意回调风险");
        } else if data.percent_change_24h < -10.0 {
            analysis.push("24小时跌幅较大，可能存在反弹机会");
        }
        
        analysis.join("；")
    }

    /// 生成投资建议（CoinMarketCap版本）
    fn generate_investment_advice_cmc(data: &crate::clients::CryptocurrencyData) -> String {
        if data.percent_change_24h > 15.0 {
            "涨幅过大，建议观望或止盈".to_string()
        } else if data.percent_change_24h > 5.0 {
            "表现良好，可考虑适度持有".to_string()
        } else if data.percent_change_24h < -15.0 {
            "跌幅较大，谨慎抄底".to_string()
        } else if data.percent_change_24h < -5.0 {
            "出现回调，可关注买入机会".to_string()
        } else {
            "价格相对稳定，持续观察".to_string()
        }
    }
}

#[async_trait]
impl Task for CryptoMarketTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "收集加密货币市场数据，包括价格、市值、交易量等信息"
    }

    fn id(&self) -> &str {
        "crypto_market_task"
    }

    fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }

    async fn execute(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>> {
        info!("🚀 开始执行加密货币市场数据任务: {}", self.name);
        
        let result = self.collect_market_data(cache).await;
        
        match &result {
            Ok(metrics) => info!("✅ 加密货币市场数据任务执行成功，收集到 {} 个指标", metrics.len()),
            Err(e) => error!("❌ 加密货币市场数据任务执行失败: {}", e),
        }
        
        result
    }
}

/// 加密货币市场数据任务构建器
pub struct CryptoMarketTaskBuilder {
    coinmarketcap_client: Option<Arc<CoinMarketCapClient>>,
    interval_seconds: Option<u64>,
    name: Option<String>,
}

impl CryptoMarketTaskBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            coinmarketcap_client: None,
            interval_seconds: None,
            name: None,
        }
    }

    /// 设置CoinMarketCap客户端
    pub fn coinmarketcap_client(mut self, client: Arc<CoinMarketCapClient>) -> Self {
        self.coinmarketcap_client = Some(client);
        self
    }

    /// 设置执行间隔
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
    pub fn build(self) -> Result<CryptoMarketTask> {
        let coinmarketcap_client = self.coinmarketcap_client
            .ok_or_else(|| anyhow::anyhow!("CoinMarketCap client is required"))?;
        let interval_seconds = self.interval_seconds.unwrap_or(14400); // 默认4小时
        let name = self.name.unwrap_or_else(|| "加密货币市场数据任务".to_string());

        Ok(CryptoMarketTask::new(name, coinmarketcap_client, interval_seconds))
    }
}

impl Default for CryptoMarketTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource {
    /// 从字符串创建数据源
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "coinmarketcap" => DataSource::CoinMarketCap,
            _ => DataSource::CoinMarketCap,
        }
    }
}