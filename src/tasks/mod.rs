pub mod task_manager;
pub mod crypto_market_task;
pub mod fear_greed_task;
pub mod altcoin_season_task;

pub use task_manager::*;
pub use crypto_market_task::*;
pub use fear_greed_task::*;
pub use altcoin_season_task::*;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};

use crate::models::AggregatedMetric;
use crate::web::cache::DataCache;

/// 任务执行特征
/// 
/// 所有数据采集任务都需要实现这个特征
#[async_trait]
pub trait Task: Send + Sync {
    /// 获取任务名称
    fn name(&self) -> &str;
    
    /// 获取任务描述
    fn description(&self) -> &str;
    
    /// 获取任务ID
    fn id(&self) -> &str;
    
    /// 获取执行间隔（秒）
    fn interval_seconds(&self) -> u64;
    
    /// 执行任务
    /// 
    /// # 参数
    /// * `cache` - 数据缓存
    /// 
    /// # 返回
    /// * `Result<Vec<AggregatedMetric>>` - 采集到的指标数据或错误
    async fn execute(&self, cache: &DataCache) -> Result<Vec<AggregatedMetric>>;
}

/// 任务状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// 空闲
    Idle = 0,
    /// 运行中
    Running = 1,
    /// 已完成
    Completed = 2,
    /// 失败
    Failed = 3,
    /// 禁用
    Disabled = 4,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Idle => write!(f, "空闲"),
            TaskStatus::Running => write!(f, "运行中"),
            TaskStatus::Completed => write!(f, "已完成"),
            TaskStatus::Failed => write!(f, "失败"),
            TaskStatus::Disabled => write!(f, "禁用"),
        }
    }
}

/// 任务执行结果
#[derive(Debug, Clone)] // 添加Clone trait
pub struct TaskExecutionResult {
    /// 任务名称
    pub task_name: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 收集到的指标数量
    pub metrics_count: usize,
    /// 执行时间（毫秒）
    pub execution_time_ms: u128,
    /// 执行时间戳
    pub executed_at: DateTime<Utc>,
}

/// 任务管理器
/// 
/// 负责管理和调度所有数据收集任务
#[derive(Clone)] // 添加Clone trait
pub struct TaskManager {
    /// 已注册的任务列表
    tasks: Arc<RwLock<Vec<Box<dyn Task>>>>,
    /// 任务执行历史
    execution_history: Arc<RwLock<HashMap<String, Vec<TaskExecutionResult>>>>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(Vec::new())),
            execution_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册任务
    /// 
    /// # 参数
    /// * `task` - 要注册的任务
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn register_task(&mut self, task: Box<dyn Task>) -> Result<()> {
        let task_name = task.name().to_string();
        
        // 检查是否已存在同名任务
        {
            let tasks = self.tasks.read().await;
            if tasks.iter().any(|t| t.name() == task_name) {
                return Err(anyhow::anyhow!("任务 '{}' 已存在", task_name));
            }
        }
        
        // 添加任务
        {
            let mut tasks = self.tasks.write().await;
            tasks.push(task);
        }
        
        info!("✅ 已注册任务: {}", task_name);
        Ok(())
    }
    
    /// 启动任务管理器
    /// 
    /// # 参数
    /// * `cache` - 数据缓存
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn start(&mut self, cache: Arc<DataCache>) -> Result<()> {
        info!("🚀 启动任务管理器");
        
        // 立即执行一次所有任务以获取初始数据
        info!("🔄 启动时执行所有任务，获取初始数据...");
        if let Err(e) = self.check_and_execute_tasks(&cache).await {
            error!("❌ 初始任务执行失败: {}", e);
        }
        
        // 启动任务调度循环
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // 每小时检查一次
        
        loop {
            interval.tick().await;
            
            // 检查并执行到期的任务
            if let Err(e) = self.check_and_execute_tasks(&cache).await {
                error!("❌ 任务执行检查失败: {}", e);
            }
        }
    }
    
    /// 停止任务管理器
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 正在停止任务管理器");
        info!("✅ 任务管理器已停止");
        Ok(())
    }
    
    /// 检查并执行到期的任务
    /// 
    /// # 参数
    /// * `cache` - 数据缓存
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn check_and_execute_tasks(&self, cache: &DataCache) -> Result<()> {
        debug!("🔍 检查待执行任务");
        
        let results = self.execute_all(cache).await?;
        
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
    
    /// 执行所有任务
    /// 
    /// # 参数
    /// * `cache` - 数据缓存
    /// 
    /// # 返回
    /// * `Result<Vec<TaskExecutionResult>>` - 执行结果列表
    pub async fn execute_all(&self, cache: &DataCache) -> Result<Vec<TaskExecutionResult>> {
        let mut results = Vec::new();
        
        // 获取所有任务并执行
        let tasks = self.tasks.read().await;
        for task in tasks.iter() {
            let start_time = std::time::Instant::now();
            let task_name = task.name().to_string();
            
            let result = match task.execute(cache).await {
                    Ok(metrics) => {
                        let execution_time = start_time.elapsed();
                        TaskExecutionResult {
                            task_name: task_name.clone(),
                            success: true,
                            error: None,
                            metrics_count: metrics.len(),
                            execution_time_ms: execution_time.as_millis(),
                            executed_at: Utc::now(),
                        }
                    }
                    Err(e) => {
                        let execution_time = start_time.elapsed();
                        TaskExecutionResult {
                            task_name: task_name.clone(),
                            success: false,
                            error: Some(e.to_string()),
                            metrics_count: 0,
                            execution_time_ms: execution_time.as_millis(),
                            executed_at: Utc::now(),
                        }
                    }
                };
                
            // 保存执行历史
            {
                let mut history = self.execution_history.write().await;
                history.entry(task_name).or_insert_with(Vec::new).push(result.clone());
            }
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// 获取任务列表
    pub async fn get_tasks(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks.iter().map(|task| task.name().to_string()).collect()
    }
    
    /// 获取任务状态
    pub async fn get_task_status(&self) -> Vec<(String, String)> {
        let tasks = self.tasks.read().await;
        tasks.iter().map(|task| {
            (task.name().to_string(), "运行中".to_string())
        }).collect()
    }
} 