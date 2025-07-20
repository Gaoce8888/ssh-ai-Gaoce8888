# 部署指南

## 概述

本文档介绍如何在不同环境中部署SSH AI Terminal应用，包括开发、测试和生产环境。

## 系统要求

### 最低要求
- **CPU**: 2核心
- **内存**: 2GB RAM
- **存储**: 10GB可用空间
- **网络**: 100Mbps带宽

### 推荐配置
- **CPU**: 4核心或更多
- **内存**: 8GB RAM
- **存储**: 50GB SSD
- **网络**: 1Gbps带宽

### 软件依赖
- **操作系统**: Linux (Ubuntu 20.04+, CentOS 8+), macOS 10.15+, Windows 10+
- **Rust**: 1.70+
- **OpenSSL**: 1.1.1+
- **Docker**: 20.10+ (可选)

## 部署方式

### 1. 直接部署

#### 环境准备

**Ubuntu/Debian:**
```bash
# 更新系统
sudo apt update && sudo apt upgrade -y

# 安装依赖
sudo apt install -y curl build-essential libssl-dev pkg-config

# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 验证安装
rustc --version
cargo --version
```

**CentOS/RHEL:**
```bash
# 安装依赖
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel pkgconfig

# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**macOS:**
```bash
# 安装Homebrew (如果未安装)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 安装依赖
brew install openssl pkg-config

# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### 应用部署

```bash
# 克隆项目
git clone https://github.com/your-username/ssh-ai-terminal.git
cd ssh-ai-terminal

# 编译优化版本
./build_optimized.sh

# 创建必要目录
mkdir -p logs data certs

# 配置应用
cp config.json.example config.json
# 编辑配置文件
nano config.json

# 启动应用
./target/release/ssh-ai-terminal
```

### 2. Docker部署

#### 创建Dockerfile

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# 编译应用
RUN cargo build --release

# 运行阶段
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制编译好的应用
COPY --from=builder /app/target/release/ssh-ai-terminal /app/
COPY --from=builder /app/config.json /app/
COPY --from=builder /app/static /app/static

# 创建必要目录
RUN mkdir -p logs data certs

# 暴露端口
EXPOSE 8005

# 启动应用
CMD ["./ssh-ai-terminal"]
```

#### 构建和运行

```bash
# 构建镜像
docker build -t ssh-ai-terminal .

# 运行容器
docker run -d \
  --name ssh-ai-terminal \
  -p 8005:8005 \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  -v $(pwd)/config.json:/app/config.json \
  ssh-ai-terminal

# 查看日志
docker logs -f ssh-ai-terminal
```

#### Docker Compose

创建 `docker-compose.yml`:

```yaml
version: '3.8'

services:
  ssh-ai-terminal:
    build: .
    container_name: ssh-ai-terminal
    ports:
      - "8005:8005"
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
      - ./config.json:/app/config.json
      - ./certs:/app/certs
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8005/api/performance/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  # 可选: 添加Redis缓存
  redis:
    image: redis:7-alpine
    container_name: ssh-ai-redis
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    restart: unless-stopped

volumes:
  redis_data:
```

运行:
```bash
docker-compose up -d
```

### 3. Kubernetes部署

#### 创建命名空间

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: ssh-ai-terminal
```

#### 创建ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: ssh-ai-config
  namespace: ssh-ai-terminal
data:
  config.json: |
    {
      "server": {
        "port": 8005,
        "address": "0.0.0.0"
      },
      "auth": {
        "enabled": true,
        "username": "admin",
        "password": "secure-password"
      }
    }
```

#### 创建Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ssh-ai-terminal
  namespace: ssh-ai-terminal
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ssh-ai-terminal
  template:
    metadata:
      labels:
        app: ssh-ai-terminal
    spec:
      containers:
      - name: ssh-ai-terminal
        image: ssh-ai-terminal:latest
        ports:
        - containerPort: 8005
        env:
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: config
          mountPath: /app/config.json
          subPath: config.json
        - name: data
          mountPath: /app/data
        - name: logs
          mountPath: /app/logs
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /api/performance/health
            port: 8005
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/performance/health
            port: 8005
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: ssh-ai-config
      - name: data
        persistentVolumeClaim:
          claimName: ssh-ai-data-pvc
      - name: logs
        persistentVolumeClaim:
          claimName: ssh-ai-logs-pvc
```

#### 创建Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: ssh-ai-terminal-service
  namespace: ssh-ai-terminal
spec:
  selector:
    app: ssh-ai-terminal
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8005
  type: LoadBalancer
```

#### 创建Ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: ssh-ai-terminal-ingress
  namespace: ssh-ai-terminal
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - ssh-ai.yourdomain.com
    secretName: ssh-ai-tls
  rules:
  - host: ssh-ai.yourdomain.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: ssh-ai-terminal-service
            port:
              number: 80
```

## 配置管理

### 环境变量

| 变量名 | 描述 | 默认值 |
|--------|------|--------|
| `RUST_LOG` | 日志级别 | `info` |
| `CONFIG_PATH` | 配置文件路径 | `config.json` |
| `DATA_PATH` | 数据目录路径 | `data/` |
| `LOG_PATH` | 日志目录路径 | `logs/` |
| `PORT` | 服务端口 | `8005` |
| `HOST` | 服务地址 | `0.0.0.0` |

### 配置文件

详细的配置选项请参考 [配置指南](config_guide.md)

### 生产环境配置

```json
{
    "server": {
        "port": 8005,
        "address": "0.0.0.0",
        "log_level": "warn",
        "max_connections": 2000,
        "keep_alive": 300,
        "compression": true,
        "tls": {
            "enabled": true,
            "cert_path": "/etc/ssl/certs/ssh-ai.crt",
            "key_path": "/etc/ssl/private/ssh-ai.key"
        }
    },
    "auth": {
        "enabled": true,
        "username": "admin",
        "password": "secure-password-hash",
        "session_timeout": 7200,
        "max_attempts": 3,
        "lockout_duration": 600
    },
    "performance": {
        "request_timeout": 60,
        "max_concurrent_requests": 500,
        "rate_limit": {
            "enabled": true,
            "window": 60,
            "max_requests": 2000
        }
    }
}
```

## 监控和日志

### 日志配置

```bash
# 创建日志目录
mkdir -p logs

# 配置日志轮转
cat > /etc/logrotate.d/ssh-ai-terminal << EOF
/path/to/ssh-ai-terminal/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 root root
    postrotate
        systemctl reload ssh-ai-terminal
    endscript
}
EOF
```

### 系统服务

创建systemd服务文件:

```bash
sudo tee /etc/systemd/system/ssh-ai-terminal.service << EOF
[Unit]
Description=SSH AI Terminal
After=network.target

[Service]
Type=simple
User=ssh-ai
Group=ssh-ai
WorkingDirectory=/opt/ssh-ai-terminal
ExecStart=/opt/ssh-ai-terminal/ssh-ai-terminal
Restart=always
RestartSec=10
Environment=RUST_LOG=info
Environment=CONFIG_PATH=/opt/ssh-ai-terminal/config.json

[Install]
WantedBy=multi-user.target
EOF

# 启用服务
sudo systemctl daemon-reload
sudo systemctl enable ssh-ai-terminal
sudo systemctl start ssh-ai-terminal
```

### 监控集成

#### Prometheus配置

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'ssh-ai-terminal'
    static_configs:
      - targets: ['localhost:8005']
    metrics_path: '/api/performance/metrics'
    scrape_interval: 15s
```

#### Grafana仪表板

创建Grafana仪表板配置:

```json
{
  "dashboard": {
    "title": "SSH AI Terminal Metrics",
    "panels": [
      {
        "title": "Active Connections",
        "type": "stat",
        "targets": [
          {
            "expr": "ssh_ai_active_connections",
            "legendFormat": "Connections"
          }
        ]
      },
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(ssh_ai_requests_total[5m])",
            "legendFormat": "Requests/sec"
          }
        ]
      }
    ]
  }
}
```

## 安全配置

### TLS/SSL配置

```bash
# 生成自签名证书 (开发环境)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# 使用Let's Encrypt (生产环境)
sudo certbot certonly --standalone -d ssh-ai.yourdomain.com
```

### 防火墙配置

```bash
# UFW (Ubuntu)
sudo ufw allow 8005/tcp
sudo ufw allow 22/tcp

# iptables (CentOS)
sudo iptables -A INPUT -p tcp --dport 8005 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT
```

### 反向代理 (Nginx)

```nginx
server {
    listen 80;
    server_name ssh-ai.yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name ssh-ai.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/ssh-ai.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ssh-ai.yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://localhost:8005;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}
```

## 备份和恢复

### 数据备份

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/ssh-ai-terminal"
DATE=$(date +%Y%m%d_%H%M%S)

# 创建备份目录
mkdir -p $BACKUP_DIR

# 备份数据
tar -czf $BACKUP_DIR/data_$DATE.tar.gz data/

# 备份配置
cp config.json $BACKUP_DIR/config_$DATE.json

# 备份日志
tar -czf $BACKUP_DIR/logs_$DATE.tar.gz logs/

# 清理旧备份 (保留30天)
find $BACKUP_DIR -name "*.tar.gz" -mtime +30 -delete
find $BACKUP_DIR -name "config_*.json" -mtime +30 -delete
```

### 数据恢复

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE=$1
BACKUP_DIR="/backup/ssh-ai-terminal"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file>"
    exit 1
fi

# 停止服务
sudo systemctl stop ssh-ai-terminal

# 恢复数据
tar -xzf $BACKUP_DIR/$BACKUP_FILE -C /

# 恢复配置
cp $BACKUP_DIR/config_$(echo $BACKUP_FILE | cut -d'_' -f2).json config.json

# 启动服务
sudo systemctl start ssh-ai-terminal
```

## 故障排除

### 常见问题

1. **端口被占用**
   ```bash
   # 检查端口使用情况
   sudo netstat -tlnp | grep 8005
   
   # 杀死占用进程
   sudo kill -9 <PID>
   ```

2. **权限问题**
   ```bash
   # 修复文件权限
   sudo chown -R ssh-ai:ssh-ai /opt/ssh-ai-terminal
   sudo chmod -R 755 /opt/ssh-ai-terminal
   ```

3. **内存不足**
   ```bash
   # 检查内存使用
   free -h
   
   # 调整系统参数
   echo 'vm.max_map_count=262144' | sudo tee -a /etc/sysctl.conf
   sudo sysctl -p
   ```

### 性能调优

```bash
# 调整文件描述符限制
echo '* soft nofile 65536' | sudo tee -a /etc/security/limits.conf
echo '* hard nofile 65536' | sudo tee -a /etc/security/limits.conf

# 调整内核参数
echo 'net.core.somaxconn = 65535' | sudo tee -a /etc/sysctl.conf
echo 'net.ipv4.tcp_max_syn_backlog = 65535' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

## 更新和升级

### 应用更新

```bash
#!/bin/bash
# update.sh

# 备份当前版本
./backup.sh

# 停止服务
sudo systemctl stop ssh-ai-terminal

# 更新代码
git pull origin main

# 重新编译
./build_optimized.sh

# 启动服务
sudo systemctl start ssh-ai-terminal

# 验证更新
curl -f http://localhost:8005/api/performance/health
```

### 回滚

```bash
#!/bin/bash
# rollback.sh

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

# 停止服务
sudo systemctl stop ssh-ai-terminal

# 切换到指定版本
git checkout $VERSION

# 重新编译
./build_optimized.sh

# 启动服务
sudo systemctl start ssh-ai-terminal
```