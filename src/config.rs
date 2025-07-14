use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use tracing::info;

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Web服务器配置
    pub server: ServerConfig,
    /// 数据源配置
    pub data_sources: DataSourcesConfig,
    /// 监控币种配置
    pub monitoring: MonitoringConfig,
}

/// Web服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 服务器监听地址
    pub host: String,
    /// 服务器监听端口
    pub port: u16,
}

/// 数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourcesConfig {
    /// CoinMarketCap配置
    pub coinmarketcap: ApiConfig,
    /// Glassnode配置（预留）
    pub glassnode: ApiConfig,
    /// DeBankAPI配置（预留）
    pub debank: ApiConfig,
    /// DuneAPI配置（预留）
    pub dune: ApiConfig,
}

/// API配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API密钥
    pub api_key: Option<String>,
    /// 请求间隔（毫秒）
    pub request_interval_ms: u64,
    /// 请求超时时间（秒）
    pub timeout_seconds: u64,
}

/// 监控币种配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 需要监控的币种ID列表
    pub coins: Vec<String>,
    /// 数据更新间隔（秒）
    pub update_interval_seconds: u64,
}

impl AppConfig {
    /// 从配置文件加载配置
    /// 
    /// # 参数
    /// * `config_path` - 配置文件路径
    /// 
    /// # 返回
    /// * `Result<Self>` - 配置实例或错误
    pub fn from_file(config_path: &str) -> Result<Self> {
        info!("📖 正在加载配置文件: {}", config_path);
        
        let content = fs::read_to_string(config_path)
            .with_context(|| format!("无法读取配置文件: {}", config_path))?;
        
        let mut config: AppConfig = toml::from_str(&content)
            .with_context(|| format!("无法解析配置文件: {}", config_path))?;
        
        // 从环境变量覆盖配置
        config.override_from_env()?;
        
        info!("✅ 配置文件加载成功");
        Ok(config)
    }
    
    /// 从环境变量覆盖配置
    fn override_from_env(&mut self) -> Result<()> {
        // 服务器配置
        if let Ok(host) = env::var("SERVER_HOST") {
            self.server.host = host;
        }
        
        if let Ok(port) = env::var("SERVER_PORT") {
            self.server.port = port.parse()
                .context("解析SERVER_PORT失败")?;
        }
        
        // API密钥配置        
        if let Ok(api_key) = env::var("COINMARKETCAP_API_KEY") {
            self.data_sources.coinmarketcap.api_key = Some(api_key);
        }
        
        if let Ok(api_key) = env::var("GLASSNODE_API_KEY") {
            self.data_sources.glassnode.api_key = Some(api_key);
        }
        
        if let Ok(api_key) = env::var("DEBANK_API_KEY") {
            self.data_sources.debank.api_key = Some(api_key);
        }
        
        if let Ok(api_key) = env::var("DUNE_API_KEY") {
            self.data_sources.dune.api_key = Some(api_key);
        }
        
        Ok(())
    }
    
    /// 创建默认配置
    pub fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
            },
            data_sources: DataSourcesConfig {
                coinmarketcap: ApiConfig {
                    api_key: None,
                    request_interval_ms: 1000,
                    timeout_seconds: 30,
                },
                glassnode: ApiConfig {
                    api_key: None,
                    request_interval_ms: 1000,
                    timeout_seconds: 30,
                },
                debank: ApiConfig {
                    api_key: None,
                    request_interval_ms: 1000,
                    timeout_seconds: 30,
                },
                dune: ApiConfig {
                    api_key: None,
                    request_interval_ms: 1000,
                    timeout_seconds: 30,
                },
            },
            monitoring: MonitoringConfig {
                coins: vec!["hyperliquid".to_string()],
                update_interval_seconds: 14400, // 4小时
            },
        }
    }
} 