use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::env;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();

    let api_key = env::var("COINMARKETCAP_API_KEY").expect("COINMARKETCAP_API_KEY 环境变量未设置");

    println!("🔑 使用API密钥: {}...", &api_key[..8]);

    let client = Client::new();

    // 测试1: 尝试获取Fear & Greed Index (历史数据端点)
    println!("\n📊 测试1: 获取Fear & Greed Index (历史数据)");
    let url1 = "https://pro-api.coinmarketcap.com/v3/fear-and-greed/historical";

    let response1 = client
        .get(url1)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .header("Accept", "application/json")
        .query(&[("limit", "1")])
        .send()
        .await?;

    println!("状态码: {}", response1.status());
    let text1 = response1.text().await?;
    println!("响应: {}", text1);

    // 测试2: 尝试获取Fear & Greed Index (最新数据端点)
    println!("\n📊 测试2: 获取Fear & Greed Index (最新数据)");
    let url2 = "https://pro-api.coinmarketcap.com/v3/fear-and-greed/latest";

    let response2 = client
        .get(url2)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .header("Accept", "application/json")
        .send()
        .await?;

    println!("状态码: {}", response2.status());
    let text2 = response2.text().await?;
    println!("响应: {}", text2);

    // 测试3: 测试基础API端点 (获取加密货币列表)
    println!("\n💰 测试3: 获取加密货币列表 (验证API密钥)");
    let url3 = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";

    let response3 = client
        .get(url3)
        .header("X-CMC_PRO_API_KEY", &api_key)
        .header("Accept", "application/json")
        .query(&[("start", "1"), ("limit", "5"), ("convert", "USD")])
        .send()
        .await?;

    println!("状态码: {}", response3.status());
    let text3 = response3.text().await?;

    // 尝试解析JSON以获取更好的输出
    match serde_json::from_str::<Value>(&text3) {
        Ok(json) => {
            if let Some(status) = json.get("status") {
                println!("API状态: {}", status);
            }
            if let Some(data) = json.get("data") {
                if let Some(array) = data.as_array() {
                    println!("获取到 {} 个币种数据", array.len());
                    for (i, coin) in array.iter().take(3).enumerate() {
                        if let (Some(name), Some(symbol)) = (coin.get("name"), coin.get("symbol")) {
                            println!("  {}. {} ({})", i + 1, name, symbol);
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("响应 (前500字符): {}", &text3[..text3.len().min(500)]);
        }
    }

    // 测试4: 尝试Alternative.me的免费Fear & Greed API
    println!("\n🆓 测试4: Alternative.me Fear & Greed Index (免费API)");
    let url4 = "https://api.alternative.me/fng/";

    let response4 = client
        .get(url4)
        .header("Accept", "application/json")
        .send()
        .await?;

    println!("状态码: {}", response4.status());
    let text4 = response4.text().await?;
    println!("响应: {}", text4);

    Ok(())
}
