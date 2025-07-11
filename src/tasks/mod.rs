pub mod dune_task;
pub mod glassnode_task;
pub mod debank_task;
pub mod coingecko_task;

pub use dune_task::*;
pub use glassnode_task::*;
pub use debank_task::*;
pub use coingecko_task::*;

use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

use crate::models::AggregatedMetric;
use crate::storage::PostgresRepository;

/// 任务trait
/// 
/// 定义所有数据获取任务的通用接口
/// 每个数据源都需要实现这个trait
#[async_trait]
pub trait Task: Send + Sync {
    /// 获取任务名称
    fn name(&self) -> &str;
    
    /// 获取任务描述
    fn description(&self) -> &str;
    
    /// 获取任务执行间隔
    fn interval(&self) -> Duration;
    
    /// 执行任务
    /// 
    /// # 参数
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<Vec<AggregatedMetric>>` - 获取的指标数据或错误
    async fn execute(&self, storage: &PostgresRepository) -> Result<Vec<AggregatedMetric>>;
    
    /// 任务健康检查
    /// 
    /// # 返回
    /// * `Result<bool>` - 健康状态或错误
    async fn health_check(&self) -> Result<bool>;
    
    /// 获取任务状态
    /// 
    /// # 返回
    /// * `TaskStatus` - 任务状态
    fn status(&self) -> TaskStatus;
    
    /// 设置任务状态
    /// 
    /// # 参数
    /// * `status` - 新的任务状态
    fn set_status(&self, status: TaskStatus);
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 空闲状态
    Idle,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已禁用
    Disabled,
}

impl TaskStatus {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Idle => "idle",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Disabled => "disabled",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 任务执行结果
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// 任务名称
    pub task_name: String,
    /// 执行是否成功
    pub success: bool,
    /// 获取的指标数量
    pub metrics_count: usize,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 任务管理器
/// 
/// 负责管理所有任务的注册、执行和状态跟踪
pub struct TaskManager {
    /// 已注册的任务
    tasks: Vec<Box<dyn Task>>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
        }
    }
    
    /// 注册任务
    /// 
    /// # 参数
    /// * `task` - 要注册的任务
    pub fn register_task(&mut self, task: Box<dyn Task>) {
        self.tasks.push(task);
    }
    
    /// 获取所有任务
    pub fn get_tasks(&self) -> &[Box<dyn Task>] {
        &self.tasks
    }
    
    /// 根据名称获取任务
    /// 
    /// # 参数
    /// * `name` - 任务名称
    /// 
    /// # 返回
    /// * `Option<&Box<dyn Task>>` - 找到的任务或None
    pub fn get_task_by_name(&self, name: &str) -> Option<&Box<dyn Task>> {
        self.tasks.iter().find(|task| task.name() == name)
    }
    
    /// 执行所有任务
    /// 
    /// # 参数
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `Result<Vec<TaskResult>>` - 执行结果或错误
    pub async fn execute_all(&self, storage: &PostgresRepository) -> Result<Vec<TaskResult>> {
        let mut results = Vec::new();
        
        for task in &self.tasks {
            let result = self.execute_task(task.as_ref(), storage).await;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// 执行单个任务
    /// 
    /// # 参数
    /// * `task` - 要执行的任务
    /// * `storage` - 存储仓库
    /// 
    /// # 返回
    /// * `TaskResult` - 执行结果
    async fn execute_task(&self, task: &dyn Task, storage: &PostgresRepository) -> TaskResult {
        let start_time = std::time::Instant::now();
        
        task.set_status(TaskStatus::Running);
        
        let result = task.execute(storage).await;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(metrics) => {
                task.set_status(TaskStatus::Completed);
                TaskResult {
                    task_name: task.name().to_string(),
                    success: true,
                    metrics_count: metrics.len(),
                    execution_time_ms,
                    error: None,
                }
            }
            Err(e) => {
                task.set_status(TaskStatus::Failed);
                TaskResult {
                    task_name: task.name().to_string(),
                    success: false,
                    metrics_count: 0,
                    execution_time_ms,
                    error: Some(e.to_string()),
                }
            }
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
} 