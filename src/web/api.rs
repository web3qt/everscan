use axum::{
    Router,
    routing::get,
    extract::State,
    response::Json,
    http::StatusCode,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::storage::PostgresRepository;
use super::cache::{DataCache, CachedMarketData, CacheStats};

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// 数据缓存
    pub cache: Arc<DataCache>,
    /// 数据库存储（可选）
    pub storage: Option<Arc<PostgresRepository>>,
}

/// API响应包装器
#[derive(Serialize)]
pub struct ApiResponse<T> {
    /// 是否成功
    pub success: bool,
    /// 响应数据
    pub data: Option<T>,
    /// 错误信息
    pub error: Option<String>,
    /// 响应时间戳
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }
    
    /// 创建错误响应
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

/// 创建API路由
/// 
/// # 参数
/// * `cache` - 数据缓存
/// * `storage` - 数据库存储（可选）
/// 
/// # 返回
/// * `Router<Arc<DataCache>>` - 配置好的API路由器
pub fn create_api_routes(
    cache: Arc<DataCache>,
    storage: Option<Arc<PostgresRepository>>,
) -> Router<Arc<DataCache>> {
    Router::new()
        // 健康检查端点
        .route("/health", get(health_check))
        // 获取所有市场数据
        .route("/market-data", get(get_all_market_data))
        // 获取特定币种数据
        .route("/market-data/:coin_id", get(get_coin_market_data))
        // 获取缓存统计
        .route("/stats", get(get_cache_stats))
        // 获取支持的币种列表
        .route("/coins", get(get_supported_coins))
        // 获取贪婪恐惧指数
        .route("/fear-greed-index", get(get_fear_greed_index))
}

/// 健康检查端点
async fn health_check() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::success("EverScan API is running"))
}

/// 获取所有市场数据
async fn get_all_market_data(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<Vec<CachedMarketData>>>, StatusCode> {
    let market_data = cache.get_all_market_data();
    Ok(Json(ApiResponse::success(market_data)))
}

/// 获取特定币种的市场数据
async fn get_coin_market_data(
    State(cache): State<Arc<DataCache>>,
    axum::extract::Path(coin_id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<Option<CachedMarketData>>>, StatusCode> {
    let market_data = cache.get_market_data(&coin_id);
    Ok(Json(ApiResponse::success(market_data)))
}

/// 获取缓存统计信息
async fn get_cache_stats(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<CacheStats>>, StatusCode> {
    let stats = cache.get_stats();
    Ok(Json(ApiResponse::success(stats)))
}

/// 获取支持的币种列表
async fn get_supported_coins(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let coins = cache.get_supported_coins();
    Ok(Json(ApiResponse::success(coins)))
}

/// 获取贪婪恐惧指数
async fn get_fear_greed_index(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<Option<serde_json::Value>>>, StatusCode> {
    let fear_greed_data = cache.get_fear_greed_index();
    Ok(Json(ApiResponse::success(fear_greed_data)))
} 