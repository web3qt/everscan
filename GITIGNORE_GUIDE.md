# .gitignore 文件说明

本文档解释了EverScan项目中`.gitignore`文件的配置和原因。

## 🚫 被忽略的文件类型

### 1. Rust 编译产物和缓存
```
/target/
/Cargo.lock
```
- **target/**: Rust编译产物目录，包含二进制文件和中间文件
- **Cargo.lock**: 依赖版本锁定文件，对于库项目通常不提交

### 2. 敏感配置文件
```
.env
.env.local
.env.production
.env.staging
config.toml
*.key
*.pem
*.crt
```
- **环境变量文件**: 包含API密钥、数据库密码等敏感信息
- **config.toml**: 包含数据库连接字符串和API密钥的配置文件
- **证书文件**: SSL证书和私钥文件

### 3. 数据库文件
```
*.db
*.sqlite
*.sqlite3
migrations/
```
- **本地数据库文件**: 开发环境的数据库文件
- **迁移文件**: 可能包含敏感的数据库结构信息

### 4. 日志和临时文件
```
*.log
logs/
log/
*.tmp
*.temp
.cache/
tmp/
temp/
```
- **日志文件**: 可能包含敏感的运行时信息
- **临时文件**: 系统和应用生成的临时文件

### 5. IDE和编辑器文件
```
.idea/
.vscode/
*.swp
*.swo
*~
.cursor/
.taskmaster/
```
- **IDE配置**: 个人开发环境配置，不应该共享
- **编辑器临时文件**: vim、emacs等编辑器的临时文件

### 6. 操作系统文件
```
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db
```
- **macOS**: .DS_Store等系统文件
- **Windows**: Thumbs.db等缩略图缓存

### 7. 加密货币相关敏感文件
```
wallet.dat
*.wallet
private_keys.txt
api_keys.txt
```
- **钱包文件**: 包含私钥的钱包文件
- **私钥文件**: 任何包含私钥的文件
- **API密钥**: 交易所和服务的API密钥

## ✅ 允许提交的文件

### 配置模板文件
```
!config.toml.example
```
- **config.toml.example**: 配置文件模板，不包含敏感信息

## 🔧 使用建议

### 1. 首次设置
```bash
# 复制配置模板
cp config.toml.example config.toml

# 编辑配置文件，添加你的API密钥和数据库连接信息
vim config.toml
```

### 2. 环境变量设置
```bash
# 创建环境变量文件
touch .env

# 添加必要的环境变量
echo "DATABASE_URL=postgresql://user:pass@localhost/everscan" >> .env
echo "COINGECKO_API_KEY=your_api_key_here" >> .env
```

### 3. 检查忽略状态
```bash
# 检查文件是否被正确忽略
git status

# 强制检查被忽略的文件
git status --ignored
```

## ⚠️ 安全提醒

1. **永远不要提交包含以下信息的文件**：
   - API密钥
   - 数据库密码
   - 私钥
   - 钱包文件
   - 个人配置

2. **如果意外提交了敏感文件**：
   ```bash
   # 从git历史中完全移除文件
   git filter-branch --force --index-filter \
   'git rm --cached --ignore-unmatch config.toml' \
   --prune-empty --tag-name-filter cat -- --all
   
   # 强制推送（危险操作，仅在确认后执行）
   git push origin --force --all
   ```

3. **定期检查**：
   - 定期审查`.gitignore`文件
   - 使用`git status --ignored`检查被忽略的文件
   - 确保敏感文件没有被意外跟踪

## 📝 维护

当添加新的功能或依赖时，记得更新`.gitignore`文件：

- 新的配置文件格式
- 新的编译产物
- 新的缓存目录
- 新的敏感文件类型

---

**记住**: `.gitignore`是你的第一道安全防线，正确配置它可以防止敏感信息泄露！ 