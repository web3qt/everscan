use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, debug, error};
use std::time::Duration;

use super::{ApiClient, HttpClientBuilder};

/// Glassnode API客户端
/// 
/// 用于与Glassnode API进行交互
/// 支持获取链上数据指标
pub struct GlassnodeClient {
    /// HTTP客户端
    client: reqwest::Client,
    /// API密钥
    api_key: String,
    /// API基础URL
    base_url: String,
    /// 超时时间
    timeout: Duration,
}

impl GlassnodeClient {
    /// 创建新的Glassnode客户端
    /// 
    /// # 参数
    /// * `api_key` - Glassnode API密钥
    /// * `timeout` - HTTP超时时间
    /// 
    /// # 返回
    /// * `Result<Self>` - 创建的客户端或错误
    pub fn new(api_key: impl Into<String>, timeout: Duration) -> Result<Self> {
        let client = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-GlassnodeClient/1.0")
            .build()?;
        
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: "https://api.glassnode.com/v1".to_string(),
            timeout,
        })
    }
    
    /// 获取指标数据
    /// 
    /// # 参数
    /// * `metric` - 指标名称
    /// * `asset` - 资产符号
    /// * `since` - 开始时间戳（可选）
    /// * `until` - 结束时间戳（可选）
    /// 
    /// # 返回
    /// * `Result<Value>` - 指标数据或错误
    pub async fn get_metric(&self, metric: &str, asset: &str, since: Option<i64>, until: Option<i64>) -> Result<Value> {
        let url = format!("{}/metrics/{}", self.base_url, metric);
        
        debug!("📊 正在获取Glassnode指标: {} (资产: {})", metric, asset);
        
        let mut request = self.client
            .get(&url)
            .query(&[("a", asset), ("api_key", &self.api_key)]);
        
        // 添加时间范围参数
        if let Some(since) = since {
            request = request.query(&[("s", &since.to_string())]);
        }
        
        if let Some(until) = until {
            request = request.query(&[("u", &until.to_string())]);
        }
        
        let response = request
            .send()
            .await
            .context("发送Glassnode请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ Glassnode API请求失败: {} - {}", status, text);
            return Err(anyhow!("Glassnode API请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析Glassnode响应失败")?;
        
        info!("✅ 获取Glassnode指标成功: {}", metric);
        
        Ok(result)
    }
}

#[async_trait::async_trait]
impl ApiClient for GlassnodeClient {
    fn source_name(&self) -> &str {
        "glassnode"
    }
    
    async fn check_api_key(&self) -> Result<bool> {
        // 尝试获取一个简单的指标来验证API密钥
        match self.get_metric("addresses/active_count", "BTC", None, None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    async fn fetch_raw_data(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);
        
        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.api_key)])
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Glassnode API请求失败: {} - {}", status, text));
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        if let Ok(client) = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-GlassnodeClient/1.0")
            .build() {
            self.client = client;
        }
    }
} 