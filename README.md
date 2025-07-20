# SSH-AI Terminal

一个高性能的 SSH AI 终端项目，支持通过 WebSocket 连接到 SSH 服务器，并集成了 AI 功能。

## 项目特性

- 🚀 **高性能架构**：基于 Rust 和 Tokio 异步运行时
- 🔌 **WebSocket 支持**：实时双向通信
- 🤖 **AI 集成**：支持 OpenAI 和 Claude 等 AI 服务
- 🔒 **安全连接**：SSH2 协议支持
- 📊 **性能监控**：实时性能指标和健康检查
- 💾 **智能缓存**：LRU 缓存策略，提升响应速度
- 🔄 **连接池**：SSH 连接复用，减少连接开销
- 📱 **移动端优化**：响应式设计，支持移动设备

## 快速开始

### 环境要求

- Rust 1.70+
- OpenSSL 开发库

### 安装依赖

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl pkg-config

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 编译运行

```bash
# 开发模式
cargo run

# 生产模式
cargo build --release
./target/release/ssh-ai-terminal
```

### 配置文件

编辑 `config.json` 文件来配置服务器：

```json
{
    "server": {
        "port": 8005,
        "address": "0.0.0.0"
    },
    "cache": {
        "capacity": 20000,
        "ttl": 7200
    },
    "ssh": {
        "max_sessions": 500,
        "timeout": 600
    }
}
```

## API 端点

### WebSocket 连接
- **URL**: `ws://localhost:8005/ws`
- **协议**: WebSocket

### REST API
- **健康检查**: `GET /health`
- **性能指标**: `GET /metrics`
- **AI 聊天**: `POST /api/ai/chat`

## WebSocket 消息格式

### 连接到 SSH 服务器
```json
{
    "msg_type": "connect",
    "connect": {
        "host": "example.com",
        "port": 22,
        "username": "user",
        "password": "password"
    }
}
```

### 执行命令
```json
{
    "msg_type": "command",
    "session_id": "uuid",
    "command": {
        "command": "ls -la"
    }
}
```

### AI 请求
```json
{
    "message": "如何查看系统日志？",
    "session_id": "uuid",
    "ai_config": {
        "provider": "openai",
        "api_key": "your-api-key"
    }
}
```

## 性能优化

项目包含多项性能优化：

1. **连接池管理**：复用 SSH 连接，减少建立连接的开销
2. **智能缓存**：缓存 AI 响应和命令结果
3. **并发优化**：使用 DashMap 和 RwLock 提高并发性能
4. **资源监控**：实时监控内存、CPU 使用情况
5. **自动清理**：定期清理过期会话和缓存

详细优化说明请查看 [OPTIMIZATION_GUIDE.md](OPTIMIZATION_GUIDE.md)

## 监控和调试

### 查看性能指标
```bash
curl http://localhost:8005/metrics
```

### 健康检查
```bash
curl http://localhost:8005/health
```

### 日志级别控制
```bash
export RUST_LOG=ssh_ai_terminal=debug
```

## 项目结构

```
src/
├── main.rs              # 主入口
├── models.rs            # 数据模型
├── ssh.rs              # SSH 连接处理
├── websocket.rs        # WebSocket 处理
├── ai.rs               # AI 集成
├── config.rs           # 配置管理
├── performance.rs      # 性能监控
├── cache.rs            # 缓存系统
└── connection_pool.rs  # 连接池
```

## 贡献指南

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
