use anyhow::{Result, Context};
use reqwest::{Client, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize, Deserializer};
use std::time::Duration;
use tracing::{info, debug, warn};
use std::collections::HashMap;
use std::fmt;

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

/// 山寨币季节指数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltcoinSeasonIndex {
    /// 指数值 (0-100)
    pub value: u8,
    /// 指数分类 (如: "比特币季节", "平衡市场", "山寨币季节")
    pub classification: String,
    /// 中文分类
    pub classification_zh: String,
    /// 获取时间戳
    pub timestamp: String,
    /// 表现优于比特币的币种数量
    pub outperforming_count: u8,
    /// 总计币种数量（通常是100）
    pub total_count: u8,
    /// 表现优于比特币的百分比
    pub outperforming_percentage: f32,
    /// 投资建议
    pub market_advice: String,
}

/// 加密货币数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptocurrencyData {
    /// 币种ID
    pub id: u64,
    /// 名称
    pub name: String,
    /// 符号
    pub symbol: String,
    /// 价格
    pub price: f64,
    /// 市值
    pub market_cap: f64,
    /// 交易量（24小时）
    pub volume_24h: f64,
    /// 价格变化百分比（24小时）
    pub percent_change_24h: f64,
    /// 价格变化百分比（7天）
    pub percent_change_7d: Option<f64>,
    /// 市值排名
    pub cmc_rank: Option<u64>,
    /// 最后更新时间
    pub last_updated: String,
}

/// CoinMarketCap Fear & Greed API响应结构（最新数据）
#[derive(Debug, Deserialize)]
struct CmcFearGreedResponse {
    /// 响应数据 - 单个对象，不是数组
    data: CmcFearGreedData,
    /// API状态
    status: ApiStatus,
}

/// CoinMarketCap Fear & Greed 数据结构（最新数据）
#[derive(Debug, Deserialize)]
struct CmcFearGreedData {
    /// 指数值 (0-100)
    value: u64,
    /// 指数分类
    value_classification: String,
    /// 更新时间
    update_time: String,
}

/// CoinMarketCap Fear & Greed API响应结构（历史数据）
#[derive(Debug, Deserialize)]
struct CmcFearGreedHistoryResponse {
    /// 响应数据 - 数组格式
    data: Vec<CmcFearGreedHistoryData>,
    /// API状态
    status: ApiStatus,
}

/// CoinMarketCap Fear & Greed 数据结构（历史数据）
#[derive(Debug, Deserialize)]
struct CmcFearGreedHistoryData {
    /// 指数值 (0-100)
    value: u64,
    /// 指数分类
    value_classification: String,
    /// 时间戳
    timestamp: String,
}

/// Legacy API响应结构（Alternative.me格式，已废弃）
#[derive(Debug, Deserialize)]
struct FearGreedResponse {
    /// 响应数据
    data: Vec<FearGreedData>,
}

/// Legacy 贪婪恐惧指数数据结构（Alternative.me格式，已废弃）
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

/// CMC 100指数API响应
#[derive(Debug, Deserialize)]
struct Cmc100Response {
    data: Vec<CmcIndexData>,
    status: ApiStatus,
}

/// CMC指数数据
#[derive(Debug, Deserialize)]
struct CmcIndexData {
    id: u64,
    name: String,
    symbol: String,
    quote: HashMap<String, Quote>,
    cmc_rank: Option<u64>,
    last_updated: String,
}

/// 报价数据
#[derive(Debug, Deserialize)]
struct Quote {
    price: f64,
    market_cap: f64,
    volume_24h: f64,
    percent_change_24h: f64,
    percent_change_7d: Option<f64>,
    last_updated: String,
    // 新增字段
    fully_diluted_market_cap: Option<f64>,
    market_cap_dominance: Option<f64>,
    percent_change_1h: Option<f64>,
    percent_change_30d: Option<f64>,
    percent_change_60d: Option<f64>,
    percent_change_90d: Option<f64>,
    tvl: Option<f64>,
    volume_change_24h: Option<f64>,
}

/// API状态
#[derive(Debug, Deserialize)]
struct ApiStatus {
    timestamp: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    error_code: u64,
    error_message: Option<String>,
    elapsed: u64,
    credit_count: u64,
    notice: Option<String>,
}

/// 加密货币响应
#[derive(Debug, Deserialize)]
struct CryptocurrencyResponse {
    data: HashMap<String, CryptocurrencyInfo>,
    status: ApiStatus,
}

/// 币种信息
#[derive(Debug, Deserialize)]
struct CryptocurrencyInfo {
    id: u64,
    name: String,
    symbol: String,
    slug: String,
    num_market_pairs: Option<u64>,
    date_added: Option<String>,
    tags: Option<Vec<String>>,
    max_supply: Option<f64>,
    circulating_supply: Option<f64>,
    total_supply: Option<f64>,
    platform: Option<serde_json::Value>,
    quote: HashMap<String, Quote>,
    cmc_rank: Option<u64>,
    last_updated: String,
    // 新增字段
    infinite_supply: Option<bool>,
    is_active: Option<u64>,
    is_fiat: Option<u64>,
    self_reported_circulating_supply: Option<f64>,
    self_reported_market_cap: Option<f64>,
    tvl_ratio: Option<f64>,
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
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", HeaderValue::from_static(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        ));
        headers.insert("Accept", HeaderValue::from_static(
            "application/json"
        ));
        headers.insert("Accept-Language", HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip, deflate, br"));
        
        let client = ClientBuilder::new()
            .timeout(timeout)
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("创建HTTP客户端失败")?;

        Ok(CoinMarketCapClient {
            client,
            api_key,
            base_url: "https://pro-api.coinmarketcap.com".to_string(),
        })
    }

    /// 获取贪婪恐惧指数
    /// 
    /// 使用Alternative.me的免费API，不需要CoinMarketCap API密钥
    /// 
    /// # 返回
    /// * `Result<FearGreedIndex>` - 贪婪恐惧指数数据或错误
    pub async fn get_fear_greed_index(&self) -> Result<FearGreedIndex> {
        info!("📊 开始获取贪婪恐惧指数（使用Alternative.me API）");
        
        // 使用Alternative.me的免费API
        let url = "https://api.alternative.me/fng/?limit=1";
        
        debug!("🌐 请求URL: {}", url);
        
        let response = self.client
            .get(url)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "identity")
            .send()
            .await
            .context("发送贪婪恐惧指数请求失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(anyhow::anyhow!(
                "Alternative.me贪婪恐惧指数API请求失败: HTTP {} - {}", 
                status, 
                error_text
            ));
        }

        let response_text = response.text().await
            .context("读取响应内容失败")?;
        
        debug!("📥 API响应: {}", response_text);
        debug!("📄 Alternative.me API原始响应: {}", response_text);

        let alt_response: FearGreedResponse = serde_json::from_str(&response_text)
            .with_context(|| format!("解析Alternative.me 贪婪恐惧指数响应失败，原始响应: {}", response_text))?;

        let data = alt_response.data.first()
            .ok_or_else(|| anyhow::anyhow!("贪婪恐惧指数数据为空"))?;
        
        let value = data.value.parse::<u8>()
            .context("解析贪婪恐惧指数值失败")?;
        
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

    /// 获取山寨币季节指数
    /// 
    /// 通过CMC 100指数API计算山寨币季节指数
    /// 
    /// # 返回
    /// * `Result<AltcoinSeasonIndex>` - 山寨币季节指数数据或错误
    pub async fn get_altcoin_season_index(&self) -> Result<AltcoinSeasonIndex> {
        info!("🪙 开始获取山寨币季节指数（基于CMC 100指数）");
        
        // 获取CMC 100指数数据
        let cmc_data = self.get_cmc_100_index().await?;
        
        // 计算山寨币季节指数
        let altcoin_index = self.calculate_altcoin_season_from_cmc(&cmc_data).await?;
        
        info!("✅ 山寨币季节指数计算成功: {} - {}", 
              altcoin_index.value, 
              altcoin_index.classification_zh);

        Ok(altcoin_index)
    }

    /// 获取CMC 100指数数据
    /// 
    /// # 返回
    /// * `Result<Vec<CmcIndexData>>` - CMC 100指数数据或错误
    async fn get_cmc_100_index(&self) -> Result<Vec<CmcIndexData>> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("需要API密钥来访问CMC 100指数"))?;
        
        let url = format!("{}/v1/cryptocurrency/listings/latest", self.base_url);
        
        debug!("🌐 请求CMC 100指数URL: {}", url);
        
        let response = self.client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "identity")
            .query(&[
                ("start", "1"),
                ("limit", "100"),
                ("convert", "USD"),
                ("sort", "market_cap"),
                ("sort_dir", "desc"),
                ("cryptocurrency_type", "all"),
                ("tag", "all"),
            ])
            .send()
            .await
            .context("发送CMC 100指数请求失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(anyhow::anyhow!(
                "CMC API请求失败: HTTP {} - {}", 
                status, 
                error_text
            ));
        }

        let response_text = response.text().await
            .context("读取CMC响应内容失败")?;
        
        debug!("📥 CMC API响应长度: {} 字符", response_text.len());
        debug!("📄 CMC API原始响应前500字符: {}", &response_text[..response_text.len().min(500)]);

        let cmc_response: Cmc100Response = serde_json::from_str(&response_text)
            .with_context(|| format!("解析CMC 100指数响应失败，响应前500字符: {}", &response_text[..response_text.len().min(500)]))?;

        if cmc_response.status.error_code != 0 {
            return Err(anyhow::anyhow!(
                "CMC API错误: {} - {}", 
                cmc_response.status.error_code,
                cmc_response.status.error_message.unwrap_or("未知错误".to_string())
            ));
        }

        info!("✅ CMC 100指数数据获取成功，共 {} 个币种", cmc_response.data.len());
        Ok(cmc_response.data)
    }

    /// 基于CMC数据计算山寨币季节指数
    /// 
    /// # 参数
    /// * `cmc_data` - CMC 100指数数据
    /// 
    /// # 返回
    /// * `Result<AltcoinSeasonIndex>` - 山寨币季节指数
    async fn calculate_altcoin_season_from_cmc(&self, cmc_data: &[CmcIndexData]) -> Result<AltcoinSeasonIndex> {
        info!("🧮 开始计算山寨币季节指数");
        
        // 找到比特币数据
        let bitcoin = cmc_data.iter()
            .find(|coin| coin.symbol == "BTC")
            .ok_or_else(|| anyhow::anyhow!("未找到比特币数据"))?;
        
        let btc_change_24h = bitcoin.quote.get("USD")
            .map(|q| q.percent_change_24h)
            .unwrap_or(0.0);
        
        info!("📊 比特币24小时变化: {:.2}%", btc_change_24h);
        
        // 计算表现优于比特币的币种数量（排除比特币本身）
        let mut outperforming_count = 0;
        let mut total_count = 0;
        
        for coin in cmc_data.iter() {
            if coin.symbol == "BTC" {
                continue; // 跳过比特币本身
            }
            
            if let Some(usd_quote) = coin.quote.get("USD") {
                total_count += 1;
                if usd_quote.percent_change_24h > btc_change_24h {
                    outperforming_count += 1;
                }
            }
        }
        
        // 计算百分比
        let outperforming_percentage = if total_count > 0 {
            (outperforming_count as f32 / total_count as f32) * 100.0
        } else {
            0.0
        };
        
        // 计算指数值（0-100）
        let index_value = outperforming_percentage.round() as u8;
        
        info!("📈 山寨币表现统计: {}/{} 币种表现优于比特币 ({:.1}%)", 
              outperforming_count, total_count, outperforming_percentage);
        
        let altcoin_index = AltcoinSeasonIndex {
            value: index_value,
            classification: Self::get_altcoin_season_classification(index_value).to_string(),
            classification_zh: Self::get_altcoin_season_classification_zh(index_value).to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            outperforming_count: outperforming_count as u8,
            total_count: total_count as u8,
            outperforming_percentage,
            market_advice: Self::get_altcoin_season_advice(index_value).to_string(),
        };

        Ok(altcoin_index)
    }

    /// 获取单个加密货币数据
    /// 
    /// # 参数
    /// * `symbol` - 币种符号（如"HYPE"）
    /// 
    /// # 返回
    /// * `Result<CryptocurrencyData>` - 币种数据或错误
    pub async fn get_cryptocurrency_data(&self, symbol: &str) -> Result<CryptocurrencyData> {
        info!("💰 开始获取 {} 币种数据", symbol);
        
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("需要API密钥来获取币种数据"))?;
        
        let url = format!("{}/v1/cryptocurrency/quotes/latest", self.base_url);
        
        debug!("🌐 请求币种数据URL: {}", url);
        
        let response = self.client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .header("Accept", "application/json")
            .header("Accept-Encoding", "identity")
            .query(&[
                ("symbol", symbol),
                ("convert", "USD"),
            ])
            .send()
            .await
            .context("发送币种数据请求失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(anyhow::anyhow!(
                "币种数据API请求失败: HTTP {} - {}", 
                status, 
                error_text
            ));
        }

        let response_text = response.text().await
            .context("读取币种数据响应内容失败")?;
        
        debug!("📥 币种数据API响应长度: {} 字符", response_text.len());

        let crypto_response: CryptocurrencyResponse = serde_json::from_str(&response_text)
            .context("解析币种数据响应失败")?;

        if crypto_response.status.error_code != 0 {
            return Err(anyhow::anyhow!(
                "币种数据API错误: {} - {}", 
                crypto_response.status.error_code,
                crypto_response.status.error_message.unwrap_or("未知错误".to_string())
            ));
        }

        // 获取币种数据
        let crypto_info = crypto_response.data
            .get(symbol)
            .ok_or_else(|| anyhow::anyhow!("未找到 {} 币种数据", symbol))?;

        let usd_quote = crypto_info.quote
            .get("USD")
            .ok_or_else(|| anyhow::anyhow!("未找到USD报价数据"))?;

        let crypto_data = CryptocurrencyData {
            id: crypto_info.id,
            name: crypto_info.name.clone(),
            symbol: crypto_info.symbol.clone(),
            price: usd_quote.price,
            market_cap: usd_quote.market_cap,
            volume_24h: usd_quote.volume_24h,
            percent_change_24h: usd_quote.percent_change_24h,
            percent_change_7d: usd_quote.percent_change_7d,
            cmc_rank: crypto_info.cmc_rank,
            last_updated: crypto_info.last_updated.clone(),
        };

        info!("✅ {} 币种数据获取成功: ${:.4}", symbol, crypto_data.price);
        Ok(crypto_data)
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

    /// 获取山寨币季节指数分类（英文）
    /// 
    /// # 参数
    /// * `value` - 指数值 (0-100)
    /// 
    /// # 返回
    /// * `&str` - 英文分类
    pub fn get_altcoin_season_classification(value: u8) -> &'static str {
        match value {
            0..=25 => "Bitcoin Season",
            26..=74 => "Balanced Market", 
            75..=100 => "Altcoin Season",
            _ => "Unknown",
        }
    }

    /// 获取山寨币季节指数分类（中文）
    /// 
    /// # 参数
    /// * `value` - 指数值 (0-100)
    /// 
    /// # 返回
    /// * `&str` - 中文分类
    pub fn get_altcoin_season_classification_zh(value: u8) -> &'static str {
        match value {
            0..=25 => "比特币季节",
            26..=74 => "平衡市场",
            75..=100 => "山寨币季节",
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

    /// 获取山寨币季节指数的市场建议
    /// 
    /// # 参数
    /// * `value` - 指数值 (0-100)
    /// 
    /// # 返回
    /// * `&str` - 市场建议
    pub fn get_altcoin_season_advice(value: u8) -> &'static str {
        match value {
            0..=25 => "比特币表现强劲，关注比特币投资机会",
            26..=49 => "市场相对平衡，可考虑比特币和优质山寨币组合",
            50..=74 => "山寨币开始活跃，可适当增加山寨币配置",
            75..=100 => "山寨币季节，山寨币表现优异，注意风险管理",
            _ => "市场情况未明，建议谨慎投资",
        }
    }
} 

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_altcoin_season_classification() {
        // 测试比特币季节
        assert_eq!(CoinMarketCapClient::get_altcoin_season_classification(20), "Bitcoin Season");
        assert_eq!(CoinMarketCapClient::get_altcoin_season_classification_zh(20), "比特币季节");
        
        // 测试平衡市场
        assert_eq!(CoinMarketCapClient::get_altcoin_season_classification(50), "Balanced Market");
        assert_eq!(CoinMarketCapClient::get_altcoin_season_classification_zh(50), "平衡市场");
        
        // 测试山寨币季节
        assert_eq!(CoinMarketCapClient::get_altcoin_season_classification(80), "Altcoin Season");
        assert_eq!(CoinMarketCapClient::get_altcoin_season_classification_zh(80), "山寨币季节");
    }

    #[test]
    fn test_altcoin_season_advice() {
        assert_eq!(CoinMarketCapClient::get_altcoin_season_advice(20), "比特币表现强劲，关注比特币投资机会");
        assert_eq!(CoinMarketCapClient::get_altcoin_season_advice(40), "市场相对平衡，可考虑比特币和优质山寨币组合");
        assert_eq!(CoinMarketCapClient::get_altcoin_season_advice(60), "山寨币开始活跃，可适当增加山寨币配置");
        assert_eq!(CoinMarketCapClient::get_altcoin_season_advice(85), "山寨币季节，山寨币表现优异，注意风险管理");
    }

    #[tokio::test]
    async fn test_altcoin_season_index_structure() {
        // 测试AltcoinSeasonIndex结构体的创建
        let index = AltcoinSeasonIndex {
            value: 48,
            classification: "Balanced Market".to_string(),
            classification_zh: "平衡市场".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            outperforming_count: 48,
            total_count: 100,
            outperforming_percentage: 48.0,
            market_advice: "市场情况未明，建议谨慎投资".to_string(),
        };

        assert_eq!(index.value, 48);
        assert_eq!(index.classification, "Balanced Market");
        assert_eq!(index.classification_zh, "平衡市场");
    }
}

/// 自定义反序列化函数，处理字符串或数字类型的error_code
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct StringOrNumberVisitor;

    impl<'de> Visitor<'de> for StringOrNumberVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u64>().map_err(de::Error::custom)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value >= 0 {
                Ok(value as u64)
            } else {
                Err(de::Error::custom("negative number not allowed"))
            }
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}