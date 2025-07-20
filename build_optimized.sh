#!/bin/bash

# SSH AI Terminal 优化构建脚本
# 用于构建高性能的生产版本

set -e

echo "🚀 开始优化构建 SSH AI Terminal..."

# 设置环境变量
export RUSTFLAGS="-C target-cpu=native -C target-feature=+crt-static"
export CARGO_PROFILE_RELEASE_OPT_LEVEL=3
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_PANIC="abort"
export CARGO_PROFILE_RELEASE_STRIP=true

# 检查系统架构
ARCH=$(uname -m)
echo "📋 系统架构: $ARCH"

# 根据架构设置优化标志
if [ "$ARCH" = "x86_64" ]; then
    export RUSTFLAGS="$RUSTFLAGS -C target-cpu=x86-64-v3"
    echo "🔧 使用 x86-64-v3 优化"
elif [ "$ARCH" = "aarch64" ]; then
    export RUSTFLAGS="$RUSTFLAGS -C target-cpu=native"
    echo "🔧 使用 ARM64 原生优化"
fi

# 清理之前的构建
echo "🧹 清理之前的构建..."
cargo clean

# 更新依赖
echo "📦 更新依赖..."
cargo update

# 检查代码
echo "🔍 检查代码..."
cargo check --release

# 运行测试
echo "🧪 运行测试..."
cargo test --release

# 构建优化版本
echo "🔨 构建优化版本..."
cargo build --release --bin ssh-ai-terminal

# 检查二进制文件大小
BINARY_SIZE=$(stat -c%s target/release/ssh-ai-terminal 2>/dev/null || stat -f%z target/release/ssh-ai-terminal 2>/dev/null || echo "unknown")
echo "📊 二进制文件大小: $BINARY_SIZE bytes"

# 运行基准测试（如果存在）
if [ -f "benches/main.rs" ]; then
    echo "⚡ 运行基准测试..."
    cargo bench
fi

# 创建发布包
echo "📦 创建发布包..."
mkdir -p dist
cp target/release/ssh-ai-terminal dist/
cp config.json dist/ 2>/dev/null || echo "⚠️  config.json 不存在"
cp -r static dist/ 2>/dev/null || echo "⚠️  static 目录不存在"

# 创建启动脚本
cat > dist/start.sh << 'EOF'
#!/bin/bash

# 设置环境变量
export RUST_LOG=info
export JWT_SECRET=${JWT_SECRET:-"your-secret-key-change-in-production"}

# 启动服务器
./ssh-ai-terminal
EOF

chmod +x dist/start.sh

# 创建 Dockerfile（可选）
cat > dist/Dockerfile << 'EOF'
FROM debian:bullseye-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# 复制二进制文件和配置
COPY ssh-ai-terminal /usr/local/bin/
COPY config.json /etc/ssh-ai-terminal/
COPY static /usr/local/share/ssh-ai-terminal/static

# 创建用户
RUN useradd -r -s /bin/false ssh-ai-terminal

# 设置权限
RUN chown -R ssh-ai-terminal:ssh-ai-terminal /etc/ssh-ai-terminal /usr/local/share/ssh-ai-terminal

# 切换到非特权用户
USER ssh-ai-terminal

# 暴露端口
EXPOSE 8005

# 启动命令
CMD ["/usr/local/bin/ssh-ai-terminal"]
EOF

# 创建 docker-compose.yml
cat > dist/docker-compose.yml << 'EOF'
version: '3.8'

services:
  ssh-ai-terminal:
    build: .
    ports:
      - "8005:8005"
    environment:
      - RUST_LOG=info
      - JWT_SECRET=your-secret-key-change-in-production
    volumes:
      - ./data:/app/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8005/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=200h'
      - '--web.enable-lifecycle'
EOF

# 创建 Prometheus 配置
cat > dist/prometheus.yml << 'EOF'
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ssh-ai-terminal'
    static_configs:
      - targets: ['ssh-ai-terminal:8005']
    metrics_path: '/metrics'
    scrape_interval: 5s
EOF

echo "✅ 构建完成！"
echo "📁 发布文件在 dist/ 目录中"
echo "🚀 运行: cd dist && ./start.sh"
echo "🐳 或使用 Docker: cd dist && docker-compose up -d"