use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct TestResponse {
    data: serde_json::Value,
    status: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    let api_key = env::var("COINMARKETCAP_API_KEY")
        .map_err(|_| "环境变量COINMARKETCAP_API_KEY未设置")?;
    
    println!("🔑 API Key: {}...", &api_key[..8]);
    
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // 测试各种可能的端点
    let endpoints = [
        "https://pro-api.coinmarketcap.com/v3/fear-and-greed/latest",
        "https://api.coinmarketcap.com/v3/fear-and-greed/latest",
        "https://pro-api.coinmarketcap.com/v1/fear-and-greed/latest",
        "https://api.coinmarketcap.com/v1/fear-and-greed/latest",
        "https://pro-api.coinmarketcap.com/v1/global-metrics/quotes/latest",
        "https://api.coinmarketcap.com/data-api/v3/fear-greed/latest",
    ];
    
    for endpoint in &endpoints {
        println!("\n🌐 测试端点: {}", endpoint);
        
        let response = client
            .get(*endpoint)
            .header("X-CMC_PRO_API_KEY", &api_key)
            .header("Accept", "application/json")
            .header("User-Agent", "EverScan/1.0")
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                let status = resp.status();
                println!("📊 HTTP状态: {}", status);
                
                let headers = resp.headers().clone();
                println!("📋 响应头:");
                for (name, value) in headers.iter() {
                    println!("  {}: {:?}", name, value);
                }
                
                let body = resp.text().await?;
                println!("📄 响应体 (前500字符): {}", 
                    if body.len() > 500 { &body[..500] } else { &body });
                
                if !body.is_empty() {
                    // 尝试解析JSON
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(json) => {
                            println!("✅ JSON解析成功:");
                            println!("{}", serde_json::to_string_pretty(&json)?);
                        }
                        Err(e) => {
                            println!("❌ JSON解析失败: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ 请求失败: {}", e);
            }
        }
        
        println!("{}", "=".repeat(50));
    }
    
    // 测试已知有效端点
    println!("\n🧪 测试已知有效端点: /v1/cryptocurrency/listings/latest");
    let listings_url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";
    let listings_response = client
        .get(listings_url)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .query(&[("limit", "10")])
        .send()
        .await?;
    
    println!("📊 HTTP状态: {}", listings_response.status());
    let listings_text = listings_response.text().await?;
    println!("📄 响应体 (前500字符): {}", &listings_text[..listings_text.len().min(500)]);
    
    // 测试quotes端点
    println!("\n🧪 测试quotes端点: /v1/cryptocurrency/quotes/latest");
    let quotes_url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest";
    let quotes_response = client
        .get(quotes_url)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .query(&[("symbol", "HYPE"), ("convert", "USD")])
        .send()
        .await?;
    
    println!("📊 HTTP状态: {}", quotes_response.status());
    let quotes_text = quotes_response.text().await?;
    println!("📄 响应体 (前500字符): {}", &quotes_text[..quotes_text.len().min(500)]);
    
    Ok(())
}