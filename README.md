# EverScan - 通用加密货币数据聚合平台

🚀 **EverScan** 是一个基于 Rust 的高性能区块链数据聚合平台，专注于提供多币种加密货币市场数据的实时监控和技术分析。

## ✨ 核心特性

### 🪙 可配置多币种监控
- **灵活的币种配置**: 通过 `config.toml` 文件轻松添加或删除要监控的加密货币
- **默认支持币种**: 比特币(BTC)、HYPE代币、以太坊(ETH)、币安币(BNB)、Solana(SOL)
- **统一数据源**: 使用 CoinGecko API 聚合全球300+交易所数据，避免管理多个交易所API的复杂性

### 📊 智能技术指标分析
- **布林带 (Bollinger Bands)**: 20周期，2倍标准差，自动识别价格突破信号
- **RSI指标**: 14周期相对强弱指数，自动检测超买/超卖信号
- **可配置参数**: 技术指标周期和阈值可通过配置文件调整
- **实时信号**: 自动识别并警告超买超卖状态

### 🏗️ 混合数据存储架构
我们采用了**内存缓存 + 可选数据库**的混合存储策略：

#### 📦 内存缓存（主要存储）
- **实时数据存储**: 使用 `Arc<RwLock<HashMap>>` 存储最新的市场数据
- **快速访问**: 毫秒级数据查询响应
- **生命周期管理**: 数据自动过期（4小时-1天）
- **线程安全**: 支持多线程并发读写

#### 💾 PostgreSQL（可选/历史数据）
- **长期存储**: 历史趋势数据和分析报告
- **数据分析**: 支持复杂查询和数据挖掘
- **可选配置**: 可以仅使用内存缓存运行，无需数据库

### 🌐 内置Web可视化界面
- **RESTful API**: 完整的API端点用于数据访问
- **实时仪表板**: 现代化的Web界面展示市场数据
- **WebSocket推送**: 实时数据更新，无需手动刷新
- **响应式设计**: 支持桌面和移动设备访问

### ⚙️ 智能调度系统
- **可配置间隔**: 默认每4小时自动更新，可自定义
- **错误重试机制**: 自动重试失败的API请求
- **API限制处理**: 智能处理CoinGecko API限制
- **健康检查**: 实时监控系统和API连接状态

## 🛠️ 快速开始

### 环境要求
- Rust 1.70+ 
- PostgreSQL 12+ (可选，仅用于历史数据)
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

# 编辑配置文件，添加你想监控的币种
nano config.toml
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
    # 添加更多币种...
]

# 技术指标配置
[crypto_monitoring.technical_indicators]
rsi_period = 14              # RSI计算周期
bollinger_period = 20        # 布林带周期
bollinger_std_dev = 2.0      # 布林带标准差倍数
rsi_overbought = 70.0        # RSI超买阈值
rsi_oversold = 30.0          # RSI超卖阈值
```

4. **运行应用**

**测试模式**（验证数据获取）：
```bash
EVERSCAN_TEST_MODE=true cargo run
```

**Web服务模式**（启动可视化界面）：
```bash
EVERSCAN_TEST_MODE=false WEB_PORT=3000 cargo run
```

5. **访问界面**
- 🌐 Web仪表板: http://localhost:3000
- 📡 API端点: http://localhost:3000/api
- 🔌 WebSocket: ws://localhost:3000/ws

## 📖 使用指南

### 🔧 配置管理

#### 添加新币种
1. 在 [CoinGecko](https://www.coingecko.com/) 上找到币种ID
2. 编辑 `config.toml` 文件，在 `coins` 数组中添加币种ID
3. 重启应用

#### 调整监控频率
```toml
[tasks.intervals]
crypto_market = 7200  # 2小时 = 7200秒
```

#### 技术指标配置
```toml
[crypto_monitoring.technical_indicators]
rsi_period = 21              # 更长的RSI周期
bollinger_period = 25        # 更长的布林带周期
bollinger_std_dev = 2.5      # 更宽的布林带
```

### 📊 API使用

#### 获取所有市场数据
```bash
curl http://localhost:3000/api/market-data
```

#### 获取特定币种数据
```bash
curl http://localhost:3000/api/market-data/bitcoin
```

#### 获取缓存统计
```bash
curl http://localhost:3000/api/stats
```

#### 健康检查
```bash
curl http://localhost:3000/api/health
```

### 🔌 WebSocket实时数据

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onmessage = function(event) {
    const marketData = JSON.parse(event.data);
    console.log('实时市场数据:', marketData);
};

// 发送ping保持连接
ws.send('ping');
```

## 🏗️ 架构设计

### 数据流程图
```
CoinGecko API → 数据获取 → 技术指标计算 → 内存缓存 → Web API/WebSocket
                    ↓
              PostgreSQL (可选)
```

### 核心组件

1. **数据获取层** (`src/clients/`)
   - CoinGecko API客户端
   - 自动重试和错误处理
   - API限制智能管理

2. **数据处理层** (`src/tasks/`)
   - 技术指标计算
   - 数据验证和清洗
   - 定时任务调度

3. **存储层** (`src/storage/`, `src/web/cache.rs`)
   - 内存缓存（主要）
   - PostgreSQL（可选）
   - 数据生命周期管理

4. **Web服务层** (`src/web/`)
   - RESTful API
   - WebSocket实时推送
   - 静态文件服务

## 🎯 使用场景

### 个人投资者
- 监控投资组合中的币种
- 获取技术指标信号
- 实时价格提醒

### 开发者
- 集成加密货币数据到应用
- 构建交易机器人
- 数据分析和研究

### 团队协作
- 共享市场数据源
- 统一的监控界面
- 历史数据分析

## 🚀 高级功能

### 批量数据获取
应用智能处理API限制，自动在请求间添加适当延迟，确保稳定的数据获取。

### 信号检测
- **超买信号**: RSI > 70 时自动警告
- **超卖信号**: RSI < 30 时自动警告
- **价格突破**: 价格突破布林带上下轨时提醒

### 性能优化
- 内存缓存提供毫秒级响应
- 异步处理避免阻塞
- 智能数据过期管理

## 🔒 安全考虑

- API密钥通过环境变量管理
- 配置文件已加入 `.gitignore`
- 无敏感信息硬编码

## 📈 扩展性

### 添加新数据源
1. 实现 `ApiClient` trait
2. 创建对应的任务类型
3. 在配置中启用

### 添加新技术指标
1. 在 `TechnicalIndicators` 结构体中添加字段
2. 实现计算逻辑
3. 更新API响应格式

### 添加新存储后端
1. 实现存储trait
2. 更新配置选项
3. 集成到应用流程

## 🛡️ 故障排除


### 日志调试
```bash
RUST_LOG=debug cargo run
```

## 🤝 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交更改
4. 发起 Pull Request

## 📄 许可证

MIT OR Apache-2.0

## 🙏 致谢

- [CoinGecko](https://www.coingecko.com/) 提供免费的加密货币数据API
- [Rust](https://www.rust-lang.org/) 提供安全高效的系统编程语言
- [Axum](https://github.com/tokio-rs/axum) 提供现代化的Web框架

---

**EverScan** - 让加密货币数据监控变得简单而强大 🚀 