use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, debug, error};
use std::time::Duration;
use std::collections::HashMap;

use super::{ApiClient, HttpClientBuilder};

/// CoinGecko API客户端
/// 
/// 用于与CoinGecko API进行交互
/// 支持获取代币价格、市值、交易量等市场数据
pub struct CoinGeckoClient {
    /// HTTP客户端
    client: reqwest::Client,
    /// API密钥（可选，Pro版本需要）
    api_key: Option<String>,
    /// API基础URL
    base_url: String,
    /// 超时时间
    timeout: Duration,
}

/// 代币价格信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoinPrice {
    /// 代币ID
    pub id: String,
    /// 代币符号
    pub symbol: String,
    /// 代币名称
    pub name: String,
    /// 当前价格（美元）
    pub current_price: f64,
    /// 市值
    pub market_cap: Option<f64>,
    /// 市值排名
    pub market_cap_rank: Option<u32>,
    /// 24小时交易量
    pub total_volume: Option<f64>,
    /// 24小时价格变化百分比
    pub price_change_percentage_24h: Option<f64>,
    /// 最后更新时间
    pub last_updated: String,
}

/// 市场数据
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketData {
    /// 总市值
    pub total_market_cap: HashMap<String, f64>,
    /// 总交易量
    pub total_volume: HashMap<String, f64>,
    /// 市场占有率
    pub market_cap_percentage: HashMap<String, f64>,
    /// 最后更新时间
    pub updated_at: i64,
}

/// 全球市场数据
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalData {
    /// 总市值
    pub total_market_cap: HashMap<String, f64>,
    /// 总交易量
    pub total_volume: HashMap<String, f64>,
    /// 市场占有率
    pub market_cap_percentage: HashMap<String, f64>,
    /// 活跃加密货币数量
    pub active_cryptocurrencies: u32,
    /// 市场数量
    pub markets: u32,
    /// 最后更新时间
    pub updated_at: i64,
}

impl CoinGeckoClient {
    /// 创建新的CoinGecko客户端
    /// 
    /// # 参数
    /// * `api_key` - API密钥（可选）
    /// * `timeout` - HTTP超时时间
    /// 
    /// # 返回
    /// * `Result<Self>` - 创建的客户端或错误
    pub fn new(api_key: Option<String>, timeout: Duration) -> Result<Self> {
        let client = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-CoinGeckoClient/1.0")
            .build()?;
        
        // 根据是否有API密钥选择不同的基础URL
        let base_url = if api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3".to_string()
        } else {
            "https://api.coingecko.com/api/v3".to_string()
        };
        
        Ok(Self {
            client,
            api_key,
            base_url,
            timeout,
        })
    }
    
    /// 获取代币价格
    /// 
    /// # 参数
    /// * `coin_ids` - 代币ID列表
    /// * `vs_currency` - 对比货币（默认为"usd"）
    /// 
    /// # 返回
    /// * `Result<Vec<CoinPrice>>` - 代币价格列表或错误
    pub async fn get_coin_prices(&self, coin_ids: &[String], vs_currency: &str) -> Result<Vec<CoinPrice>> {
        let ids = coin_ids.join(",");
        let url = format!("{}/coins/markets", self.base_url);
        
        debug!("💰 正在获取CoinGecko代币价格: {:?}", coin_ids);
        
        let mut request = self.client
            .get(&url)
            .query(&[
                ("ids", ids.as_str()),
                ("vs_currency", vs_currency),
                ("order", "market_cap_desc"),
                ("per_page", "250"),
                ("page", "1"),
                ("sparkline", "false"),
            ]);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送CoinGecko价格请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ CoinGecko API请求失败: {} - {}", status, text);
            return Err(anyhow!("CoinGecko API请求失败: {} - {}", status, text));
        }
        
        let prices: Vec<CoinPrice> = response
            .json()
            .await
            .context("解析CoinGecko价格响应失败")?;
        
        info!("✅ 获取到 {} 个代币的价格信息", prices.len());
        
        Ok(prices)
    }
    
    /// 获取全球市场数据
    /// 
    /// # 返回
    /// * `Result<GlobalData>` - 全球市场数据或错误
    pub async fn get_global_data(&self) -> Result<GlobalData> {
        let url = format!("{}/global", self.base_url);
        
        debug!("🌍 正在获取CoinGecko全球市场数据");
        
        let mut request = self.client.get(&url);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送CoinGecko全球数据请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ CoinGecko全球数据请求失败: {} - {}", status, text);
            return Err(anyhow!("CoinGecko全球数据请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析CoinGecko全球数据响应失败")?;
        
        // 提取data字段
        let data = result["data"].clone();
        let global_data: GlobalData = serde_json::from_value(data)
            .context("解析CoinGecko全球数据失败")?;
        
        info!("✅ 获取CoinGecko全球市场数据成功");
        
        Ok(global_data)
    }
    
    /// 获取特定代币的详细信息
    /// 
    /// # 参数
    /// * `coin_id` - 代币ID
    /// 
    /// # 返回
    /// * `Result<Value>` - 代币详细信息或错误
    pub async fn get_coin_details(&self, coin_id: &str) -> Result<Value> {
        let url = format!("{}/coins/{}", self.base_url, coin_id);
        
        debug!("🔍 正在获取代币详细信息: {}", coin_id);
        
        let mut request = self.client
            .get(&url)
            .query(&[
                ("localization", "false"),
                ("tickers", "false"),
                ("market_data", "true"),
                ("community_data", "true"),
                ("developer_data", "true"),
                ("sparkline", "false"),
            ]);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送CoinGecko代币详情请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ CoinGecko代币详情请求失败: {} - {}", status, text);
            return Err(anyhow!("CoinGecko代币详情请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析CoinGecko代币详情响应失败")?;
        
        info!("✅ 获取代币详细信息成功: {}", coin_id);
        
        Ok(result)
    }
    
    /// 获取热门代币列表
    /// 
    /// # 返回
    /// * `Result<Vec<String>>` - 热门代币ID列表或错误
    pub async fn get_trending_coins(&self) -> Result<Vec<String>> {
        let url = format!("{}/search/trending", self.base_url);
        
        debug!("🔥 正在获取CoinGecko热门代币");
        
        let mut request = self.client.get(&url);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送CoinGecko热门代币请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ CoinGecko热门代币请求失败: {} - {}", status, text);
            return Err(anyhow!("CoinGecko热门代币请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析CoinGecko热门代币响应失败")?;
        
        // 提取代币ID
        let mut coin_ids = Vec::new();
        if let Some(coins) = result["coins"].as_array() {
            for coin in coins {
                if let Some(item) = coin["item"].as_object() {
                    if let Some(id) = item["id"].as_str() {
                        coin_ids.push(id.to_string());
                    }
                }
            }
        }
        
        info!("✅ 获取到 {} 个热门代币", coin_ids.len());
        
        Ok(coin_ids)
    }
    
    /// 获取支持的货币列表
    /// 
    /// # 返回
    /// * `Result<Vec<String>>` - 支持的货币列表或错误
    pub async fn get_supported_currencies(&self) -> Result<Vec<String>> {
        let url = format!("{}/simple/supported_vs_currencies", self.base_url);
        
        let mut request = self.client.get(&url);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送CoinGecko支持货币请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("CoinGecko支持货币请求失败: {} - {}", status, text));
        }
        
        let currencies: Vec<String> = response
            .json()
            .await
            .context("解析CoinGecko支持货币响应失败")?;
        
        Ok(currencies)
    }
}

#[async_trait::async_trait]
impl ApiClient for CoinGeckoClient {
    fn source_name(&self) -> &str {
        "coingecko"
    }
    
    async fn check_api_key(&self) -> Result<bool> {
        // 尝试获取全球数据来验证API密钥
        match self.get_global_data().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    async fn fetch_raw_data(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);
        
        let mut request = self.client.get(&url);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("CoinGecko API请求失败: {} - {}", status, text));
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        // 重新构建客户端
        if let Ok(client) = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-CoinGeckoClient/1.0")
            .build() {
            self.client = client;
        }
    }
} 