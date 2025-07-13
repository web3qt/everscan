# EverScan - 通用加密货币数据聚合平台

🚀 **EverScan** 是一个基于 Rust 的高性能区块链数据聚合平台，专注于提供多币种加密货币市场数据的实时监控和技术分析。

## ✨ 核心特性

### 🪙 多币种监控
- **可配置币种列表**: 通过配置文件轻松添加或删除要监控的加密货币
- **默认支持**: 比特币(BTC)、HYPE代币、以太坊(ETH)、币安币(BNB)、Solana(SOL)
- **统一数据源**: 使用 CoinGecko API 聚合全球300+交易所数据

### 📊 技术指标分析
- **布林带 (Bollinger Bands)**: 20周期，2倍标准差
- **RSI指标**: 14周期相对强弱指数
- **自动信号识别**: 超买/超卖信号自动检测
- **可配置参数**: 技术指标周期和阈值可通过配置文件调整

### ⚙️ 智能调度系统
- **定时数据收集**: 默认每4小时自动更新市场数据
- **错误重试机制**: 自动重试失败的API请求
- **健康检查**: 实时监控系统和API连接状态
- **灵活配置**: 支持自定义执行间隔和超时时间

### 🗄️ 数据存储
- **PostgreSQL集成**: 高性能数据库存储
- **结构化数据**: 标准化的指标数据格式
- **历史数据**: 支持长期数据存储和分析

## 🛠️ 快速开始

### 环境要求
- Rust 1.70+ 
- PostgreSQL 12+
- 网络连接（访问CoinGecko API）

### 安装步骤

1. **克隆项目**
```bash
git clone <repository-url>
cd everscan
```

2. **配置环境**
```bash
# 复制配置文件模板
cp config.toml.example config.toml

# 设置环境变量（可选）
cp .env.example .env
```

3. **配置币种监控**

编辑 `config.toml` 文件：

```toml
[crypto_monitoring]
# 要监控的币种列表（使用CoinGecko的币种ID）
coins = [
    "bitcoin",          # 比特币 (BTC)
    "hyperliquid",      # HYPE代币
    "ethereum",         # 以太坊 (ETH)
    "binancecoin",      # 币安币 (BNB)
    "solana",           # Solana (SOL)
    # 可以添加更多币种...
]

# 技术指标配置
[crypto_monitoring.technical_indicators]
rsi_period = 14         # RSI计算周期（天）
bollinger_period = 20   # 布林带计算周期（天）
bollinger_std_dev = 2.0 # 布林带标准差倍数

# 数据收集配置
[crypto_monitoring.data_collection]
history_days = 30       # 收集30天历史数据
enable_technical_indicators = true
```

4. **数据库配置**
```toml
[database]
url = "postgresql://username:password@localhost/everscan"
max_connections = 10
timeout_seconds = 30
```

5. **运行应用**
```bash
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/everscan
```

## 📋 配置指南

### 添加新的加密货币

要添加新的币种到监控列表：

1. 在 [CoinGecko](https://www.coingecko.com/) 上找到币种的ID
2. 将ID添加到 `config.toml` 的 `coins` 数组中
3. 重启应用

**示例**：添加 Cardano (ADA)
```toml
[crypto_monitoring]
coins = [
    "bitcoin",
    "hyperliquid", 
    "cardano",      # 新添加的ADA
    # ... 其他币种
]
```

### 调整监控频率

修改任务执行间隔：

```toml
[tasks.intervals]
crypto_market = 7200    # 2小时 = 7200秒
```

### 技术指标自定义

```toml
[crypto_monitoring.technical_indicators]
rsi_period = 21         # 使用21天RSI
bollinger_period = 25   # 使用25天布林带
bollinger_std_dev = 2.5 # 使用2.5倍标准差
```

## 🔧 API配置

### CoinGecko API
- **免费版本**: 每分钟10-50次请求
- **付费版本**: 更高的请求限制和更多功能

```toml
[api_keys]
coingecko_api_key = "your_api_key_here"  # 可选，提高请求限制
```

### 环境变量配置

创建 `.env` 文件：
```bash
# CoinGecko API密钥（可选）
COINGECKO_API_KEY=your_coingecko_api_key_here

# 数据库连接
DATABASE_URL=postgresql://username:password@localhost/everscan

# 日志级别
RUST_LOG=info
```


## 🏗️ 架构设计

### 核心组件

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Orchestrator  │    │   TaskManager   │    │  CoinGecko API  │
│   (调度器)       │◄──►│   (任务管理)     │◄──►│   (数据源)       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Configuration  │    │ CryptoMarketTask│    │ Technical       │
│  (配置管理)      │    │ (加密货币任务)    │    │ Indicators      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   PostgreSQL    │    │   Data Models   │    │   Metrics       │
│   (数据存储)     │    │   (数据模型)      │    │   (指标数据)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 数据流程

1. **配置加载**: 从 `config.toml` 读取币种列表和参数
2. **任务调度**: Orchestrator 根据配置创建 CryptoMarketTask
3. **数据获取**: 定时调用 CoinGecko API 获取市场数据
4. **技术分析**: 计算布林带和RSI等技术指标
5. **数据存储**: 将结构化数据保存到PostgreSQL
6. **状态监控**: 实时监控任务状态和API健康度

## 🚀 扩展功能

### 添加新的技术指标

1. 在 `src/clients/coingecko_client.rs` 中添加指标计算逻辑
2. 更新 `TechnicalIndicators` 结构体
3. 在配置文件中添加相关参数

### 集成其他数据源

项目架构支持轻松集成其他数据源：
- Binance API
- Coinbase API  
- 自定义数据源

### 添加预警系统

可以基于技术指标添加价格预警：
- RSI超买/超卖警报
- 价格突破布林带警报
- 自定义阈值警报

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🆘 支持

如果你遇到问题或有功能建议：

1. 查看现有的 [Issues](../../issues)
2. 创建新的 Issue 描述问题
3. 提供详细的错误信息和配置

## 🔮 路线图

- [ ] 添加更多技术指标 (MACD, KDJ等)
- [ ] Web界面dashboard
- [ ] 实时价格预警系统

---

**EverScan** - 让加密货币数据监控变得简单而强大！ 🚀 