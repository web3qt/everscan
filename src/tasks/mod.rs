// pub mod dune_task;
// pub mod glassnode_task;
// pub mod debank_task;
// pub mod coingecko_task;
pub mod crypto_market_task;
pub mod fear_greed_task; // 新增：贪婪恐惧指数任务
pub mod task_manager;


// pub use dune_task::*;
// pub use glassnode_task::*;
// pub use debank_task::*;
// pub use coingecko_task::*;
pub use task_manager::*;
pub use crypto_market_task::*;
pub use fear_greed_task::*; // 新增：导出贪婪恐惧指数任务


use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{info, error, debug};

use crate::storage::PostgresRepository;
use crate::models::AggregatedMetric;

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

/// 任务特征
/// 
/// 所有数据收集任务都必须实现此特征
#[async_trait::async_trait]
pub trait Task: Send + Sync {
    /// 获取任务名称
    fn name(&self) -> &str;
    
    /// 获取任务描述
    fn description(&self) -> &str;
    
    /// 获取执行间隔
    fn interval(&self) -> std::time::Duration;
    
    /// 执行任务
    /// 
    /// # 参数
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<Vec<AggregatedMetric>>` - 收集到的指标数据或错误
    async fn execute(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>>;
    
    /// 健康检查
    /// 
    /// # 返回
    /// * `Result<bool>` - 健康状态或错误
    async fn health_check(&self) -> Result<bool>;
    
    /// 获取任务状态
    fn status(&self) -> TaskStatus;
    
    /// 设置任务状态
    fn set_status(&self, status: TaskStatus);
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
    tasks: Arc<std::sync::RwLock<Vec<Box<dyn Task>>>>,
    /// 任务执行历史
    execution_history: Arc<std::sync::RwLock<HashMap<String, Vec<TaskExecutionResult>>>>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(std::sync::RwLock::new(Vec::new())),
            execution_history: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册任务
    /// 
    /// # 参数
    /// * `task` - 要注册的任务
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub fn register_task(&mut self, task: Box<dyn Task>) -> Result<()> {
        let task_name = task.name().to_string();
        
        // 检查是否已存在同名任务
        {
            let tasks = self.tasks.read().unwrap();
            if tasks.iter().any(|t| t.name() == task_name) {
                return Err(anyhow::anyhow!("任务 '{}' 已存在", task_name));
            }
        }
        
        // 添加任务
        {
            let mut tasks = self.tasks.write().unwrap();
            tasks.push(task);
        }
        
        info!("✅ 已注册任务: {}", task_name);
        Ok(())
    }
    
    /// 启动任务管理器
    /// 
    /// # 参数
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn start(&mut self, storage: Arc<PostgresRepository>) -> Result<()> {
        info!("🚀 启动任务管理器");
        
        // 执行健康检查
        self.health_check().await?;
        
        // 启动任务调度循环
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // 每分钟检查一次
        
        loop {
            interval.tick().await;
            
            // 检查并执行到期的任务
            if let Err(e) = self.check_and_execute_tasks(&storage).await {
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
        
        // 这里可以添加清理逻辑
        // 例如：等待正在运行的任务完成、关闭连接等
        
        info!("✅ 任务管理器已停止");
        Ok(())
    }
    
    /// 健康检查
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn health_check(&self) -> Result<()> {
        info!("🏥 正在执行健康检查");
        
        // 检查所有任务的健康状态
        let tasks = self.tasks.read().unwrap();
        for task in tasks.iter() {
            let task_name = task.name();
            match task.health_check().await {
                Ok(is_healthy) => {
                    if is_healthy {
                        info!("✅ 任务 {} 健康状态良好", task_name);
                    } else {
                        info!("⚠️ 任务 {} 健康状态不佳", task_name);
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
    /// # 参数
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    async fn check_and_execute_tasks(&self, storage: &PostgresRepository) -> Result<()> {
        debug!("🔍 检查待执行任务");
        
        // 这里简化实现，每次检查时都执行所有任务
        // 在实际应用中，应该根据任务的最后执行时间和间隔来决定是否执行
        let results = self.execute_all(storage).await?;
        
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
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<Vec<TaskExecutionResult>>` - 执行结果列表
    pub async fn execute_all(&self, storage: &PostgresRepository) -> Result<Vec<TaskExecutionResult>> {
        let mut results = Vec::new();
        
        let tasks = self.tasks.read().unwrap();
        for task in tasks.iter() {
            let start_time = std::time::Instant::now();
            let task_name = task.name().to_string();
            
            let result = match task.execute(storage).await {
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
                let mut history = self.execution_history.write().unwrap();
                history.entry(task_name).or_insert_with(Vec::new).push(result.clone());
            }
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// 获取任务列表
    pub fn get_tasks(&self) -> Vec<String> {
        let tasks = self.tasks.read().unwrap();
        tasks.iter().map(|task| task.name().to_string()).collect()
    }
    
    /// 获取任务状态
    /// 
    /// # 返回
    /// * `Vec<(String, String)>` - 任务名称和状态的列表
    pub fn get_task_status(&self) -> Vec<(String, String)> {
        let tasks = self.tasks.read().unwrap();
        tasks.iter()
            .map(|task| (task.name().to_string(), task.status().to_string()))
            .collect()
    }
} 