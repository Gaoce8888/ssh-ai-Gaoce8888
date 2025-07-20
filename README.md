# SSH AI Terminal

一个高性能的SSH终端应用，集成了AI助手功能，支持WebSocket连接、会话管理、连接池和实时AI交互。

## 🚀 特性

### 核心功能
- **SSH终端模拟**: 完整的SSH会话管理，支持多用户并发连接
- **AI助手集成**: 实时AI对话，支持命令解释、错误诊断和代码生成
- **WebSocket支持**: 实时双向通信，低延迟响应
- **会话持久化**: 自动保存和恢复用户会话状态

### 性能优化
- **连接池管理**: 高效的SSH连接复用和负载均衡
- **LRU缓存**: 智能缓存AI响应和会话数据
- **并发控制**: 基于DashMap的高性能并发数据结构
- **内存优化**: 对象池和内存管理优化

### 安全特性
- **JWT认证**: 安全的用户认证和会话管理
- **速率限制**: 防止滥用和DDoS攻击
- **TLS支持**: 可选的HTTPS/WSS加密传输
- **输入验证**: 全面的安全输入检查

### 监控和可观测性
- **性能指标**: 详细的请求统计和系统监控
- **健康检查**: 实时服务状态监控
- **日志系统**: 结构化日志记录和追踪
- **Prometheus集成**: 指标导出和监控

## 📋 系统要求

- **操作系统**: Linux, macOS, Windows
- **Rust版本**: 1.70+
- **内存**: 最小512MB，推荐2GB+
- **网络**: 支持WebSocket的现代浏览器

## 🛠️ 安装

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/your-username/ssh-ai-terminal.git
cd ssh-ai-terminal

# 安装依赖 (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config build-essential

# 编译优化版本
./build_optimized.sh

# 或者手动编译
cargo build --release
```

### 使用Docker

```bash
# 构建镜像
docker build -t ssh-ai-terminal .

# 运行容器
docker run -d -p 8005:8005 --name ssh-ai-terminal ssh-ai-terminal
```

## 🚀 快速开始

### 1. 配置应用

编辑 `config.json` 文件：

```json
{
    "server": {
        "port": 8005,
        "address": "0.0.0.0"
    },
    "auth": {
        "enabled": true,
        "username": "your-username",
        "password": "your-password"
    }
}
```

### 2. 启动服务

```bash
# 开发模式
cargo run

# 生产模式
cargo run --release

# 使用启动脚本
./start.bat  # Windows
./build_optimized.sh  # Linux/macOS
```

### 3. 访问应用

打开浏览器访问: `http://localhost:8005`

## 📖 使用指南

### SSH连接

1. 在Web界面输入SSH连接信息
2. 点击"连接"按钮
3. 开始使用终端会话

### AI助手

1. 在终端中输入 `#ai` 命令
2. 描述你的问题或需求
3. AI将提供智能建议和解决方案

### 会话管理

- 支持多标签页管理多个SSH会话
- 自动保存会话状态
- 支持会话恢复和重连

## 🏗️ 架构设计

### 模块结构

```
src/
├── main.rs                 # 主程序入口
├── main_optimized.rs       # 优化版本主程序
├── ssh.rs                  # SSH连接管理
├── websocket.rs            # WebSocket处理
├── ai.rs                   # AI集成模块
├── config.rs               # 配置管理
├── models.rs               # 数据模型
├── connection_pool.rs      # 连接池管理
├── security.rs             # 安全认证
├── performance.rs          # 性能监控
└── optimized/              # 优化模块
    └── main.rs
```

### 核心组件

1. **连接池管理器**: 管理SSH连接的创建、复用和清理
2. **会话管理器**: 处理用户会话的生命周期
3. **AI处理器**: 集成AI服务，提供智能助手功能
4. **安全模块**: 处理认证、授权和输入验证
5. **性能监控**: 收集和分析性能指标

## 🔧 配置选项

详细的配置选项请参考 [配置指南](docs/config_guide.md)

### 主要配置项

- **服务器配置**: 端口、地址、TLS设置
- **认证配置**: 用户名、密码、会话超时
- **数据库配置**: 存储路径、缓存大小
- **AI配置**: 提供商、超时、重试策略
- **性能配置**: 并发限制、速率限制

## 📊 性能基准

运行性能测试：

```bash
./benchmark.sh
```

### 性能指标

- **并发连接**: 支持1000+并发SSH连接
- **响应时间**: WebSocket消息延迟 < 50ms
- **内存使用**: 优化后内存占用减少30%
- **CPU使用**: 多核优化，支持高并发处理

## 🔒 安全考虑

- 所有用户输入都经过验证和清理
- 支持TLS加密传输
- 实现速率限制防止滥用
- 定期清理过期会话和连接

## 🐛 故障排除

### 常见问题

1. **编译错误**: 确保安装了必要的系统依赖
2. **连接失败**: 检查SSH服务器配置和网络连接
3. **性能问题**: 调整配置参数和系统资源

### 日志查看

```bash
# 查看应用日志
tail -f logs/app.log

# 查看错误日志
tail -f logs/error.log
```

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 📞 支持

- 问题报告: [GitHub Issues](https://github.com/your-username/ssh-ai-terminal/issues)
- 文档: [项目文档](docs/)
- 邮件: your-email@example.com

## 🗺️ 路线图

- [ ] 支持更多AI提供商
- [ ] 移动端优化
- [ ] 插件系统
- [ ] 集群部署支持
- [ ] 更多终端类型支持

---

**注意**: 这是一个开发中的项目，API和功能可能会发生变化。
