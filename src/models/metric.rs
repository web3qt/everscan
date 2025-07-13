use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sqlx::FromRow;

/// 聚合指标数据模型
/// 
/// 这是系统中所有数据的统一存储格式
/// 支持存储来自不同数据源的各种类型的指标数据
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AggregatedMetric {
    /// 唯一标识符
    pub id: Uuid,
    /// 数据源标识（dune、glassnode、debank等）
    pub source: String,
    /// 指标名称（如"eth_active_addresses"、"uniswap_v3_volume"）
    pub metric_name: String,
    /// 指标值（使用JSON格式存储，支持复杂数据结构）
    pub value: serde_json::Value,
    /// 数据时间戳
    pub timestamp: DateTime<Utc>,
    /// 记录创建时间
    pub created_at: DateTime<Utc>,
    /// 记录更新时间
    pub updated_at: DateTime<Utc>,
    /// 扩展元数据（可选）
    pub metadata: Option<serde_json::Value>,
}

/// 数据源枚举
/// 
/// 定义系统支持的所有数据源类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    /// Dune Analytics
    Dune,
    /// Glassnode
    Glassnode,
    /// DeBank
    DeBank,
    /// CoinGecko
    CoinGecko,
    /// Arkham Intelligence
    Arkham,
    /// Bitget
    Bitget,
}

impl DataSource {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            DataSource::Dune => "dune",
            DataSource::Glassnode => "glassnode",
            DataSource::DeBank => "debank",
            DataSource::CoinGecko => "coingecko",
            DataSource::Arkham => "arkham",
            DataSource::Bitget => "bitget",
        }
    }
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 指标构建器
/// 
/// 用于方便地构建聚合指标实例
pub struct MetricBuilder {
    source: String,
    metric_name: String,
    value: serde_json::Value,
    timestamp: Option<DateTime<Utc>>,
    metadata: Option<serde_json::Value>,
}

impl MetricBuilder {
    /// 创建新的构建器
    pub fn new(source: DataSource, metric_name: impl Into<String>) -> Self {
        Self {
            source: source.to_string(),
            metric_name: metric_name.into(),
            value: serde_json::Value::Null,
            timestamp: None,
            metadata: None,
        }
    }
    
    /// 设置指标值
    pub fn value(mut self, value: serde_json::Value) -> Self {
        self.value = value;
        self
    }
    
    /// 设置时间戳
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    
    /// 设置元数据
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// 构建聚合指标
    pub fn build(self) -> AggregatedMetric {
        let now = Utc::now();
        AggregatedMetric {
            id: Uuid::new_v4(),
            source: self.source,
            metric_name: self.metric_name,
            value: self.value,
            timestamp: self.timestamp.unwrap_or(now),
            created_at: now,
            updated_at: now,
            metadata: self.metadata,
        }
    }
}

/// 查询过滤器
/// 
/// 用于数据库查询时的条件过滤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricFilter {
    /// 数据源过滤
    pub source: Option<String>,
    /// 指标名称过滤
    pub metric_name: Option<String>,
    /// 时间范围过滤
    pub time_range: Option<TimeRange>,
    /// 限制返回数量
    pub limit: Option<i64>,
    /// 偏移量
    pub offset: Option<i64>,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// 开始时间
    pub start: DateTime<Utc>,
    /// 结束时间
    pub end: DateTime<Utc>,
}

impl MetricFilter {
    /// 创建新的过滤器
    pub fn new() -> Self {
        Self {
            source: None,
            metric_name: None,
            time_range: None,
            limit: None,
            offset: None,
        }
    }
    
    /// 设置数据源过滤
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    
    /// 设置指标名称过滤
    pub fn metric_name(mut self, metric_name: impl Into<String>) -> Self {
        self.metric_name = Some(metric_name.into());
        self
    }
    
    /// 设置时间范围过滤
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some(TimeRange { start, end });
        self
    }
    
    /// 设置限制数量
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// 设置偏移量
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl Default for MetricFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 聚合统计结果
/// 
/// 用于返回数据统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    /// 总记录数
    pub total_count: i64,
    /// 按数据源分组的统计
    pub by_source: std::collections::HashMap<String, i64>,
    /// 按指标名称分组的统计
    pub by_metric: std::collections::HashMap<String, i64>,
    /// 最新数据时间
    pub latest_timestamp: Option<DateTime<Utc>>,
    /// 最早数据时间
    pub earliest_timestamp: Option<DateTime<Utc>>,
} 