use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// CoinMarketCap API客户端
/// 
/// 用于获取贪婪恐惧指数等市场情绪数据
#[derive(Clone)]
pub struct CoinMarketCapClient {
    /// HTTP客户端
    client: Client,
    /// API密钥（可选，某些端点不需要）
    api_key: Option<String>,
    /// 基础URL
    base_url: String,
}

/// 贪婪恐惧指数数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FearGreedIndex {
    /// 指数值 (0-100)
    pub value: u8,
    /// 指数分类 (如: "Extreme Fear", "Fear", "Neutral", "Greed", "Extreme Greed")
    pub value_classification: String,
    /// 时间戳
    pub timestamp: String,
    /// 更新时间（Unix时间戳）
    pub time_until_update: Option<u64>,
}

/// CoinMarketCap API响应结构
#[derive(Debug, Deserialize)]
struct FearGreedResponse {
    /// 响应数据
    data: Vec<FearGreedData>,
}

/// 贪婪恐惧指数数据结构
#[derive(Debug, Deserialize)]
struct FearGreedData {
    /// 指数值
    value: String,
    /// 指数分类
    value_classification: String,
    /// 时间戳
    timestamp: String,
    /// 更新时间
    time_until_update: Option<String>,
}

impl CoinMarketCapClient {
    /// 创建新的CoinMarketCap客户端
    /// 
    /// # 参数
    /// * `api_key` - API密钥（可选）
    /// * `timeout` - 请求超时时间
    /// 
    /// # 返回
    /// * `Result<Self>` - 客户端实例或错误
    pub fn new(api_key: Option<String>, timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("EverScan/1.0")
            .build()
            .context("创建HTTP客户端失败")?;

        Ok(Self {
            client,
            api_key,
            base_url: "https://api.coinmarketcap.com".to_string(),
        })
    }

    /// 获取贪婪恐惧指数
    /// 
    /// 注意：这个端点使用的是Alternative.me的免费API，不需要CoinMarketCap API密钥
    /// 
    /// # 返回
    /// * `Result<FearGreedIndex>` - 贪婪恐惧指数数据或错误
    pub async fn get_fear_greed_index(&self) -> Result<FearGreedIndex> {
        info!("📊 开始获取贪婪恐惧指数");
        
        // 使用Alternative.me的免费API
        let url = "https://api.alternative.me/fng/";
        
        debug!("🌐 请求URL: {}", url);
        
        let response = self.client
            .get(url)
            .send()
            .await
            .context("发送贪婪恐惧指数请求失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(anyhow::anyhow!(
                "API请求失败: HTTP {} - {}", 
                status, 
                error_text
            ));
        }

        let response_text = response.text().await
            .context("读取响应内容失败")?;
        
        debug!("📥 API响应: {}", response_text);

        let fear_greed_response: FearGreedResponse = serde_json::from_str(&response_text)
            .context("解析贪婪恐惧指数响应失败")?;

        if fear_greed_response.data.is_empty() {
            return Err(anyhow::anyhow!("API返回空数据"));
        }

        let data = &fear_greed_response.data[0];
        
        // 解析指数值
        let value = data.value.parse::<u8>()
            .context("解析贪婪恐惧指数值失败")?;

        // 解析更新时间
        let time_until_update = data.time_until_update.as_ref()
            .and_then(|s| s.parse::<u64>().ok());

        let fear_greed_index = FearGreedIndex {
            value,
            value_classification: data.value_classification.clone(),
            timestamp: data.timestamp.clone(),
            time_until_update,
        };

        info!("✅ 贪婪恐惧指数获取成功: {} - {}", 
              fear_greed_index.value, 
              fear_greed_index.value_classification);

        Ok(fear_greed_index)
    }

    /// 健康检查
    /// 
    /// # 返回
    /// * `Result<bool>` - 健康状态
    pub async fn health_check(&self) -> Result<bool> {
        debug!("🏥 执行CoinMarketCap客户端健康检查");
        
        // 尝试获取贪婪恐惧指数来验证连接
        match self.get_fear_greed_index().await {
            Ok(_) => {
                info!("✅ CoinMarketCap客户端健康检查通过");
                Ok(true)
            }
            Err(e) => {
                warn!("⚠️ CoinMarketCap客户端健康检查失败: {}", e);
                Ok(false)
            }
        }
    }

    /// 获取指数分类的中文描述
    /// 
    /// # 参数
    /// * `classification` - 英文分类
    /// 
    /// # 返回
    /// * `&str` - 中文描述
    pub fn get_chinese_classification(classification: &str) -> &'static str {
        match classification {
            "Extreme Fear" => "极度恐惧",
            "Fear" => "恐惧", 
            "Neutral" => "中性",
            "Greed" => "贪婪",
            "Extreme Greed" => "极度贪婪",
            _ => "未知",
        }
    }

    /// 获取指数值对应的情绪描述
    /// 
    /// # 参数
    /// * `value` - 指数值 (0-100)
    /// 
    /// # 返回
    /// * `&str` - 情绪描述
    pub fn get_sentiment_description(value: u8) -> &'static str {
        match value {
            0..=24 => "极度恐惧",
            25..=44 => "恐惧",
            45..=55 => "中性",
            56..=75 => "贪婪", 
            76..=100 => "极度贪婪",
            _ => "未知",
        }
    }

    /// 获取指数值对应的投资建议
    /// 
    /// # 参数
    /// * `value` - 指数值 (0-100)
    /// 
    /// # 返回
    /// * `&str` - 投资建议
    pub fn get_investment_advice(value: u8) -> &'static str {
        match value {
            0..=24 => "市场极度恐惧，可能是买入机会",
            25..=44 => "市场恐惧，谨慎观察", 
            45..=55 => "市场中性，保持观望",
            56..=75 => "市场贪婪，注意风险",
            76..=100 => "市场极度贪婪，考虑获利了结",
            _ => "市场情况未知，请谨慎投资",
        }
    }
} 