use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt}; // 添加必要的trait导入
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use serde_json;

use super::cache::DataCache;

/// WebSocket连接处理器
/// 
/// # 参数
/// * `ws` - WebSocket升级请求
/// * `cache` - 数据缓存
/// 
/// # 返回
/// * `Response` - WebSocket响应
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(cache): State<Arc<DataCache>>,
) -> Response {
    info!("🔌 新的WebSocket连接请求");
    ws.on_upgrade(move |socket| handle_socket(socket, cache))
}

/// 处理WebSocket连接
/// 
/// # 参数
/// * `socket` - WebSocket连接
/// * `cache` - 数据缓存
async fn handle_socket(socket: WebSocket, cache: Arc<DataCache>) {
    info!("✅ WebSocket连接已建立");
    
    let (mut sender, mut receiver) = socket.split();
    
    // 启动数据推送任务
    let cache_clone = cache.clone();
    let push_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30)); // 每30秒推送一次数据
        
        loop {
            interval.tick().await;
            
            // 获取所有市场数据
            let market_data = cache_clone.get_all_market_data();
            
            if !market_data.is_empty() {
                // 序列化数据
                match serde_json::to_string(&market_data) {
                    Ok(json_data) => {
                        // 发送数据
                        if let Err(e) = sender.send(Message::Text(json_data)).await {
                            error!("❌ 发送WebSocket消息失败: {}", e);
                            break;
                        }
                        info!("📤 已推送 {} 个币种的市场数据", market_data.len());
                    }
                    Err(e) => {
                        error!("❌ 序列化市场数据失败: {}", e);
                    }
                }
            }
        }
    });
    
    // 处理客户端消息
    let message_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("📨 收到WebSocket消息: {}", text);
                    
                    // 这里可以处理客户端的特殊请求
                    // 比如订阅特定币种、更改推送频率等
                    match text.as_str() {
                        "ping" => {
                            // 响应ping请求
                            info!("🏓 响应ping请求");
                        }
                        "get_stats" => {
                            // 发送缓存统计信息
                            let stats = cache.get_stats();
                            if let Ok(stats_json) = serde_json::to_string(&stats) {
                                info!("📊 发送缓存统计信息");
                            }
                        }
                        _ => {
                            info!("❓ 未知WebSocket消息: {}", text);
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!("📦 收到二进制消息，暂不支持");
                }
                Ok(Message::Close(_)) => {
                    info!("👋 WebSocket连接关闭");
                    break;
                }
                Err(e) => {
                    error!("❌ WebSocket消息错误: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });
    
    // 等待任何一个任务完成
    tokio::select! {
        _ = push_task => {
            info!("📤 数据推送任务结束");
        }
        _ = message_task => {
            info!("📨 消息处理任务结束");
        }
    }
    
    info!("🔌 WebSocket连接已断开");
} 