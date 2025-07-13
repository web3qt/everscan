use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, debug, error};
use std::time::Duration;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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

/// 增强的代币市场数据（包含技术指标）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMarketData {
    /// 基础价格信息
    pub coin_price: CoinPrice,
    /// 技术指标
    pub technical_indicators: TechnicalIndicators,
    /// 数据更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 技术指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    /// 布林带
    pub bollinger_bands: BollingerBands,
    /// RSI（相对强弱指数）
    pub rsi: RSI,
}

/// 布林带指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBands {
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

/// RSI指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSI {
    /// RSI值
    pub value: f64,
    /// 计算周期
    pub period: u32,
    /// 超买阈值
    pub overbought_threshold: f64,
    /// 超卖阈值
    pub oversold_threshold: f64,
}

/// 历史价格数据点
#[derive(Debug, Clone, Deserialize)]
pub struct PricePoint {
    /// 时间戳（毫秒）
    pub timestamp: i64,
    /// 价格
    pub price: f64,
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

    /// 获取代币的历史价格数据
    /// 
    /// # 参数
    /// * `coin_id` - 代币ID（如 "bitcoin"）
    /// * `days` - 历史天数
    /// 
    /// # 返回
    /// * `Result<Vec<PricePoint>>` - 历史价格数据点列表
    pub async fn get_coin_history(&self, coin_id: &str, days: u32) -> Result<Vec<PricePoint>> {
        let url = format!("{}/coins/{}/market_chart", self.base_url, coin_id);
        
        debug!("📈 正在获取 {} 的历史价格数据（{}天）", coin_id, days);
        
        // 根据天数决定是否使用interval参数
        // CoinGecko免费API: 2-90天会自动返回小时级数据，无需指定interval
        let mut request = if days >= 2 && days <= 90 {
            // 2-90天范围内，CoinGecko会自动返回小时级数据
            self.client
                .get(&url)
                .query(&[
                    ("vs_currency", "usd"),
                    ("days", &days.to_string()),
                ])
        } else {
            // 其他情况使用默认间隔
            self.client
                .get(&url)
                .query(&[
                    ("vs_currency", "usd"),
                    ("days", &days.to_string()),
                ])
        };
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-cg-pro-api-key", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送CoinGecko历史数据请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ CoinGecko历史数据请求失败: {} - {}", status, text);
            return Err(anyhow!("CoinGecko历史数据请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析CoinGecko历史数据响应失败")?;
        
        // 解析价格数据
        let mut price_points = Vec::new();
        if let Some(prices) = result["prices"].as_array() {
            for price_data in prices {
                if let Some(price_array) = price_data.as_array() {
                    if price_array.len() >= 2 {
                        if let (Some(timestamp), Some(price)) = (
                            price_array[0].as_i64(),
                            price_array[1].as_f64()
                        ) {
                            price_points.push(PricePoint {
                                timestamp,
                                price,
                            });
                        }
                    }
                }
            }
        }
        
        info!("✅ 获取到 {} 个历史价格数据点", price_points.len());
        Ok(price_points)
    }

    /// 获取增强的市场数据（包含技术指标）
    /// 
    /// # 参数
    /// * `coin_id` - 代币ID（如 "bitcoin"）
    /// * `vs_currency` - 计价货币（如 "usd"）
    /// 
    /// # 返回
    /// * `Result<EnhancedMarketData>` - 包含技术指标的增强市场数据
    pub async fn get_enhanced_market_data(&self, coin_id: &str, vs_currency: &str) -> Result<EnhancedMarketData> {
        info!("🔍 正在获取 {} 的增强市场数据", coin_id);
        
        // 获取当前价格数据
        let coin_prices = self.get_coin_prices(&[coin_id.to_string()], vs_currency).await?;
        let coin_price = coin_prices.into_iter().next()
            .ok_or_else(|| anyhow!("未找到代币 {} 的价格数据", coin_id))?;
        
        // 获取历史价格数据用于计算技术指标
        let history = self.get_coin_history(coin_id, 30).await?; // 获取30天历史数据
        
        // 计算技术指标
        let technical_indicators = self.calculate_technical_indicators(&history)?;
        
        Ok(EnhancedMarketData {
            coin_price,
            technical_indicators,
            updated_at: Utc::now(),
        })
    }

    /// 计算技术指标
    /// 
    /// # 参数
    /// * `price_history` - 历史价格数据
    /// 
    /// # 返回
    /// * `Result<TechnicalIndicators>` - 计算得出的技术指标
    fn calculate_technical_indicators(&self, price_history: &[PricePoint]) -> Result<TechnicalIndicators> {
        if price_history.len() < 20 {
            return Err(anyhow!("历史数据不足，无法计算技术指标（需要至少20个数据点）"));
        }
        
        let prices: Vec<f64> = price_history.iter().map(|p| p.price).collect();
        
        // 计算布林带（20周期，2倍标准差）
        let bollinger_bands = self.calculate_bollinger_bands(&prices, 20, 2.0)?;
        
        // 计算RSI（14周期）
        let rsi = self.calculate_rsi(&prices, 14)?;
        
        Ok(TechnicalIndicators {
            bollinger_bands,
            rsi,
        })
    }

    /// 计算布林带
    /// 
    /// # 参数
    /// * `prices` - 价格数组
    /// * `period` - 计算周期
    /// * `std_dev_multiplier` - 标准差倍数
    /// 
    /// # 返回
    /// * `Result<BollingerBands>` - 布林带数据
    fn calculate_bollinger_bands(&self, prices: &[f64], period: usize, std_dev_multiplier: f64) -> Result<BollingerBands> {
        if prices.len() < period {
            return Err(anyhow!("价格数据不足，无法计算布林带"));
        }
        
        // 取最近的数据计算
        let recent_prices = &prices[prices.len() - period..];
        
        // 计算移动平均线（中轨）
        let middle = recent_prices.iter().sum::<f64>() / period as f64;
        
        // 计算标准差
        let variance = recent_prices.iter()
            .map(|price| (price - middle).powi(2))
            .sum::<f64>() / period as f64;
        let std_dev = variance.sqrt();
        
        // 计算上轨和下轨
        let upper = middle + (std_dev_multiplier * std_dev);
        let lower = middle - (std_dev_multiplier * std_dev);
        
        Ok(BollingerBands {
            upper,
            middle,
            lower,
            period: period as u32,
            std_dev_multiplier,
        })
    }

    /// 计算RSI（相对强弱指数）
    /// 
    /// # 参数
    /// * `prices` - 价格数组
    /// * `period` - 计算周期
    /// 
    /// # 返回
    /// * `Result<RSI>` - RSI数据
    fn calculate_rsi(&self, prices: &[f64], period: usize) -> Result<RSI> {
        if prices.len() < period + 1 {
            return Err(anyhow!("价格数据不足，无法计算RSI"));
        }
        
        // 计算价格变化
        let mut gains = Vec::new();
        let mut losses = Vec::new();
        
        for i in 1..prices.len() {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-change);
            }
        }
        
        if gains.len() < period {
            return Err(anyhow!("价格变化数据不足，无法计算RSI"));
        }
        
        // 取最近的数据计算
        let recent_gains = &gains[gains.len() - period..];
        let recent_losses = &losses[losses.len() - period..];
        
        // 计算平均收益和平均损失
        let avg_gain = recent_gains.iter().sum::<f64>() / period as f64;
        let avg_loss = recent_losses.iter().sum::<f64>() / period as f64;
        
        // 计算RSI
        let rsi_value = if avg_loss == 0.0 {
            100.0 // 如果没有损失，RSI为100
        } else {
            let rs = avg_gain / avg_loss;
            100.0 - (100.0 / (1.0 + rs))
        };
        
        Ok(RSI {
            value: rsi_value,
            period: period as u32,
            overbought_threshold: 70.0,
            oversold_threshold: 30.0,
        })
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