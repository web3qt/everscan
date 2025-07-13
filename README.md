# EverScan 区块链数据聚合平台

🚀 **全新升级版本** - 多币种监控 + 贪婪恐惧指数 + 实时可视化界面

## 🌟 核心功能

### 📊 多币种实时监控
- **可配置币种列表** - 通过 `config.toml` 自由添加/删除监控币种
- **实时价格数据** - 来自 CoinGecko API 的最新市场数据
- **技术指标分析** - 布林带、RSI 等专业技术指标
- **自动信号检测** - 超买/超卖警告，价格突破提醒

### 😱 市场情绪分析
- **贪婪恐惧指数** - 来自 CoinMarketCap 的市场情绪指标
- **中文本地化** - 完整的中文情绪分类和投资建议
- **实时更新** - 与市场数据同步更新

### 🌐 现代化Web界面
- **响应式设计** - 支持桌面和移动设备
- **实时图表** - 基于 Chart.js 的动态价格图表
- **数据仪表板** - 直观的市场数据展示
- **WebSocket支持** - 实时数据推送

## 🚀 快速开始

### 1. 环境准备

确保您的系统已安装：
- **Rust** (1.70+): [安装指南](https://rustup.rs/)
- **Git**: 用于克隆代码库

### 2. 项目安装

```bash
# 克隆项目
git clone <your-repo-url>
cd everscan

# 编译项目
cargo build --release
```

### 3. 配置设置

#### 配置监控币种
编辑 `config.toml` 文件中的币种列表：

```toml
[crypto_monitoring]
# 要监控的币种列表（使用CoinGecko的币种ID）
coins = [
    \"bitcoin\",          # 比特币 (BTC)
    \"hyperliquid\",      # HYPE代币
    \"ethereum\",         # 以太坊 (ETH)
    \"binancecoin\",      # 币安币 (BNB)
    \"solana\",           # Solana (SOL)
]
```

#### 技术指标配置
```toml
[crypto_monitoring.technical_indicators]
rsi_period = 14         # RSI计算周期（天）
bollinger_period = 20   # 布林带计算周期（天）
bollinger_std_dev = 2.0 # 布林带标准差倍数
```

### 4. 运行应用

#### 测试模式（验证数据获取）
```bash
EVERSCAN_TEST_MODE=true cargo run
```

#### 生产模式（启动Web服务器）
```bash
cargo run
```

### 5. 访问界面

启动成功后，访问以下地址：

- **📊 主仪表板**: http://localhost:3000
- **🔗 API健康检查**: http://localhost:3000/api/health
- **📈 市场数据API**: http://localhost:3000/api/market-data
- **😱 贪婪恐惧指数API**: http://localhost:3000/api/fear-greed-index

## 📋 API 接口

### 市场数据接口

#### 获取所有市场数据
```bash
curl http://localhost:3000/api/market-data
```

#### 获取特定币种数据
```bash
curl http://localhost:3000/api/market-data/bitcoin
```

#### 获取贪婪恐惧指数
```bash
curl http://localhost:3000/api/fear-greed-index
```

#### 获取缓存统计
```bash
curl http://localhost:3000/api/stats
```


## 🔧 高级配置

### 数据收集间隔
在 `config.toml` 中配置任务执行间隔：

```toml
[tasks.intervals]
crypto_market = 14400   # 4小时采集一次市场数据
```

### 技术指标阈值
```toml
[crypto_monitoring.technical_indicators]
rsi_period = 14         # RSI计算周期
bollinger_period = 20   # 布林带计算周期
bollinger_std_dev = 2.0 # 布林带标准差倍数
```

### 历史数据配置
```toml
[crypto_monitoring.data_collection]
history_days = 30       # 收集30天历史数据用于技术指标计算
enable_technical_indicators = true  # 启用技术指标计算
```

## 📊 支持的币种

系统支持所有 CoinGecko 平台上的币种。常用币种ID：

| 币种名称 | CoinGecko ID | 符号 |
|---------|--------------|------|
| 比特币 | `bitcoin` | BTC |
| 以太坊 | `ethereum` | ETH |
| HYPE | `hyperliquid` | HYPE |
| 币安币 | `binancecoin` | BNB |
| Solana | `solana` | SOL |
| Cardano | `cardano` | ADA |
| Polkadot | `polkadot` | DOT |
| Chainlink | `chainlink` | LINK |

更多币种ID可在 [CoinGecko API](https://api.coingecko.com/api/v3/coins/list) 查询。

## 🛠️ 技术架构

### 数据存储策略
- **内存缓存** - 使用 `Arc<RwLock<HashMap>>` 实现高性能实时数据访问
- **数据生命周期** - 4小时到1天的数据有效期，适合实时监控需求
- **可选数据库** - 支持 PostgreSQL 用于历史数据存储和分析

### Web服务架构
- **Axum框架** - 现代化的异步Web框架
- **RESTful API** - 标准化的API接口设计
- **WebSocket支持** - 实时数据推送功能
- **静态文件服务** - 集成的前端资源服务

### 任务调度系统
- **启动时执行** - 应用启动时自动获取初始数据
- **可配置间隔** - 通过配置文件控制数据更新频率
- **错误恢复** - 自动重试和错误处理机制

## 🔍 故障排除

### 常见问题

#### 1. 编译错误
```bash
# 更新Rust工具链
rustup update

# 清理并重新编译
cargo clean
cargo build
```

#### 2. API限流错误
```
429 Too Many Requests - Rate Limit Exceeded
```
**解决方案**：
- CoinGecko免费API有请求限制
- 增加配置文件中的 `crypto_market` 间隔时间
- 考虑升级到付费API计划

#### 3. 端口占用错误
```
Address already in use (os error 48)
```
**解决方案**：
```bash
# 查找占用端口的进程
lsof -i :3000

# 杀掉进程
kill <PID>
```

#### 4. 数据获取失败
检查网络连接和API可用性：
```bash
# 测试CoinGecko API
curl \"https://api.coingecko.com/api/v3/ping\"

# 测试贪婪恐惧指数API
curl \"https://api.alternative.me/fng/\"
```

### 日志调试

启用详细日志：
```bash
RUST_LOG=debug cargo run
```

查看特定模块日志：
```bash
RUST_LOG=everscan::clients=debug cargo run
```

## 🔮 未来规划

### 即将推出的功能
- **更多技术指标** - MACD、KDJ、移动平均线等
- **价格预警系统** - 自定义价格阈值通知
- **历史数据分析** - 长期趋势分析和回测功能
- **多交易所数据** - 整合更多数据源
- **移动端应用** - React Native移动应用

### 扩展性考虑
- **微服务架构** - 支持服务拆分和独立部署
- **容器化部署** - Docker和Kubernetes支持
- **分布式缓存** - Redis集群支持
- **消息队列** - 异步任务处理优化

## 🤝 贡献指南

欢迎提交Issue和Pull Request！

### 开发环境设置
```bash
# 克隆项目
git clone <repo-url>
cd everscan

# 安装开发依赖
cargo install cargo-watch

# 开发模式运行（自动重载）
cargo watch -x run
```

### 代码规范
- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 添加适当的中文注释
- 遵循Rust最佳实践

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [CoinGecko](https://www.coingecko.com/) - 提供免费的加密货币数据API
- [CoinMarketCap](https://coinmarketcap.com/) - 提供贪婪恐惧指数数据
- [Rust社区](https://www.rust-lang.org/) - 优秀的编程语言和生态系统
- [Axum](https://github.com/tokio-rs/axum) - 现代化的Web框架

---

**💡 提示**: 这是一个开源项目，仅供学习和研究使用。投资有风险，请谨慎决策！ 