#!/bin/bash

echo "🚀 启动 EverScan - 端口 3000"
echo "============================="

# 检查环境变量
if [ ! -f ".env" ]; then
    echo "❌ 错误: .env 文件不存在"
    exit 1
fi

# 编译项目
echo "🔨 编译项目..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ 编译失败"
    exit 1
fi

echo "📡 启动服务器在端口 3000..."
echo "🌐 API端点: http://localhost:3000/api"
echo "📊 健康检查: http://localhost:3000/api/health"
echo "😱 贪婪恐惧指数: http://localhost:3000/api/fear-greed-index"
echo "🪙 山寨币季节指数: http://localhost:3000/api/altcoin-season-index"
echo ""
echo "按 Ctrl+C 停止服务器"
echo ""

# 启动应用程序
RUST_LOG=info ./target/release/everscan 