pub mod api;
pub mod cache;
pub mod websocket;

use axum::{
    Router,
    routing::get,
    http::StatusCode,
    response::{Html, IntoResponse},
    extract::Path,
};
use tower_http::{
    services::ServeDir,
    cors::CorsLayer,
    trace::TraceLayer,
};
use std::sync::Arc;

use crate::config::Config;
use crate::storage::PostgresRepository;
use self::{
    api::{create_api_routes, AppState},
    cache::DataCache,
    websocket::websocket_handler,
};

/// Web服务器结构
/// 
/// 负责提供RESTful API和静态文件服务
/// 支持实时数据推送和可视化界面
#[derive(Clone)] // 添加Clone trait
pub struct WebServer {
    /// 应用配置
    config: Config,
    /// 数据缓存
    cache: Arc<DataCache>,
    /// 数据库存储（可选）
    storage: Option<Arc<PostgresRepository>>,
}

impl WebServer {
    /// 创建新的Web服务器
    /// 
    /// # 参数
    /// * `config` - 应用配置
    /// * `cache` - 数据缓存
    /// * `storage` - 数据库存储（可选）
    /// 
    /// # 返回
    /// * `Self` - Web服务器实例
    pub fn new(
        config: Config,
        cache: Arc<DataCache>,
        storage: Option<Arc<PostgresRepository>>,
    ) -> Self {
        Self {
            config,
            cache,
            storage,
        }
    }
    
    /// 启动Web服务器
    /// 
    /// # 参数
    /// * `port` - 监听端口
    /// 
    /// # 返回
    /// * `Result<()>` - 成功或错误
    pub async fn start(&self, port: u16) -> anyhow::Result<()> {
        let app = self.create_app();
        
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        
        tracing::info!("🌐 Web服务器启动在 http://0.0.0.0:{}", port);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
    
    /// 创建应用路由
    fn create_app(&self) -> Router {
        // 创建API路由
        let api_routes = create_api_routes(
            self.cache.clone(),
            self.storage.clone(),
        );
        
        // 创建应用状态
        let app_state = AppState {
            cache: self.cache.clone(),
            storage: self.storage.clone(),
        };
        
        Router::new()
            // 主页
            .route("/", get(dashboard_page))
            // WebSocket端点
            .route("/ws", get(websocket_handler))
            // API路由
            .nest("/api", api_routes)
            // 静态文件服务
            .nest_service("/static", ServeDir::new("static"))
            // 中间件
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .with_state(self.cache.clone())
    }
}

/// 仪表板页面处理器
async fn dashboard_page() -> impl IntoResponse {
    // 读取静态HTML文件
    match tokio::fs::read_to_string("static/dashboard.html").await {
        Ok(content) => Html(content),
        Err(_) => {
            // 如果文件不存在，返回简单的HTML页面
            Html(include_str!("../../static/dashboard.html").to_string())
        }
    }
}

/// 404处理
async fn not_found(Path(path): Path<String>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("页面未找到: /{}", path)
    )
} 