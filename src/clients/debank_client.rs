use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, debug, error};
use std::time::Duration;

use super::{ApiClient, HttpClientBuilder};

/// DeBank API客户端
/// 
/// 用于与DeBank API进行交互
/// 支持获取钱包资产、交易历史等DeFi数据
pub struct DeBankClient {
    /// HTTP客户端
    client: reqwest::Client,
    /// API密钥（可选）
    api_key: Option<String>,
    /// API基础URL
    base_url: String,
    /// 超时时间
    timeout: Duration,
}

impl DeBankClient {
    /// 创建新的DeBank客户端
    /// 
    /// # 参数
    /// * `api_key` - DeBank API密钥（可选）
    /// * `timeout` - HTTP超时时间
    /// 
    /// # 返回
    /// * `Result<Self>` - 创建的客户端或错误
    pub fn new(api_key: Option<String>, timeout: Duration) -> Result<Self> {
        let client = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-DeBankClient/1.0")
            .build()?;
        
        Ok(Self {
            client,
            api_key,
            base_url: "https://openapi.debank.com".to_string(),
            timeout,
        })
    }
    
    /// 获取钱包资产总览
    /// 
    /// # 参数
    /// * `address` - 钱包地址
    /// 
    /// # 返回
    /// * `Result<Value>` - 资产总览或错误
    pub async fn get_wallet_balance(&self, address: &str) -> Result<Value> {
        let url = format!("{}/v1/user/total_balance", self.base_url);
        
        debug!("💰 正在获取DeBank钱包资产: {}", address);
        
        let mut request = self.client
            .get(&url)
            .query(&[("id", address)]);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("AccessKey", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送DeBank请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ DeBank API请求失败: {} - {}", status, text);
            return Err(anyhow!("DeBank API请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析DeBank响应失败")?;
        
        info!("✅ 获取DeBank钱包资产成功: {}", address);
        
        Ok(result)
    }
    
    /// 获取钱包代币列表
    /// 
    /// # 参数
    /// * `address` - 钱包地址
    /// 
    /// # 返回
    /// * `Result<Value>` - 代币列表或错误
    pub async fn get_wallet_tokens(&self, address: &str) -> Result<Value> {
        let url = format!("{}/v1/user/token_list", self.base_url);
        
        debug!("🪙 正在获取DeBank钱包代币: {}", address);
        
        let mut request = self.client
            .get(&url)
            .query(&[("id", address)]);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("AccessKey", api_key);
        }
        
        let response = request
            .send()
            .await
            .context("发送DeBank代币请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ DeBank代币请求失败: {} - {}", status, text);
            return Err(anyhow!("DeBank代币请求失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析DeBank代币响应失败")?;
        
        info!("✅ 获取DeBank钱包代币成功: {}", address);
        
        Ok(result)
    }
}

#[async_trait::async_trait]
impl ApiClient for DeBankClient {
    fn source_name(&self) -> &str {
        "debank"
    }
    
    async fn check_api_key(&self) -> Result<bool> {
        // 尝试获取一个测试地址的资产来验证API密钥
        let test_address = "0x0000000000000000000000000000000000000000";
        match self.get_wallet_balance(test_address).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    async fn fetch_raw_data(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);
        
        let mut request = self.client.get(&url);
        
        // 如果有API密钥，添加到请求头
        if let Some(api_key) = &self.api_key {
            request = request.header("AccessKey", api_key);
        }
        
        let response = request
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("DeBank API请求失败: {} - {}", status, text));
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        if let Ok(client) = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-DeBankClient/1.0")
            .build() {
            self.client = client;
        }
    }
} 