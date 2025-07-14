// pub mod bitget_client; // 已移除Bitget客户端
// pub mod dune_client;
// pub mod glassnode_client;
// pub mod debank_client;
pub mod coinmarketcap_client; // CoinMarketCap客户端

// pub use bitget_client::*; // 已移除
// pub use dune_client::*;
// pub use glassnode_client::*;
// pub use debank_client::*;
pub use coinmarketcap_client::*; // 导出CoinMarketCap客户端


use anyhow::Result;
use serde_json::Value;
use std::time::Duration;

/// 通用API客户端trait
/// 
/// 定义所有数据源客户端的通用接口
#[async_trait::async_trait]
pub trait ApiClient {
    /// 获取数据源名称
    fn source_name(&self) -> &str;
    
    /// 检查API密钥是否有效
    async fn check_api_key(&self) -> Result<bool>;
    
    /// 获取原始数据
    async fn fetch_raw_data(&self, endpoint: &str) -> Result<Value>;
    
    /// 设置HTTP超时时间
    fn set_timeout(&mut self, timeout: Duration);
}

/// HTTP客户端构建器
/// 
/// 用于创建配置好的HTTP客户端
pub struct HttpClientBuilder {
    timeout: Duration,
    user_agent: String,
}

impl HttpClientBuilder {
    /// 创建新的HTTP客户端构建器
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            user_agent: "EverScan/1.0".to_string(),
        }
    }
    
    /// 设置超时时间
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// 设置用户代理
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }
    
    /// 构建HTTP客户端
    pub fn build(self) -> Result<reqwest::Client> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .user_agent(self.user_agent)
            .build()?;
        
        Ok(client)
    }
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
} 