use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, debug, error, warn};

use crate::config::Config;
use crate::storage::PostgresRepository;
use crate::tasks::TaskManager;
use crate::clients::*;
use crate::tasks::*;
use crate::web::{WebServer, cache::DataCache}; // 新增：导入Web服务和数据缓存

/// 任务调度器
/// 
/// 负责管理所有数据获取任务的生命周期
/// 包括初始化、调度和监控，以及Web服务器启动
pub struct Orchestrator {
    /// 配置
    config: Config,
    /// 存储仓库
    storage: Arc<PostgresRepository>,
    /// 任务管理器
    task_manager: TaskManager,
    /// 数据缓存（新增）
    cache: Arc<DataCache>,
    /// Web服务器（新增）
    web_server: WebServer,
}

impl Orchestrator {
    /// 创建新的任务调度器
    /// 
    /// # 参数
    /// * `config` - 应用配置
    /// 
    /// # 返回
    /// * `Result<Self>` - 调度器实例或错误
    pub async fn new(config: Config) -> Result<Self> {
        info!("🔧 正在初始化任务调度器");
        
        // 初始化存储仓库
        let storage = Arc::new(
            PostgresRepository::new(&config.database)
                .await
                .context("初始化数据库连接失败")?
        );
        
        // 初始化数据缓存（新增）
        let cache = Arc::new(DataCache::new());
        
        // 创建任务管理器
        let mut task_manager = TaskManager::new();
        
        // 注册所有任务
        Self::register_tasks(&mut task_manager, &config, storage.clone(), cache.clone()).await?;
        
        // 创建Web服务器（新增）
        let web_server = WebServer::new(
            config.clone(),
            cache.clone(),
            Some(storage.clone()),
        );
        
        info!("✅ 任务调度器初始化完成，共注册 {} 个任务", task_manager.get_tasks().len());
        
        Ok(Self {
            config,
            storage,
            task_manager,
            cache,
            web_server,
        })
    }
    
    /// 注册所有任务
    /// 
    /// # 参数
    /// * `task_manager` - 任务管理器
    /// * `config` - 应用配置
    /// * `storage` - 存储仓库
    /// * `cache` - 数据缓存（新增）
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn register_tasks(
        task_manager: &mut TaskManager, 
        config: &Config, 
        storage: Arc<PostgresRepository>,
        cache: Arc<DataCache>, // 新增参数
    ) -> Result<()> {
        info!("📋 开始注册任务");
        
        // 注册加密货币市场数据任务
        let coingecko_client = Arc::new(CoinGeckoClient::new(
            config.api_keys.coingecko_api_key.clone(),
            Duration::from_secs(30)
        )?);
        
        // 从配置文件获取要监控的代币列表
        let monitored_coins = if let Some(crypto_config) = &config.crypto_monitoring {
            if crypto_config.coins.is_empty() {
                // 如果配置为空，使用默认币种
                warn!("⚠️ 配置文件中的币种列表为空，使用默认币种");
                vec![
                    "bitcoin".to_string(),
                    "hyperliquid".to_string(),
                ]
            } else {
                crypto_config.coins.clone()
            }
        } else {
            // 如果没有配置，使用默认币种
            warn!("⚠️ 未找到加密货币监控配置，使用默认币种");
            vec![
                "bitcoin".to_string(),
                "hyperliquid".to_string(),
            ]
        };
        
        info!("📊 配置的监控币种: {:?}", monitored_coins);
        
        // 获取任务执行间隔
        let crypto_interval = config.tasks.intervals
            .get("crypto_market")
            .copied()
            .unwrap_or(14400); // 默认4小时
        
        // 创建加密货币市场数据任务（使用新的构建器模式）
        let crypto_task = CryptoMarketTaskBuilder::new()
            .name("CryptoMarketDataTask".to_string())
            .client(coingecko_client)
            .coin_ids(monitored_coins)
            .interval(Duration::from_secs(crypto_interval))
            .cache(cache.clone()) // 新增：设置缓存
            .build()
            .context("创建加密货币市场数据任务失败")?;
        
        // 注册任务
        task_manager.register_task(Box::new(crypto_task))?;
        
        info!("✅ 任务注册完成");
        Ok(())
    }
    
    /// 启动调度器
    /// 
    /// # 参数
    /// * `web_port` - Web服务器端口（可选，默认3000）
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn start(&mut self, web_port: Option<u16>) -> Result<()> {
        info!("🚀 启动任务调度器");
        
        // 启动Web服务器（新增）
        let port = web_port.unwrap_or(3000);
        let web_server = self.web_server.clone();
        let web_handle = tokio::spawn(async move {
            if let Err(e) = web_server.start(port).await {
                error!("❌ Web服务器启动失败: {}", e);
            }
        });
        
        // 启动任务管理器
        let task_handle = tokio::spawn({
            let mut task_manager = self.task_manager.clone();
            let storage = self.storage.clone();
            async move {
                if let Err(e) = task_manager.start(storage).await {
                    error!("❌ 任务管理器启动失败: {}", e);
                }
            }
        });
        
        // 启动缓存清理任务（新增）
        let cache_cleanup_handle = tokio::spawn({
            let cache = self.cache.clone();
            async move {
                let mut cleanup_interval = time::interval(Duration::from_secs(3600)); // 每小时清理一次
                loop {
                    cleanup_interval.tick().await;
                    let removed = cache.cleanup_expired_data(24); // 清理24小时前的数据
                    if removed > 0 {
                        info!("🧹 缓存清理完成，移除 {} 条过期数据", removed);
                    }
                }
            }
        });
        
        info!("✅ 所有服务已启动");
        info!("🌐 Web仪表板: http://localhost:{}", port);
        info!("📡 API端点: http://localhost:{}/api", port);
        
        // 等待所有任务完成（实际上会一直运行）
        tokio::select! {
            _ = web_handle => {
                warn!("⚠️ Web服务器已停止");
            }
            _ = task_handle => {
                warn!("⚠️ 任务管理器已停止");
            }
            _ = cache_cleanup_handle => {
                warn!("⚠️ 缓存清理任务已停止");
            }
        }
        
        Ok(())
    }
    
    /// 停止调度器
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 正在停止任务调度器");
        
        // 停止任务管理器
        self.task_manager.stop().await?;
        
        // 清空缓存（新增）
        self.cache.clear_all();
        
        info!("✅ 任务调度器已停止");
        Ok(())
    }
    
    /// 获取调度器状态
    pub fn get_status(&self) -> OrchestratorStatus {
        OrchestratorStatus {
            task_count: self.task_manager.get_tasks().len(),
            cache_size: self.cache.size(), // 新增：缓存大小
            cache_stats: self.cache.get_stats(), // 新增：缓存统计
        }
    }
    
    /// 获取缓存引用（新增）
    pub fn get_cache(&self) -> Arc<DataCache> {
        self.cache.clone()
    }
}

/// 调度器状态
#[derive(Debug)]
pub struct OrchestratorStatus {
    /// 任务数量
    pub task_count: usize,
    /// 缓存大小（新增）
    pub cache_size: usize,
    /// 缓存统计（新增）
    pub cache_stats: crate::web::cache::CacheStats,
} 