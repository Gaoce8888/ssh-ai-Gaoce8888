# 开发指南

## 概述

本文档为SSH AI Terminal项目的开发者提供详细的开发环境设置、编码规范、测试指南和贡献流程。

## 开发环境设置

### 系统要求

- **操作系统**: Linux, macOS, Windows
- **Rust**: 1.70+
- **Git**: 2.30+
- **编辑器**: VS Code, IntelliJ IDEA, Vim/Neovim
- **终端**: 支持ANSI转义序列的现代终端

### 环境准备

#### 1. 安装Rust

```bash
# 安装Rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 配置环境
source ~/.cargo/env

# 验证安装
rustc --version
cargo --version
```

#### 2. 安装开发工具

```bash
# 安装常用工具
cargo install cargo-edit    # 依赖管理
cargo install cargo-watch   # 文件监控
cargo install cargo-audit   # 安全审计
cargo install cargo-tarpaulin  # 代码覆盖率
cargo install cargo-fmt     # 代码格式化
cargo install cargo-clippy  # 代码检查

# 安装调试工具
cargo install cargo-expand  # 宏展开
cargo install cargo-tree    # 依赖树
```

#### 3. 配置IDE

**VS Code配置** (`settings.json`):
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.procMacro.enable": true,
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
        "source.fixAll": true
    }
}
```

**推荐的VS Code扩展**:
- rust-analyzer
- CodeLLDB
- Even Better TOML
- GitLens
- Error Lens

### 项目设置

#### 1. 克隆项目

```bash
git clone https://github.com/your-username/ssh-ai-terminal.git
cd ssh-ai-terminal

# 安装依赖
cargo build
```

#### 2. 配置开发环境

```bash
# 创建开发配置
cp config.json config.dev.json

# 编辑开发配置
nano config.dev.json
```

开发环境配置示例:
```json
{
    "server": {
        "port": 8005,
        "address": "127.0.0.1",
        "log_level": "debug"
    },
    "auth": {
        "enabled": false,
        "username": "dev",
        "password": "dev123"
    },
    "database": {
        "path": "data/dev.db"
    }
}
```

#### 3. 设置Git钩子

```bash
# 安装pre-commit钩子
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# 代码格式化
cargo fmt --all

# 代码检查
cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
cargo test

# 安全审计
cargo audit

echo "Pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

## 项目结构

```
ssh-ai-terminal/
├── src/                    # 源代码
│   ├── main.rs            # 主程序入口
│   ├── main_optimized.rs  # 优化版本主程序
│   ├── ssh.rs             # SSH连接管理
│   ├── websocket.rs       # WebSocket处理
│   ├── ai.rs              # AI集成
│   ├── config.rs          # 配置管理
│   ├── models.rs          # 数据模型
│   ├── connection_pool.rs # 连接池
│   ├── security.rs        # 安全模块
│   ├── performance.rs     # 性能监控
│   └── optimized/         # 优化模块
├── tests/                 # 测试文件
├── docs/                  # 文档
├── static/                # 静态文件
├── data/                  # 数据文件
├── logs/                  # 日志文件
├── scripts/               # 脚本文件
├── Cargo.toml            # 项目配置
├── Cargo.lock            # 依赖锁定
├── config.json           # 配置文件
├── README.md             # 项目说明
├── build_optimized.sh    # 构建脚本
└── benchmark.sh          # 基准测试脚本
```

## 编码规范

### Rust代码规范

#### 1. 命名规范

```rust
// 模块名: snake_case
mod ssh_connection;

// 结构体名: PascalCase
pub struct SshSession {
    // 字段名: snake_case
    pub session_id: String,
    pub host: String,
    pub port: u16,
}

// 函数名: snake_case
pub fn create_ssh_session(host: &str, port: u16) -> Result<SshSession, Error> {
    // 实现
}

// 常量名: SCREAMING_SNAKE_CASE
pub const MAX_CONNECTIONS: usize = 1000;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// 类型别名: PascalCase
pub type SessionId = String;
pub type ConnectionResult<T> = Result<T, ConnectionError>;
```

#### 2. 错误处理

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SshError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    
    #[error("认证失败: {0}")]
    AuthenticationFailed(String),
    
    #[error("超时: {operation}")]
    Timeout { operation: String },
    
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
}

// 使用Result类型
pub fn connect_ssh(host: &str, port: u16) -> Result<SshSession, SshError> {
    // 实现
}

// 使用?操作符
pub fn execute_command(session: &mut SshSession, cmd: &str) -> Result<String, SshError> {
    let output = session.execute(cmd)?;
    Ok(output)
}
```

#### 3. 异步编程

```rust
use tokio::time::{timeout, Duration};

// 异步函数
pub async fn async_ssh_connect(host: &str, port: u16) -> Result<SshSession, SshError> {
    let session = timeout(
        Duration::from_secs(30),
        SshSession::connect(host, port)
    ).await
    .map_err(|_| SshError::Timeout { 
        operation: "SSH连接".to_string() 
    })??;
    
    Ok(session)
}

// 并发处理
pub async fn handle_multiple_connections(hosts: Vec<String>) -> Vec<Result<SshSession, SshError>> {
    let futures: Vec<_> = hosts
        .into_iter()
        .map(|host| async_ssh_connect(&host, 22))
        .collect();
    
    futures::future::join_all(futures).await
}
```

#### 4. 文档注释

```rust
/// SSH会话管理器
/// 
/// 提供SSH连接的创建、管理和清理功能。
/// 
/// # 示例
/// 
/// ```rust
/// use ssh_ai_terminal::ssh::SshManager;
/// 
/// let manager = SshManager::new();
/// let session = manager.create_session("localhost", 22).await?;
/// ```
pub struct SshManager {
    // 字段
}

impl SshManager {
    /// 创建新的SSH会话
    /// 
    /// # 参数
    /// 
    /// * `host` - SSH服务器地址
    /// * `port` - SSH服务器端口
    /// 
    /// # 返回值
    /// 
    /// 返回包含会话信息的`SshSession`实例
    /// 
    /// # 错误
    /// 
    /// 如果连接失败，返回`SshError::ConnectionFailed`
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// let session = manager.create_session("192.168.1.100", 22).await?;
    /// ```
    pub async fn create_session(&self, host: &str, port: u16) -> Result<SshSession, SshError> {
        // 实现
    }
}
```

### 代码组织

#### 1. 模块结构

```rust
// lib.rs 或 main.rs
mod ssh {
    mod connection;
    mod session;
    mod manager;
    
    pub use connection::SshConnection;
    pub use session::SshSession;
    pub use manager::SshManager;
}

mod websocket {
    mod handler;
    mod protocol;
    
    pub use handler::WebSocketHandler;
    pub use protocol::Message;
}

mod ai {
    mod client;
    mod processor;
    
    pub use client::AiClient;
    pub use processor::AiProcessor;
}
```

#### 2. 特征(Trait)设计

```rust
use async_trait::async_trait;

/// SSH连接特征
#[async_trait]
pub trait SshConnection {
    /// 连接到SSH服务器
    async fn connect(&mut self) -> Result<(), SshError>;
    
    /// 执行命令
    async fn execute(&self, command: &str) -> Result<String, SshError>;
    
    /// 关闭连接
    async fn disconnect(&mut self) -> Result<(), SshError>;
    
    /// 检查连接状态
    fn is_connected(&self) -> bool;
}

/// AI处理器特征
#[async_trait]
pub trait AiProcessor {
    /// 处理AI请求
    async fn process(&self, request: &AiRequest) -> Result<AiResponse, AiError>;
    
    /// 获取处理统计
    fn get_stats(&self) -> AiStats;
}
```

## 测试指南

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    fn test_ssh_session_creation() {
        let session = SshSession::new("localhost", 22);
        assert_eq!(session.host, "localhost");
        assert_eq!(session.port, 22);
        assert!(!session.is_connected());
    }

    #[tokio::test]
    async fn test_async_ssh_connection() {
        let mut session = SshSession::new("localhost", 22);
        let result = session.connect().await;
        
        // 在测试环境中，连接可能失败，这是正常的
        match result {
            Ok(_) => assert!(session.is_connected()),
            Err(_) => assert!(!session.is_connected()),
        }
    }

    #[test]
    fn test_error_handling() {
        let error = SshError::ConnectionFailed("连接被拒绝".to_string());
        assert_eq!(error.to_string(), "连接失败: 连接被拒绝");
    }
}
```

### 集成测试

```rust
// tests/integration_test.rs
use ssh_ai_terminal::ssh::SshManager;
use ssh_ai_terminal::websocket::WebSocketHandler;

#[tokio::test]
async fn test_ssh_websocket_integration() {
    // 设置测试环境
    let manager = SshManager::new();
    let handler = WebSocketHandler::new(manager);
    
    // 测试WebSocket连接
    let result = handler.handle_connection("test_connection").await;
    assert!(result.is_ok());
}
```

### 性能测试

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_ssh_connection(c: &mut Criterion) {
    c.bench_function("ssh_connection", |b| {
        b.iter(|| {
            // 基准测试代码
            let session = SshSession::new(black_box("localhost"), black_box(22));
            session
        });
    });
}

criterion_group!(benches, benchmark_ssh_connection);
criterion_main!(benches);
```

### 测试运行

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_ssh_connection

# 运行集成测试
cargo test --test integration_test

# 运行基准测试
cargo bench

# 生成测试覆盖率报告
cargo tarpaulin --out Html
```

## 调试指南

### 日志配置

```rust
use tracing::{info, warn, error, debug};

// 初始化日志
fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter("ssh_ai_terminal=debug")
        .with_file(true)
        .with_line_number(true)
        .init();
}

// 使用日志
pub async fn connect_ssh(host: &str, port: u16) -> Result<SshSession, SshError> {
    info!("正在连接SSH服务器: {}:{}", host, port);
    
    match SshSession::connect(host, port).await {
        Ok(session) => {
            info!("SSH连接成功: {}:{}", host, port);
            Ok(session)
        }
        Err(e) => {
            error!("SSH连接失败: {}:{} - {}", host, port, e);
            Err(e)
        }
    }
}
```

### 调试工具

```bash
# 使用RUST_BACKTRACE获取详细错误信息
RUST_BACKTRACE=1 cargo run

# 使用RUST_LOG设置日志级别
RUST_LOG=debug cargo run

# 使用cargo-expand展开宏
cargo expand > expanded.rs

# 使用cargo-tree查看依赖关系
cargo tree
```

## 性能优化

### 内存优化

```rust
use std::sync::Arc;
use parking_lot::RwLock;

// 使用Arc共享数据
pub struct SshManager {
    sessions: Arc<RwLock<HashMap<String, SshSession>>>,
}

// 使用对象池
use object_pool::Pool;

pub struct ConnectionPool {
    pool: Pool<SshConnection>,
}

impl ConnectionPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: Pool::new(capacity, || SshConnection::new()),
        }
    }
    
    pub fn get_connection(&self) -> Option<PooledConnection> {
        self.pool.pull()
    }
}
```

### 并发优化

```rust
use tokio::sync::Semaphore;
use dashmap::DashMap;

pub struct ConcurrentSshManager {
    sessions: DashMap<String, SshSession>,
    semaphore: Arc<Semaphore>,
}

impl ConcurrentSshManager {
    pub async fn execute_command(&self, session_id: &str, command: &str) -> Result<String, SshError> {
        let _permit = self.semaphore.acquire().await.unwrap();
        
        if let Some(session) = self.sessions.get(session_id) {
            session.execute(command).await
        } else {
            Err(SshError::SessionNotFound)
        }
    }
}
```

## 贡献指南

### 开发流程

1. **Fork项目**
   ```bash
   git clone https://github.com/your-username/ssh-ai-terminal.git
   cd ssh-ai-terminal
   ```

2. **创建功能分支**
   ```bash
   git checkout -b feature/new-feature
   ```

3. **开发功能**
   - 遵循编码规范
   - 编写测试
   - 更新文档

4. **提交代码**
   ```bash
   git add .
   git commit -m "feat: 添加新功能描述"
   git push origin feature/new-feature
   ```

5. **创建Pull Request**
   - 填写PR模板
   - 描述功能和变更
   - 关联相关Issue

### 提交规范

使用[Conventional Commits](https://www.conventionalcommits.org/)规范:

```bash
# 功能提交
git commit -m "feat: 添加SSH连接池功能"

# 修复提交
git commit -m "fix: 修复WebSocket连接断开问题"

# 文档提交
git commit -m "docs: 更新API文档"

# 重构提交
git commit -m "refactor: 重构SSH会话管理"

# 测试提交
git commit -m "test: 添加连接池测试用例"

# 性能提交
git commit -m "perf: 优化内存使用"
```

### 代码审查

#### 审查清单

- [ ] 代码符合项目规范
- [ ] 包含必要的测试
- [ ] 文档已更新
- [ ] 性能影响已评估
- [ ] 安全性已考虑
- [ ] 向后兼容性已确认

#### 审查流程

1. **自动检查**
   - CI/CD流水线通过
   - 代码覆盖率达标
   - 安全检查通过

2. **人工审查**
   - 至少1名维护者审查
   - 解决所有审查意见
   - 获得批准后合并

### 发布流程

#### 版本管理

使用[语义化版本](https://semver.org/):

```bash
# 主版本号.次版本号.修订号
# 例如: 1.2.3

# 更新版本号
cargo set-version 1.2.3

# 创建发布标签
git tag -a v1.2.3 -m "Release version 1.2.3"
git push origin v1.2.3
```

#### 发布检查清单

- [ ] 所有测试通过
- [ ] 文档已更新
- [ ] 版本号已更新
- [ ] 变更日志已更新
- [ ] 发布说明已准备
- [ ] 安全审计已通过

## 工具和资源

### 开发工具

- **cargo-edit**: 依赖管理
- **cargo-watch**: 文件监控
- **cargo-audit**: 安全审计
- **cargo-tarpaulin**: 代码覆盖率
- **cargo-fmt**: 代码格式化
- **cargo-clippy**: 代码检查

### 有用的资源

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust Reference](https://doc.rust-lang.org/reference/)
- [Tokio Documentation](https://tokio.rs/)
- [Serde Documentation](https://serde.rs/)
- [ThisError Documentation](https://docs.rs/thiserror/)

### 社区资源

- [Rust Forum](https://users.rust-lang.org/)
- [Rust Discord](https://discord.gg/rust-lang)
- [Stack Overflow](https://stackoverflow.com/questions/tagged/rust)