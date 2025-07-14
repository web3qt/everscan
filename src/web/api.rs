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

use super::cache::{DataCache, CachedMarketData, CacheStats};
// 新增：导入山寨季节指数类型
use crate::clients::coinmarketcap_client::AltcoinSeasonIndex;

/// API响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 是否成功
    pub success: bool,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 响应时间戳
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            timestamp: Utc::now(),
        }
    }
    
    /// 创建错误响应
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message.into()),
            timestamp: Utc::now(),
        }
    }
}

/// 创建API路由
/// 
/// # 参数
/// * `cache` - 数据缓存
/// 
/// # 返回
/// * `Router<Arc<DataCache>>` - 配置好的API路由器
pub fn create_api_routes(
    cache: Arc<DataCache>,
) -> Router<Arc<DataCache>> {
    Router::new()
        // 健康检查端点
        .route("/health", get(health_check))
        // 获取所有市场数据
        .route("/market-data", get(get_all_market_data))
        // 获取特定币种数据
        .route("/market-data/:coin_id", get(get_market_data))
        // 获取缓存统计信息
        .route("/cache-stats", get(get_cache_stats))
        // 获取恐惧贪婪指数
        .route("/fear-greed-index", get(get_fear_greed_index))
        // 获取山寨币季节指数
        .route("/altcoin-season-index", get(get_altcoin_season_index))
        .with_state(cache)
}

/// 健康检查端点
async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "service": "EverScan API",
        "version": "1.0.0"
    })))
}

/// 获取所有市场数据
async fn get_all_market_data(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<Vec<CachedMarketData>>>, StatusCode> {
    let market_data = cache.get_all_market_data();
    
    if market_data.is_empty() {
        return Ok(Json(ApiResponse::error("暂无市场数据")));
    }
    
    Ok(Json(ApiResponse::success(market_data)))
}

/// 获取特定币种的市场数据
async fn get_market_data(
    State(cache): State<Arc<DataCache>>,
    axum::extract::Path(coin_id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<CachedMarketData>>, StatusCode> {
    match cache.get_market_data(&coin_id) {
        Some(data) => Ok(Json(ApiResponse::success(data))),
        None => Ok(Json(ApiResponse::error(format!("未找到币种 {} 的数据", coin_id)))),
    }
}

/// 获取缓存统计信息
async fn get_cache_stats(
    State(cache): State<Arc<DataCache>>,
) -> Json<ApiResponse<CacheStats>> {
    let stats = cache.get_stats();
    Json(ApiResponse::success(stats))
}

/// 获取恐惧贪婪指数
async fn get_fear_greed_index(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    match cache.get_fear_greed_index() {
        Some(data) => Ok(Json(ApiResponse::success(data))),
        None => Ok(Json(ApiResponse::error("恐惧贪婪指数数据不可用"))),
    }
}

/// 获取山寨币季节指数
async fn get_altcoin_season_index(
    State(cache): State<Arc<DataCache>>,
) -> Result<Json<ApiResponse<AltcoinSeasonIndex>>, StatusCode> {
    match cache.get_altcoin_season_index() {
        Some(data) => Ok(Json(ApiResponse::success(data))),
        None => Ok(Json(ApiResponse::error("山寨币季节指数数据不可用"))),
    }
} 