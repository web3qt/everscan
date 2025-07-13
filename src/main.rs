use anyhow::Result;
use tracing::{info, error};
use std::env;
use std::sync::Arc;
use std::time::Duration;

mod config;
mod tasks;
mod clients;
mod storage;
mod models;
mod web;

use config::Config;

/// 应用程序入口点
/// 
/// 主要功能：
/// 1. 初始化日志系统
/// 2. 加载配置
/// 3. 创建并注册所有任务
/// 4. 启动时执行一次所有任务获取初始数据
/// 5. 启动Web服务器
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
    
    info!("🚀 启动EverScan区块链数据聚合平台");
    
    // 加载配置
    let config = Config::load().await?;
    info!("✅ 配置加载完成");
    
    // 检查是否为测试模式
    let is_test_mode = env::var("EVERSCAN_TEST_MODE")
        .unwrap_or_else(|_| "false".to_string()) // 默认为生产模式
        .parse::<bool>()
        .unwrap_or(false);
    
    if is_test_mode {
        info!("🧪 运行在测试模式");
        run_test_mode(&config).await?;
    } else {
        info!("🌐 运行在生产模式");
        run_production_mode(config).await?;
    }
    
    Ok(())
}

/// 生产模式：启动Web服务器并执行所有任务
async fn run_production_mode(config: Config) -> Result<()> {
    use crate::web::{WebServer, cache::DataCache};
    use crate::tasks::{TaskManager, CryptoMarketTaskBuilder, FearGreedTaskBuilder};
    use crate::clients::{CoinGeckoClient, CoinMarketCapClient};
    use crate::storage::PostgresRepository;
    
    // 创建数据缓存
    let cache = Arc::new(DataCache::new());
    
    // 创建存储仓库（暂时使用模拟实现）
    let storage = Arc::new(PostgresRepository::new_mock());
    
    // 创建任务管理器
    let mut task_manager = TaskManager::new();
    
    // 创建CoinGecko客户端
    let coingecko_client = Arc::new(CoinGeckoClient::new(
        config.api_keys.coingecko_api_key.clone(),
        Duration::from_secs(30)
    )?);
    
    // 创建CoinMarketCap客户端
    let coinmarketcap_client = Arc::new(CoinMarketCapClient::new(
        None, // 使用免费API，不需要密钥
        Duration::from_secs(30)
    )?);
    
    // 获取要监控的币种列表
    let coins_to_monitor = if let Some(crypto_config) = &config.crypto_monitoring {
        crypto_config.coins.clone()
    } else {
        vec!["bitcoin".to_string(), "hyperliquid".to_string()]
    };
    
    info!("📊 配置监控 {} 个币种: {:?}", coins_to_monitor.len(), coins_to_monitor);
    
    // 创建加密货币市场数据任务
    let crypto_task = CryptoMarketTaskBuilder::new()
        .name("加密货币市场数据采集".to_string())
        .client(coingecko_client.clone())
        .coin_ids(coins_to_monitor)
        .interval(Duration::from_secs(4 * 3600)) // 4小时执行一次
        .cache(cache.clone())
        .build()?;
    
    // 创建贪婪恐惧指数任务
    let fear_greed_task = FearGreedTaskBuilder::new()
        .name("贪婪恐惧指数采集".to_string())
        .client(coinmarketcap_client.clone())
        .interval(Duration::from_secs(6 * 3600)) // 6小时执行一次
        .cache(cache.clone())
        .build()?;
    
    // 注册任务
    task_manager.register_task(Box::new(crypto_task))?;
    task_manager.register_task(Box::new(fear_greed_task))?;
    
    info!("✅ 所有任务已注册完成");
    
    // 启动时执行一次所有任务，获取初始数据
    info!("🔄 启动时执行所有任务，获取初始数据...");
    match task_manager.execute_all(&storage).await {
        Ok(results) => {
            info!("✅ 初始数据采集完成，共执行 {} 个任务", results.len());
            for result in results {
                if result.success {
                    info!("  ✅ {} - 获取 {} 条数据，耗时 {}ms", 
                          result.task_name, result.metrics_count, result.execution_time_ms);
                } else {
                    error!("  ❌ {} - 执行失败: {}", 
                          result.task_name, result.error.unwrap_or_else(|| "未知错误".to_string()));
                }
            }
        }
        Err(e) => {
            error!("❌ 初始数据采集失败: {}", e);
            // 即使初始数据采集失败，也继续启动Web服务器
        }
    }
    
    // 创建Web服务器
    let web_server = WebServer::new(config, cache.clone(), Some(storage.clone()));
    
    // 获取Web服务器端口
    let web_port = env::var("WEB_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    
    info!("🌐 启动Web服务器在端口: {}", web_port);
    info!("📊 访问仪表板: http://localhost:{}", web_port);
    info!("📡 API端点: http://localhost:{}/api", web_port);
    info!("😱 贪婪恐惧指数: http://localhost:{}/api/fear-greed-index", web_port);
    
    // 注意：任务管理器的定期执行功能暂时注释掉，避免线程安全问题
    // 目前只在启动时执行一次任务，定期执行可以通过外部cron任务实现
    info!("📝 注意：定期任务执行功能已禁用，仅在启动时执行一次数据采集");
    
    // 启动Web服务器（这会一直运行）
    web_server.start(web_port).await?;
    
    Ok(())
}

/// 测试模式：仅测试数据获取功能
async fn run_test_mode(config: &Config) -> Result<()> {
    info!("🧪 开始测试模式");
    
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
                
                info!("✅ {} ({}) 数据获取成功:", coin_price.name, coin_price.symbol.to_uppercase());
                info!("   💰 当前价格: ${:.2}", coin_price.current_price);
                
                if let Some(volume) = coin_price.total_volume {
                    info!("   📈 24小时交易量: ${:.0}", volume);
                }
                if let Some(change) = coin_price.price_change_percentage_24h {
                    let change_symbol = if change >= 0.0 { "📈" } else { "📉" };
                    info!("   {} 24小时涨跌幅: {:.2}%", change_symbol, change);
                }
                if let Some(market_cap) = coin_price.market_cap {
                    info!("   🏛️ 市值: ${:.0}", market_cap);
                }
                
                // 技术指标
                info!("   📊 技术指标:");
                info!("      布林带上轨: ${:.2}", indicators.bollinger_bands.upper);
                info!("      布林带中轨: ${:.2}", indicators.bollinger_bands.middle);
                info!("      布林带下轨: ${:.2}", indicators.bollinger_bands.lower);
                
                let rsi_signal = if indicators.rsi.value >= indicators.rsi.overbought_threshold {
                    "超买 ⚠️"
                } else if indicators.rsi.value <= indicators.rsi.oversold_threshold {
                    "超卖 ⚠️"
                } else {
                    "正常 ✅"
                };
                info!("      RSI: {:.2} ({})", indicators.rsi.value, rsi_signal);
            }
            Err(e) => {
                error!("❌ {} 数据获取失败: {}", coin_id, e);
            }
        }
        
        // 在请求之间添加延迟
        if index < test_coins.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
    
    // 测试贪婪恐惧指数
    info!("🧪 测试贪婪恐惧指数获取...");
    let coinmarketcap_client = crate::clients::CoinMarketCapClient::new(
        None, // 使用免费API
        std::time::Duration::from_secs(30)
    )?;
    
    match coinmarketcap_client.get_fear_greed_index().await {
        Ok(fear_greed_data) => {
            let chinese_classification = crate::clients::CoinMarketCapClient::get_chinese_classification(&fear_greed_data.value_classification);
            let investment_advice = crate::clients::CoinMarketCapClient::get_investment_advice(fear_greed_data.value);
            
            info!("✅ 贪婪恐惧指数获取成功:");
            info!("   📊 指数值: {}", fear_greed_data.value);
            info!("   😱 情绪分类: {} ({})", chinese_classification, fear_greed_data.value_classification);
            info!("   💡 投资建议: {}", investment_advice);
            info!("   🕐 时间戳: {}", fear_greed_data.timestamp);
        }
        Err(e) => {
            error!("❌ 贪婪恐惧指数获取失败: {}", e);
        }
    }
    
    info!("🧪 测试模式完成");
    Ok(())
} 