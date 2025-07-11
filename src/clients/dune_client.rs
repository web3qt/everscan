use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, debug, error, warn};
use std::time::Duration;
use std::collections::HashMap;

use super::{ApiClient, HttpClientBuilder};

/// Dune Analytics API客户端
/// 
/// 用于与Dune Analytics API进行交互
/// 支持执行查询、获取查询结果等操作
pub struct DuneClient {
    /// HTTP客户端
    client: reqwest::Client,
    /// API密钥
    api_key: String,
    /// API基础URL
    base_url: String,
    /// 超时时间
    timeout: Duration,
}

/// Dune查询执行请求
#[derive(Debug, Clone, Serialize)]
pub struct DuneQueryRequest {
    /// 查询ID
    pub query_id: u32,
    /// 查询参数（可选）
    pub parameters: Option<HashMap<String, Value>>,
}

/// Dune查询执行响应
#[derive(Debug, Clone, Deserialize)]
pub struct DuneQueryResponse {
    /// 执行ID
    pub execution_id: String,
    /// 状态
    pub state: String,
}

/// Dune查询结果响应
#[derive(Debug, Clone, Deserialize)]
pub struct DuneResultResponse {
    /// 执行ID
    pub execution_id: String,
    /// 查询ID
    pub query_id: u32,
    /// 状态
    pub state: String,
    /// 结果数据
    pub result: Option<DuneResultData>,
}

/// Dune结果数据
#[derive(Debug, Clone, Deserialize)]
pub struct DuneResultData {
    /// 行数据
    pub rows: Vec<Value>,
    /// 元数据
    pub metadata: DuneResultMetadata,
}

/// Dune结果元数据
#[derive(Debug, Clone, Deserialize)]
pub struct DuneResultMetadata {
    /// 列信息
    pub column_names: Vec<String>,
    /// 行数
    pub row_count: u32,
    /// 结果集ID
    pub result_set_bytes: Option<u64>,
    /// 总行数
    pub total_row_count: Option<u32>,
}

impl DuneClient {
    /// 创建新的Dune客户端
    /// 
    /// # 参数
    /// * `api_key` - Dune API密钥
    /// * `timeout` - HTTP超时时间
    /// 
    /// # 返回
    /// * `Result<Self>` - 创建的客户端或错误
    pub fn new(api_key: impl Into<String>, timeout: Duration) -> Result<Self> {
        let client = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-DuneClient/1.0")
            .build()?;
        
        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: "https://api.dune.com/api/v1".to_string(),
            timeout,
        })
    }
    
    /// 执行Dune查询
    /// 
    /// # 参数
    /// * `query_id` - 查询ID
    /// * `parameters` - 查询参数（可选）
    /// 
    /// # 返回
    /// * `Result<DuneQueryResponse>` - 查询响应或错误
    pub async fn execute_query(&self, query_id: u32, parameters: Option<HashMap<String, Value>>) -> Result<DuneQueryResponse> {
        let url = format!("{}/query/{}/execute", self.base_url, query_id);
        
        debug!("🔍 正在执行Dune查询: {}", query_id);
        
        let mut request = self.client
            .post(&url)
            .header("X-DUNE-API-KEY", &self.api_key)
            .header("Content-Type", "application/json");
        
        // 如果有参数，添加到请求体中
        if let Some(params) = parameters {
            request = request.json(&serde_json::json!({
                "query_parameters": params
            }));
        }
        
        let response = request
            .send()
            .await
            .context("发送Dune查询请求失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ Dune API请求失败: {} - {}", status, text);
            return Err(anyhow!("Dune API请求失败: {} - {}", status, text));
        }
        
        let result: DuneQueryResponse = response
            .json()
            .await
            .context("解析Dune查询响应失败")?;
        
        info!("✅ Dune查询执行成功: {} (执行ID: {})", query_id, result.execution_id);
        
        Ok(result)
    }
    
    /// 获取查询结果
    /// 
    /// # 参数
    /// * `execution_id` - 执行ID
    /// 
    /// # 返回
    /// * `Result<DuneResultResponse>` - 查询结果或错误
    pub async fn get_query_result(&self, execution_id: &str) -> Result<DuneResultResponse> {
        let url = format!("{}/execution/{}/results", self.base_url, execution_id);
        
        debug!("📊 正在获取Dune查询结果: {}", execution_id);
        
        let response = self.client
            .get(&url)
            .header("X-DUNE-API-KEY", &self.api_key)
            .send()
            .await
            .context("获取Dune查询结果失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ 获取Dune查询结果失败: {} - {}", status, text);
            return Err(anyhow!("获取Dune查询结果失败: {} - {}", status, text));
        }
        
        let result: DuneResultResponse = response
            .json()
            .await
            .context("解析Dune结果响应失败")?;
        
        debug!("📊 Dune查询结果获取成功: {} (状态: {})", execution_id, result.state);
        
        Ok(result)
    }
    
    /// 执行查询并等待结果
    /// 
    /// # 参数
    /// * `query_id` - 查询ID
    /// * `parameters` - 查询参数（可选）
    /// * `max_wait_time` - 最大等待时间
    /// 
    /// # 返回
    /// * `Result<DuneResultResponse>` - 查询结果或错误
    pub async fn execute_and_wait(&self, query_id: u32, parameters: Option<HashMap<String, Value>>, max_wait_time: Duration) -> Result<DuneResultResponse> {
        // 执行查询
        let exec_response = self.execute_query(query_id, parameters).await?;
        
        // 等待结果
        let start_time = std::time::Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        
        loop {
            if start_time.elapsed() > max_wait_time {
                warn!("⏰ Dune查询超时: {}", query_id);
                return Err(anyhow!("Dune查询超时"));
            }
            
            interval.tick().await;
            
            let result = self.get_query_result(&exec_response.execution_id).await?;
            
            match result.state.as_str() {
                "QUERY_STATE_COMPLETED" => {
                    info!("✅ Dune查询完成: {}", query_id);
                    return Ok(result);
                }
                "QUERY_STATE_FAILED" => {
                    error!("❌ Dune查询失败: {}", query_id);
                    return Err(anyhow!("Dune查询失败"));
                }
                "QUERY_STATE_CANCELLED" => {
                    warn!("🚫 Dune查询被取消: {}", query_id);
                    return Err(anyhow!("Dune查询被取消"));
                }
                _ => {
                    debug!("⏳ Dune查询进行中: {} (状态: {})", query_id, result.state);
                    continue;
                }
            }
        }
    }
    
    /// 获取查询的最新结果（缓存结果）
    /// 
    /// # 参数
    /// * `query_id` - 查询ID
    /// 
    /// # 返回
    /// * `Result<DuneResultResponse>` - 查询结果或错误
    pub async fn get_latest_result(&self, query_id: u32) -> Result<DuneResultResponse> {
        let url = format!("{}/query/{}/results", self.base_url, query_id);
        
        debug!("📊 正在获取Dune查询最新结果: {}", query_id);
        
        let response = self.client
            .get(&url)
            .header("X-DUNE-API-KEY", &self.api_key)
            .send()
            .await
            .context("获取Dune查询最新结果失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ 获取Dune查询最新结果失败: {} - {}", status, text);
            return Err(anyhow!("获取Dune查询最新结果失败: {} - {}", status, text));
        }
        
        let result: DuneResultResponse = response
            .json()
            .await
            .context("解析Dune最新结果响应失败")?;
        
        info!("✅ Dune查询最新结果获取成功: {} (行数: {})", 
              query_id, 
              result.result.as_ref().map(|r| r.metadata.row_count).unwrap_or(0));
        
        Ok(result)
    }
    
    /// 获取查询状态
    /// 
    /// # 参数
    /// * `execution_id` - 执行ID
    /// 
    /// # 返回
    /// * `Result<String>` - 查询状态或错误
    pub async fn get_query_status(&self, execution_id: &str) -> Result<String> {
        let url = format!("{}/execution/{}/status", self.base_url, execution_id);
        
        let response = self.client
            .get(&url)
            .header("X-DUNE-API-KEY", &self.api_key)
            .send()
            .await
            .context("获取Dune查询状态失败")?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("获取Dune查询状态失败: {} - {}", status, text));
        }
        
        let result: Value = response
            .json()
            .await
            .context("解析Dune状态响应失败")?;
        
        let state = result["state"].as_str().unwrap_or("UNKNOWN").to_string();
        
        Ok(state)
    }
}

#[async_trait::async_trait]
impl ApiClient for DuneClient {
    fn source_name(&self) -> &str {
        "dune"
    }
    
    async fn check_api_key(&self) -> Result<bool> {
        // 尝试获取一个简单的查询结果来验证API密钥
        let url = format!("{}/query/1/results", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("X-DUNE-API-KEY", &self.api_key)
            .send()
            .await?;
        
        // 如果返回401，说明API密钥无效
        // 如果返回其他状态码，说明API密钥有效（可能是其他错误）
        Ok(response.status() != 401)
    }
    
    async fn fetch_raw_data(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);
        
        let response = self.client
            .get(&url)
            .header("X-DUNE-API-KEY", &self.api_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Dune API请求失败: {} - {}", status, text));
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        // 重新构建客户端
        if let Ok(client) = HttpClientBuilder::new()
            .timeout(timeout)
            .user_agent("EverScan-DuneClient/1.0")
            .build() {
            self.client = client;
        }
    }
} 