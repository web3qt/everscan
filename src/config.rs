use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use anyhow::{Result, Context};
use std::fs;

/// 应用程序配置结构
/// 
/// 包含所有必要的配置信息：
/// - 数据库连接信息
/// - 各个API的密钥
/// - 任务调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 数据库配置
    pub database: DatabaseConfig,
    /// API密钥配置
    pub api_keys: ApiKeys,
    /// 任务调度配置
    pub tasks: TasksConfig,
    /// 应用程序配置
    pub app: AppConfig,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库连接URL
    pub url: String,
    /// 最大连接数
    pub max_connections: u32,
    /// 连接超时时间（秒）
    pub timeout_seconds: u64,
}

/// API密钥配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeys {
    /// Dune Analytics API密钥
    pub dune_api_key: Option<String>,
    /// Glassnode API密钥
    pub glassnode_api_key: Option<String>,
    /// DeBank API密钥
    pub debank_api_key: Option<String>,
    /// CoinGecko API密钥（可选，有免费额度）
    pub coingecko_api_key: Option<String>,
    /// Arkham Intelligence API密钥
    pub arkham_api_key: Option<String>,
}

/// 任务调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    /// 任务执行间隔配置（任务名 -> 间隔秒数）
    pub intervals: HashMap<String, u64>,
    /// 任务重试次数
    pub retry_count: u32,
    /// 任务超时时间（秒）
    pub timeout_seconds: u64,
}

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 应用程序名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 日志级别
    pub log_level: String,
    /// HTTP客户端超时时间（秒）
    pub http_timeout_seconds: u64,
}

impl Config {
    /// 加载配置
    /// 
    /// 优先级：
    /// 1. 环境变量
    /// 2. config.toml文件
    /// 3. 默认值
    pub async fn load() -> Result<Self> {
        // 首先尝试从config.toml文件加载
        let config = if let Ok(config_str) = fs::read_to_string("config.toml") {
            toml::from_str(&config_str)
                .context("解析config.toml文件失败")?
        } else {
            // 如果没有配置文件，使用默认配置
            Self::default()
        };
        
        // 然后从环境变量覆盖配置
        let config = Self::override_from_env(config)?;
        
        Ok(config)
    }
    
    /// 从环境变量覆盖配置
    fn override_from_env(mut config: Self) -> Result<Self> {
        // 数据库配置
        if let Ok(db_url) = env::var("DATABASE_URL") {
            config.database.url = db_url;
        }
        
        if let Ok(max_conn) = env::var("DATABASE_MAX_CONNECTIONS") {
            config.database.max_connections = max_conn.parse()
                .context("解析DATABASE_MAX_CONNECTIONS失败")?;
        }
        
        // API密钥配置
        if let Ok(key) = env::var("DUNE_API_KEY") {
            config.api_keys.dune_api_key = Some(key);
        }
        
        if let Ok(key) = env::var("GLASSNODE_API_KEY") {
            config.api_keys.glassnode_api_key = Some(key);
        }
        
        if let Ok(key) = env::var("DEBANK_API_KEY") {
            config.api_keys.debank_api_key = Some(key);
        }
        
        if let Ok(key) = env::var("COINGECKO_API_KEY") {
            config.api_keys.coingecko_api_key = Some(key);
        }
        
        if let Ok(key) = env::var("ARKHAM_API_KEY") {
            config.api_keys.arkham_api_key = Some(key);
        }
        
        // 应用程序配置
        if let Ok(log_level) = env::var("RUST_LOG") {
            config.app.log_level = log_level;
        }
        
        Ok(config)
    }
}

impl Default for Config {
    /// 提供默认配置
    fn default() -> Self {
        let mut task_intervals = HashMap::new();
        task_intervals.insert("dune".to_string(), 3600); // 1小时
        task_intervals.insert("glassnode".to_string(), 3600); // 1小时
        task_intervals.insert("debank".to_string(), 1800); // 30分钟
        task_intervals.insert("coingecko".to_string(), 300); // 5分钟
        task_intervals.insert("arkham".to_string(), 3600); // 1小时
        
        Self {
            database: DatabaseConfig {
                url: "postgresql://localhost/everscan".to_string(),
                max_connections: 10,
                timeout_seconds: 30,
            },
            api_keys: ApiKeys {
                dune_api_key: None,
                glassnode_api_key: None,
                debank_api_key: None,
                coingecko_api_key: None,
                arkham_api_key: None,
            },
            tasks: TasksConfig {
                intervals: task_intervals,
                retry_count: 3,
                timeout_seconds: 300,
            },
            app: AppConfig {
                name: "EverScan".to_string(),
                version: "0.1.0".to_string(),
                log_level: "info".to_string(),
                http_timeout_seconds: 30,
            },
        }
    }
} 