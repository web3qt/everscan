# EverScan 区块链数据聚合平台

🚀 **EverScan** 是一个现代化的区块链数据聚合平台，专注于提供实时的加密货币市场数据、技术指标分析和市场情绪监控。

## ✨ 主要功能

### 📊 核心数据展示

- **HYPE代币监控**: 实时价格、24小时变化、交易量、市值
- **技术指标分析**: RSI指标、布林带上下轨
- **恐惧贪婪指数**: 市场情绪监控和投资建议
- **山寨季节指数**: 山寨币vs比特币表现对比分析

### 🎨 用户界面特色

- **响应式设计**: 支持桌面和移动设备
- **实时数据更新**: 每5分钟自动刷新
- **优雅的卡片布局**: 清晰的数据可视化
- **智能错误处理**: API失败时显示模拟数据
- **现代化UI**: 渐变背景、动画效果、阴影设计

### 🔧 技术架构

- **后端**: Rust + Axum Web框架
- **前端**: 原生HTML/CSS/JavaScript
- **数据源**: CoinMarketCap API
- **缓存系统**: 内存缓存优化性能
- **任务调度**: 异步数据采集任务

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- CoinMarketCap API密钥（可选，有免费额度）

### 安装步骤

1. **克隆项目**

```bash
git clone <repository-url>
cd everscan
```

2. **配置环境变量**

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，添加你的API密钥
COINMARKETCAP_API_KEY=your_api_key_here
```

3. **配置应用**

```bash
# 编辑 config.toml 文件
# 可以修改服务器端口、监控币种等配置
```

4. **构建和运行**

```bash
# 开发模式运行
cargo run --bin everscan

# 或者先构建再运行
cargo build --release
./target/release/everscan
```

5. **访问应用**

```
打开浏览器访问: http://localhost:3001
```

## 📁 项目结构

```
everscan/
├── src/
│   ├── bin/                    # 可执行文件
│   │   ├── everscan.rs         # 主程序入口
│   │   └── test_*.rs           # 测试程序
│   ├── clients/                # API客户端
│   │   └── coinmarketcap_client.rs
│   ├── models/                 # 数据模型
│   ├── tasks/                  # 数据采集任务
│   │   ├── crypto_market_task.rs
│   │   ├── fear_greed_task.rs
│   │   └── altcoin_season_task.rs
│   ├── web/                    # Web服务
│   │   ├── api.rs              # API路由
│   │   ├── cache.rs            # 数据缓存
│   │   └── websocket.rs        # WebSocket支持
│   ├── config.rs               # 配置管理
│   └── main.rs                 # 应用入口
├── static/                     # 静态文件
│   └── dashboard.html          # 前端页面
├── config.toml                 # 应用配置
├── Cargo.toml                  # Rust依赖配置
└── README.md                   # 项目文档
```

## 🔌 API 接口

### 健康检查

```
GET /api/health
```

### 市场数据

```
GET /api/market-data/{coin_id}    # 获取指定币种数据
GET /api/market-data              # 获取所有监控币种数据
```

### 市场指标

```
GET /api/fear-greed-index         # 恐惧贪婪指数
GET /api/altcoin-season-index     # 山寨季节指数
```

### 系统信息

```
GET /api/cache/stats              # 缓存统计信息
```

## ⚙️ 配置说明

### config.toml 主要配置项

```toml
[server]
host = "0.0.0.0"          # 服务器地址
port = 3001               # 服务器端口

[data_sources.coinmarketcap]
api_key = ""              # CoinMarketCap API密钥
timeout_seconds = 30      # 请求超时时间

[monitoring]
coins = ["hyperliquid"]   # 监控的币种列表
update_interval_seconds = 14400  # 数据更新间隔（4小时）
```

## 🧪 测试

### 运行测试程序

```bash
# 测试CoinMarketCap API连接
cargo run --bin test_coinmarketcap

# 测试恐惧贪婪指数
cargo run --bin test_fear_greed
```

### 测试模式运行

```bash
# 设置测试模式环境变量
export EVERSCAN_TEST_MODE=1
cargo run --bin everscan
```

## 📊 数据源

### CoinMarketCap API

- **HYPE代币数据**: 价格、交易量、市值等基础数据
- **恐惧贪婪指数**: 市场情绪指标
- **山寨季节指数**: 基于CMC 100指数计算

### 技术指标计算

- **RSI**: 相对强弱指数，14日周期
- **布林带**: 20日移动平均线 ± 2倍标准差
- **投资建议**: 基于技术指标的智能建议

## 🔄 数据更新机制

1. **定时任务**: 后台定时采集数据
2. **缓存系统**: 减少API调用，提高响应速度
3. **错误处理**: API失败时显示缓存数据或模拟数据
4. **自动刷新**: 前端每5分钟自动更新显示

## 🛠️ 开发指南

### 添加新的数据源

1. 在 `clients/` 目录创建新的客户端
2. 在 `tasks/` 目录创建对应的采集任务
3. 在 `main.rs` 中注册新任务
4. 更新前端页面显示

### 自定义监控币种

1. 编辑 `config.toml` 中的 `coins` 数组
2. 重启应用即可生效

## 🚨 故障排除

### 常见问题

1. **API连接失败**
   - 检查网络连接
   - 验证API密钥是否正确
   - 查看API配额是否用完

2. **页面显示错误**
   - 检查服务器是否正常运行
   - 查看浏览器控制台错误信息
   - 确认端口3001是否被占用

3. **数据不更新**
   - 查看服务器日志
   - 检查任务调度是否正常
   - 验证缓存是否工作正常

### 日志查看

```bash
# 设置日志级别
export RUST_LOG=debug
cargo run --bin everscan
```

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [CoinMarketCap](https://coinmarketcap.com/) - 提供可靠的加密货币数据API
- [Rust](https://www.rust-lang.org/) - 高性能系统编程语言
- [Axum](https://github.com/tokio-rs/axum) - 现代化的Rust Web框架

---

**EverScan** - 让区块链数据触手可及 🚀
