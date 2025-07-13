# 🌟 EverScan - 区块链数据聚合平台

EverScan 是一个现代化的区块链数据聚合平台，专注于实时监控加密货币市场数据和技术指标分析。

## ✨ 特性

### 🚀 HYPE Token 专项监控
- **专注监控**: 当前专门监控 Hyperliquid (HYPE) 代币
- **实时数据**: 自动获取最新价格、交易量、市值信息
- **技术指标**: 内置 RSI、布林带等技术分析工具
- **智能缓存**: 高效的数据缓存机制，减少API调用

### 😱 市场情绪分析
- **恐惧贪婪指数**: 实时获取市场情绪指标
- **中文本地化**: 完整的中文情绪描述和投资建议
- **直观展示**: 动漫风格的圆形指数表盘

### 🎨 动漫风格界面
- **卡片式设计**: 现代化的卡片布局，信息一目了然
- **动漫美学**: 渐变色彩、圆角设计、动态效果
- **响应式布局**: 完美适配各种设备屏幕
- **实时刷新**: 自动更新数据，手动刷新按钮

### 🔧 技术架构
- **Rust 后端**: 高性能、内存安全的后端服务
- **Axum 框架**: 现代化的异步 Web 框架
- **多数据源**: 支持 CoinGecko、Alternative.me 等多个数据源
- **RESTful API**: 标准化的 API 接口设计

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- Cargo (随 Rust 安装)

### 安装运行

1. **克隆项目**
```bash
git clone <repository-url>
cd everscan
```

2. **配置设置**
```bash
# 复制配置文件
cp config.toml.example config.toml

# 编辑配置（可选）
# 默认配置已设置为只监控 HYPE 代币
```

3. **运行程序**
```bash
# 测试模式（显示详细日志）
RUST_LOG=info EVERSCAN_TEST_MODE=true cargo run

# 生产模式
RUST_LOG=info cargo run
```

4. **访问界面**
```
打开浏览器访问: http://localhost:3000
```

## 🎮 界面预览

### 主仪表板
- **HYPE Token 卡片**: 显示实时价格、24h涨跌、交易量、市值
- **技术指标**: RSI 进度条、布林带上下轨
- **恐惧贪婪指数**: 动态圆形表盘显示市场情绪

### 动漫风格特色
- 🌈 渐变背景和卡片边框
- ✨ 悬停动画效果
- 🎯 圆角设计和阴影效果
- 📱 响应式移动端适配

## 📊 API 接口

### HYPE 数据接口
```bash
# 获取 HYPE 代币数据
GET /api/market-data/hyperliquid

# 响应示例
{
  "success": true,
  "data": {
    "current_price": 47.93,
    "price_change_24h": 2.23,
    "total_volume": 412943395,
    "market_cap": 15979991260,
    "rsi": 77.75,
    "bollinger_upper": 48.44,
    "bollinger_lower": 45.05
  }
}
```

### 恐惧贪婪指数接口
```bash
# 获取恐惧贪婪指数
GET /api/fear-greed-index

# 响应示例
{
  "success": true,
  "data": {
    "value": 74,
    "classification": "Greed",
    "chinese_classification": "贪婪",
    "investment_advice": "市场贪婪，注意风险",
    "sentiment_chinese": "贪婪"
  }
}
```

### 系统状态接口
```bash
# 缓存统计
GET /api/stats

# 健康检查
GET /api/health
```

## ⚙️ 配置说明

### 监控币种配置
```toml
[crypto_monitoring]
# 当前只监控 HYPE 代币
coins = [
    "hyperliquid",      # HYPE代币
    # 可以添加更多币种:
    # "bitcoin",        # 比特币 (BTC)
    # "ethereum",       # 以太坊 (ETH)
]
```

### 技术指标配置
```toml
[crypto_monitoring.technical_indicators]
rsi_period = 14         # RSI计算周期（天）
bollinger_period = 20   # 布林带计算周期（天）
bollinger_std_dev = 2.0 # 布林带标准差倍数
```

## 🔧 开发说明

### 项目结构
```
src/
├── clients/           # API客户端
│   ├── coingecko_client.rs    # CoinGecko API
│   └── coinmarketcap_client.rs # 恐惧贪婪指数
├── tasks/            # 数据采集任务
│   ├── crypto_market_task.rs  # HYPE数据采集
│   └── fear_greed_task.rs     # 恐惧贪婪指数采集
├── web/              # Web服务
│   ├── api.rs        # API路由
│   ├── cache.rs      # 数据缓存
│   └── mod.rs        # Web服务器
└── static/           # 静态文件
    └── dashboard.html # 动漫风格前端界面
```

### 添加新币种
1. 在 `config.toml` 的 `coins` 数组中添加币种ID
2. 重启程序即可自动监控新币种

### 自定义前端
- 修改 `static/dashboard.html` 文件
- 支持完整的 HTML/CSS/JavaScript 自定义
- 内置响应式设计和动漫风格样式

## 🔄 数据更新机制

- **启动时执行**: 程序启动时自动执行所有任务获取初始数据
- **定时更新**: 每4小时自动更新一次数据
- **智能缓存**: 内存缓存减少API调用频率
- **手动刷新**: 前端提供手动刷新按钮

## 🎯 使用场景

- **个人投资**: 专注监控感兴趣的代币（如HYPE）
- **技术分析**: 实时技术指标分析
- **市场情绪**: 掌握整体市场恐惧贪婪情绪
- **数据看板**: 美观的数据展示界面

## 📝 更新日志

### v0.2.0 - HYPE专项版本
- ✅ 移除BTC等其他币种，专注HYPE监控
- ✅ 全新动漫风格卡片式界面
- ✅ 恐惧贪婪指数中文本地化
- ✅ 响应式设计和移动端适配
- ✅ 启动时自动数据获取

### v0.1.0 - 基础版本
- ✅ 多币种数据采集
- ✅ 技术指标计算
- ✅ RESTful API接口
- ✅ 基础Web界面

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

---