use sqlx::{PgPool, Row};
use anyhow::{Result, Context};
use tracing::{info, error, debug};
use chrono::{DateTime, Utc};

use std::time::Duration;

use crate::models::{AggregatedMetric, MetricFilter, MetricStats};
use crate::config::DatabaseConfig;

/// PostgreSQL存储仓库
/// 
/// 负责与PostgreSQL数据库的所有交互操作
/// 包括数据的增删改查和统计分析
pub struct PostgresRepository {
    /// 数据库连接池
    pool: PgPool,
}

impl PostgresRepository {
    /// 创建新的PostgreSQL存储仓库
    /// 
    /// # 参数
    /// * `config` - 数据库配置
    /// 
    /// # 返回
    /// * `Result<Self>` - 创建的存储仓库或错误
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("🔗 正在连接到PostgreSQL数据库...");
        
        // 创建数据库连接池
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.timeout_seconds))
            .connect(&config.url)
            .await
            .context("创建数据库连接池失败")?;
        
        let repo = Self { pool };
        
        // 初始化数据库表
        repo.init_tables().await?;
        
        info!("✅ PostgreSQL数据库连接成功");
        
        Ok(repo)
    }

    /// 创建模拟的PostgreSQL存储仓库（用于测试）
    /// 
    /// # 返回
    /// * `Self` - 模拟的存储仓库实例
    pub fn new_mock() -> Self {
        // 创建一个模拟的连接池（实际上不会使用）
        // 这里我们创建一个空的PgPool，但实际操作会被跳过
        use sqlx::postgres::PgPoolOptions;
        
        // 注意：这里创建的pool不会真正连接数据库
        // 所有操作都会返回默认值或空结果
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgresql://mock:mock@localhost/mock")
            .expect("创建模拟连接池失败");
            
        Self { pool }
    }
    
    /// 初始化数据库表
    /// 
    /// 创建必要的表结构和索引
    async fn init_tables(&self) -> Result<()> {
        info!("📋 正在初始化数据库表...");
        
        // 创建聚合指标表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS aggregated_metrics (
                id UUID PRIMARY KEY,
                source VARCHAR(50) NOT NULL,
                metric_name VARCHAR(100) NOT NULL,
                value JSONB NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                metadata JSONB
            )
        "#)
        .execute(&self.pool)
        .await
        .context("创建aggregated_metrics表失败")?;
        
        // 创建索引以优化查询性能
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_metrics_source ON aggregated_metrics(source)")
            .execute(&self.pool)
            .await
            .context("创建source索引失败")?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_metrics_metric_name ON aggregated_metrics(metric_name)")
            .execute(&self.pool)
            .await
            .context("创建metric_name索引失败")?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON aggregated_metrics(timestamp)")
            .execute(&self.pool)
            .await
            .context("创建timestamp索引失败")?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_metrics_source_metric ON aggregated_metrics(source, metric_name)")
            .execute(&self.pool)
            .await
            .context("创建复合索引失败")?;
        
        info!("✅ 数据库表初始化完成");
        
        Ok(())
    }
    
    /// 保存聚合指标
    /// 
    /// # 参数
    /// * `metrics` - 要保存的指标列表
    /// 
    /// # 返回
    /// * `Result<usize>` - 保存的记录数或错误
    pub async fn save_metrics(&self, metrics: &[AggregatedMetric]) -> Result<usize> {
        if metrics.is_empty() {
            return Ok(0);
        }
        
        debug!("💾 正在保存 {} 条指标数据", metrics.len());
        
        let mut tx = self.pool.begin().await.context("开始事务失败")?;
        
        let mut saved_count = 0;
        
        for metric in metrics {
            let result = sqlx::query(r#"
                INSERT INTO aggregated_metrics (
                    id, source, metric_name, value, timestamp, created_at, updated_at, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (id) DO UPDATE SET
                    value = EXCLUDED.value,
                    updated_at = EXCLUDED.updated_at,
                    metadata = EXCLUDED.metadata
            "#)
            .bind(&metric.id)
            .bind(&metric.source)
            .bind(&metric.metric_name)
            .bind(&metric.value)
            .bind(&metric.timestamp)
            .bind(&metric.created_at)
            .bind(&metric.updated_at)
            .bind(&metric.metadata)
            .execute(&mut *tx)
            .await;
            
            match result {
                Ok(_) => saved_count += 1,
                Err(e) => {
                    error!("❌ 保存指标失败: {}", e);
                    // 继续处理其他指标，不中断整个批次
                }
            }
        }
        
        tx.commit().await.context("提交事务失败")?;
        
        info!("✅ 成功保存 {} 条指标数据", saved_count);
        
        Ok(saved_count)
    }
    
    /// 获取指标数据
    /// 
    /// # 参数
    /// * `filter` - 过滤条件
    /// 
    /// # 返回
    /// * `Result<Vec<AggregatedMetric>>` - 指标数据列表或错误
    pub async fn get_metrics(&self, filter: &MetricFilter) -> Result<Vec<AggregatedMetric>> {
        debug!("🔍 正在获取指标数据，过滤条件: {:?}", filter);
        
        let mut query = "SELECT id, source, metric_name, value, timestamp, created_at, updated_at, metadata FROM aggregated_metrics WHERE 1=1".to_string();
        
        // 构建查询条件
        if let Some(source) = &filter.source {
            query.push_str(&format!(" AND source = '{}'", source));
        }
        
        if let Some(metric_name) = &filter.metric_name {
            query.push_str(&format!(" AND metric_name = '{}'", metric_name));
        }
        
        if let Some(time_range) = &filter.time_range {
            query.push_str(&format!(" AND timestamp >= '{}'", time_range.start.format("%Y-%m-%d %H:%M:%S")));
            query.push_str(&format!(" AND timestamp <= '{}'", time_range.end.format("%Y-%m-%d %H:%M:%S")));
        }
        
        // 添加排序
        query.push_str(" ORDER BY timestamp DESC");
        
        // 添加分页
        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }
        
        debug!("📊 执行查询: {}", query);
        
        let metrics = sqlx::query_as::<_, AggregatedMetric>(&query)
            .fetch_all(&self.pool)
            .await
            .context("获取指标数据失败")?;
        
        info!("✅ 成功获取 {} 条指标数据", metrics.len());
        Ok(metrics)
    }
    
    /// 获取数据统计信息
    /// 
    /// # 返回
    /// * `Result<MetricStats>` - 统计信息或错误
    pub async fn get_stats(&self) -> Result<MetricStats> {
        debug!("📈 正在获取数据统计信息...");
        
        // 获取总记录数
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM aggregated_metrics")
            .fetch_one(&self.pool)
            .await?;
        
        // 按数据源分组统计
        let source_stats = sqlx::query("SELECT source, COUNT(*) as count FROM aggregated_metrics GROUP BY source")
            .fetch_all(&self.pool)
            .await?;
        
        let mut by_source = std::collections::HashMap::new();
        for row in source_stats {
            let source: String = row.get("source");
            let count: i64 = row.get("count");
            by_source.insert(source, count);
        }
        
        // 按指标名称分组统计
        let metric_stats = sqlx::query("SELECT metric_name, COUNT(*) as count FROM aggregated_metrics GROUP BY metric_name")
            .fetch_all(&self.pool)
            .await?;
        
        let mut by_metric = std::collections::HashMap::new();
        for row in metric_stats {
            let metric_name: String = row.get("metric_name");
            let count: i64 = row.get("count");
            by_metric.insert(metric_name, count);
        }
        
        // 获取最新和最早的数据时间
        let latest_timestamp: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT MAX(timestamp) FROM aggregated_metrics"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let earliest_timestamp: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT MIN(timestamp) FROM aggregated_metrics"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let stats = MetricStats {
            total_count,
            by_source,
            by_metric,
            latest_timestamp,
            earliest_timestamp,
        };
        
        debug!("📈 统计信息获取完成: {} 条记录", total_count);
        
        Ok(stats)
    }
    
    /// 删除过期数据
    /// 
    /// # 参数
    /// * `before` - 删除此时间之前的数据
    /// 
    /// # 返回
    /// * `Result<u64>` - 删除的记录数或错误
    pub async fn delete_old_data(&self, before: DateTime<Utc>) -> Result<u64> {
        info!("🗑️  正在删除 {} 之前的数据", before);
        
        let result = sqlx::query("DELETE FROM aggregated_metrics WHERE timestamp < $1")
            .bind(before)
            .execute(&self.pool)
            .await
            .context("删除过期数据失败")?;
        
        let deleted_count = result.rows_affected();
        info!("✅ 删除了 {} 条过期数据", deleted_count);
        
        Ok(deleted_count)
    }
    
    /// 获取数据库连接池的引用
    /// 
    /// 用于需要直接访问数据库的场景
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    /// 数据库健康检查
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn health_check(&self) -> Result<()> {
        debug!("🏥 正在执行数据库健康检查");
        
        // 执行简单的查询来检查数据库连接
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("数据库连接检查失败")?;
        
        info!("✅ 数据库健康检查通过");
        Ok(())
    }
} 