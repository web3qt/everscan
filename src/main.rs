use anyhow::Result;
use tracing::{info, error};
use std::env;

mod config;
mod orchestrator;
mod tasks;
mod clients;
mod storage;
mod models;

use config::Config;
use orchestrator::Orchestrator;

/// 应用程序入口点
/// 
/// 主要功能：
/// 1. 初始化日志系统
/// 2. 加载配置
/// 3. 启动任务调度器
#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
        )
        .init();
    
    info!("🚀 启动区块链数据聚合平台");
    
    // 加载配置
    let config = Config::load().await?;
    info!("✅ 配置加载完成");
    
    // 测试CoinGecko客户端
    info!("🧪 测试CoinGecko客户端...");
    let coingecko_client = crate::clients::CoinGeckoClient::new(
        config.api_keys.coingecko_api_key.clone(),
        std::time::Duration::from_secs(30)
    )?;
    
    // 从配置文件获取要测试的币种列表
    let test_coins = if let Some(crypto_config) = &config.crypto_monitoring {
        crypto_config.coins.clone()
    } else {
        vec!["bitcoin".to_string(), "hyperliquid".to_string()]
    };
    
    info!("📊 开始测试 {} 个币种的数据获取", test_coins.len());
    
    // 测试每个币种的数据获取
    for (index, coin_id) in test_coins.iter().enumerate() {
        info!("🔍 [{}/{}] 测试币种: {}", index + 1, test_coins.len(), coin_id);
        
        match coingecko_client.get_enhanced_market_data(coin_id, "usd").await {
            Ok(enhanced_data) => {
                let coin_price = &enhanced_data.coin_price;
                let indicators = &enhanced_data.technical_indicators;
                
                info!("✅ 成功获取 {} 的增强市场数据:", coin_id);
                info!("   代币名称: {} ({})", coin_price.name, coin_price.symbol.to_uppercase());
                info!("   当前价格: ${:.6}", coin_price.current_price);
                
                if let Some(volume) = coin_price.total_volume {
                    info!("   24小时交易量: ${:.0}", volume);
                }
                if let Some(change) = coin_price.price_change_percentage_24h {
                    let change_symbol = if change >= 0.0 { "📈" } else { "📉" };
                    info!("   24小时涨跌幅: {}{:.2}%", change_symbol, change);
                }
                if let Some(market_cap) = coin_price.market_cap {
                    info!("   市值: ${:.0}", market_cap);
                }
                
                // 技术指标
                info!("📊 技术指标:");
                info!("   布林带上轨: ${:.6}", indicators.bollinger_bands.upper);
                info!("   布林带中轨: ${:.6}", indicators.bollinger_bands.middle);
                info!("   布林带下轨: ${:.6}", indicators.bollinger_bands.lower);
                info!("   RSI: {:.2}", indicators.rsi.value);
                
                // RSI信号分析
                if indicators.rsi.value >= indicators.rsi.overbought_threshold {
                    info!("⚠️ {} RSI超买信号 (RSI: {:.2})", coin_price.symbol.to_uppercase(), indicators.rsi.value);
                } else if indicators.rsi.value <= indicators.rsi.oversold_threshold {
                    info!("⚠️ {} RSI超卖信号 (RSI: {:.2})", coin_price.symbol.to_uppercase(), indicators.rsi.value);
                } else {
                    info!("✅ {} RSI正常 (RSI: {:.2})", coin_price.symbol.to_uppercase(), indicators.rsi.value);
                }
                
                info!(""); // 空行分隔
            }
            Err(e) => {
                error!("❌ 获取 {} 的增强市场数据时出错: {}", coin_id, e);
                // 如果是API限制错误，等待一下再继续
                if e.to_string().contains("429") || e.to_string().contains("rate limit") {
                    info!("⏳ 遇到API限制，等待2秒后继续...");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }
    
    info!("✅ 多币种数据测试完成");
    
    // 如果没有配置数据库，就只进行客户端测试
    if config.database.url.is_empty() || config.database.url == "postgresql://localhost/everscan" {
        info!("⚠️ 数据库未配置，跳过完整的任务调度器启动");
        info!("✅ CoinGecko客户端测试完成");
        return Ok(());
    }
    
    // 创建并启动任务调度器
    let orchestrator = Orchestrator::new(config).await?;
    info!("✅ 任务调度器初始化完成");
    
    // 启动调度器（这会一直运行）
    if let Err(e) = orchestrator.start().await {
        error!("❌ 任务调度器运行失败: {}", e);
        return Err(e);
    }
    
    Ok(())
} 