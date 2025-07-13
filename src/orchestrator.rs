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

/// 任务调度器
/// 
/// 负责管理所有数据获取任务的生命周期
/// 包括初始化、调度和监控
pub struct Orchestrator {
    /// 配置
    config: Config,
    /// 存储仓库
    storage: Arc<PostgresRepository>,
    /// 任务管理器
    task_manager: TaskManager,
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
        
        // 创建任务管理器
        let mut task_manager = TaskManager::new();
        
        // 注册所有任务
        Self::register_tasks(&mut task_manager, &config, storage.clone()).await?;
        
        info!("✅ 任务调度器初始化完成，共注册 {} 个任务", task_manager.get_tasks().len());
        
        Ok(Self {
            config,
            storage,
            task_manager,
        })
    }
    
    /// 注册所有任务
    /// 
    /// # 参数
    /// * `task_manager` - 任务管理器
    /// * `config` - 应用配置
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn register_tasks(task_manager: &mut TaskManager, config: &Config, storage: Arc<PostgresRepository>) -> Result<()> {
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
                info!("📊 从配置文件读取到 {} 个监控币种: {:?}", 
                      crypto_config.coins.len(), crypto_config.coins);
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
        
        let crypto_interval = Duration::from_secs(
            config.tasks.intervals.get("crypto_market").copied().unwrap_or(14400) as u64
        );
        
        let crypto_task = CryptoMarketTaskBuilder::new()
            .client(coingecko_client.clone())
            .interval(crypto_interval)
            .coin_ids(monitored_coins.clone())
            .name("CryptoMarketDataTask".to_string())
            .build()?;
        
        task_manager.register_task(Box::new(crypto_task));
        info!("✅ 已注册加密货币市场数据任务");
        info!("   📈 监控币种: {:?}", monitored_coins);
        info!("   ⏰ 执行间隔: {} 秒 ({} 小时)", 
              crypto_interval.as_secs(), 
              crypto_interval.as_secs() / 3600);
        
        // 如果有技术指标配置，记录配置信息
        if let Some(crypto_config) = &config.crypto_monitoring {
            if let Some(tech_config) = &crypto_config.technical_indicators {
                info!("📊 技术指标配置:");
                if let Some(rsi_period) = tech_config.rsi_period {
                    info!("   RSI周期: {} 天", rsi_period);
                }
                if let Some(bollinger_period) = tech_config.bollinger_period {
                    info!("   布林带周期: {} 天", bollinger_period);
                }
                if let Some(bollinger_std) = tech_config.bollinger_std_dev {
                    info!("   布林带标准差: {}", bollinger_std);
                }
            }
            
            if let Some(data_config) = &crypto_config.data_collection {
                info!("📋 数据收集配置:");
                if let Some(history_days) = data_config.history_days {
                    info!("   历史数据天数: {} 天", history_days);
                }
                if let Some(enable_tech) = data_config.enable_technical_indicators {
                    info!("   技术指标计算: {}", if enable_tech { "启用" } else { "禁用" });
                }
            }
        }
        
        // 保留原有的CoinGecko任务作为备用（如果需要）
        if let Some(api_key) = &config.api_keys.coingecko_api_key {
            let coingecko_interval = Duration::from_secs(
                config.tasks.intervals.get("coingecko").copied().unwrap_or(300) as u64
            );
            
            let coingecko_task = CoinGeckoTaskBuilder::new()
                .client(coingecko_client)
                .interval(coingecko_interval)
                .build()?;
            
            task_manager.register_task(Box::new(coingecko_task));
            info!("✅ 已注册传统CoinGecko任务");
        }
        
        // 可以继续注册其他任务...
        // 注册Dune任务
        if let Some(_) = &config.api_keys.dune_api_key {
            info!("✅ 已配置Dune API密钥（暂未实现任务）");
        }
        
        // 注册Glassnode任务
        if let Some(_) = &config.api_keys.glassnode_api_key {
            info!("✅ 已配置Glassnode API密钥（暂未实现任务）");
        }
        
        // 注册DeBank任务
        if let Some(_) = &config.api_keys.debank_api_key {
            info!("✅ 已配置DeBank API密钥（暂未实现任务）");
        }
        
        info!("📋 任务注册完成");
        Ok(())
    }
    
    /// 启动任务调度器
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn start(&self) -> Result<()> {
        info!("🚀 启动任务调度器");
        
        // 执行健康检查
        self.health_check().await?;
        
        // 启动任务调度循环
        let mut interval = time::interval(Duration::from_secs(60)); // 每分钟检查一次
        
        loop {
            interval.tick().await;
            
            // 检查并执行到期的任务
            if let Err(e) = self.check_and_execute_tasks().await {
                error!("❌ 任务执行检查失败: {}", e);
            }
        }
    }
    
    /// 健康检查
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn health_check(&self) -> Result<()> {
        info!("🏥 正在执行健康检查");
        
        // 检查数据库连接
        if let Err(e) = self.storage.health_check().await {
            error!("❌ 数据库健康检查失败: {}", e);
            return Err(e);
        }
        
        // 检查所有任务的健康状态
        for task in self.task_manager.get_tasks() {
            let task_name = task.name();
            match task.health_check().await {
                Ok(is_healthy) => {
                    if is_healthy {
                        info!("✅ 任务 {} 健康状态良好", task_name);
                    } else {
                        warn!("⚠️ 任务 {} 健康状态不佳", task_name);
                    }
                }
                Err(e) => {
                    error!("❌ 任务 {} 健康检查失败: {}", task_name, e);
                }
            }
        }
        
        info!("✅ 健康检查完成");
        Ok(())
    }
    
    /// 检查并执行到期的任务
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn check_and_execute_tasks(&self) -> Result<()> {
        debug!("🔍 检查待执行任务");
        
        // 这里简化实现，每次检查时都执行所有任务
        // 在实际应用中，应该根据任务的最后执行时间和间隔来决定是否执行
        let results = self.task_manager.execute_all(&self.storage).await?;
        
        // 记录执行结果
        for result in results {
            if result.success {
                info!("✅ 任务 {} 执行成功，获取 {} 条数据，耗时 {}ms", 
                      result.task_name, result.metrics_count, result.execution_time_ms);
            } else {
                error!("❌ 任务 {} 执行失败: {}", 
                      result.task_name, result.error.unwrap_or_else(|| "未知错误".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// 获取任务状态
    /// 
    /// # 返回
    /// * `Vec<(String, String)>` - 任务名称和状态的列表
    pub fn get_task_status(&self) -> Vec<(String, String)> {
        self.task_manager.get_tasks()
            .iter()
            .map(|task| (task.name().to_string(), task.status().to_string()))
            .collect()
    }
    
    /// 停止调度器
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 正在停止任务调度器");
        
        // 这里可以添加清理逻辑
        // 例如：等待正在运行的任务完成、关闭连接等
        
        info!("✅ 任务调度器已停止");
        Ok(())
    }
} 