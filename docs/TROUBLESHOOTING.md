# 故障排除指南

## 概述

本文档提供SSH AI Terminal项目中常见问题的诊断和解决方案。

## 快速诊断

### 健康检查

```bash
# 检查服务状态
curl -f http://localhost:8005/api/performance/health

# 检查系统资源
free -h
df -h
top

# 检查网络连接
netstat -tlnp | grep 8005
ss -tlnp | grep 8005
```

### 日志查看

```bash
# 查看应用日志
tail -f logs/app.log

# 查看错误日志
tail -f logs/error.log

# 查看系统日志
sudo journalctl -u ssh-ai-terminal -f

# 查看Docker日志
docker logs -f ssh-ai-terminal
```

## 常见问题

### 1. 编译问题

#### 问题: 编译失败 - 缺少依赖

**错误信息:**
```
error: linking with `cc` failed: exit code: 1
/usr/bin/ld: cannot find -lssl
/usr/bin/ld: cannot find -lcrypto
```

**解决方案:**
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y libssl-dev pkg-config build-essential

# CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel pkgconfig

# macOS
brew install openssl pkg-config
```

#### 问题: object-pool 编译错误

**错误信息:**
```
error[E0554]: `#![feature(...)]` may not be used on the stable release channel
```

**解决方案:**
```bash
# 移除 object-pool 依赖
# 编辑 Cargo.toml，删除或注释掉 object-pool 行
# object-pool = "0.1"

# 重新编译
cargo clean
cargo build --release
```

#### 问题: 版本冲突

**错误信息:**
```
error: failed to select a version for `serde`.
    ... required by `ssh-ai-terminal`
    ... which satisfies dependency `ssh-ai-terminal` of `ssh-ai-terminal`
```

**解决方案:**
```bash
# 清理并重新构建
cargo clean
cargo update
cargo build
```

### 2. 运行时问题

#### 问题: 端口被占用

**错误信息:**
```
Error: Address already in use (os error 98)
```

**解决方案:**
```bash
# 查找占用端口的进程
sudo netstat -tlnp | grep 8005
sudo lsof -i :8005

# 杀死进程
sudo kill -9 <PID>

# 或者修改配置文件中的端口
# 编辑 config.json，修改 server.port
```

#### 问题: 权限不足

**错误信息:**
```
Permission denied (os error 13)
```

**解决方案:**
```bash
# 修复文件权限
sudo chown -R $USER:$USER /path/to/ssh-ai-terminal
sudo chmod -R 755 /path/to/ssh-ai-terminal

# 确保日志目录可写
mkdir -p logs
chmod 755 logs
```

#### 问题: 内存不足

**错误信息:**
```
thread 'main' panicked at 'out of memory'
```

**解决方案:**
```bash
# 检查内存使用
free -h

# 增加交换空间
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# 调整JVM参数（如果使用）
export RUST_MIN_STACK=8388608
```

### 3. SSH连接问题

#### 问题: SSH连接失败

**错误信息:**
```
SSH connection failed: Connection refused
```

**诊断步骤:**
```bash
# 1. 检查SSH服务状态
sudo systemctl status sshd

# 2. 测试SSH连接
ssh -v username@hostname

# 3. 检查防火墙
sudo ufw status
sudo iptables -L

# 4. 检查SSH配置
sudo cat /etc/ssh/sshd_config | grep -E "(Port|ListenAddress)"
```

**解决方案:**
```bash
# 启动SSH服务
sudo systemctl start sshd
sudo systemctl enable sshd

# 开放SSH端口
sudo ufw allow 22/tcp

# 检查SSH密钥权限
chmod 600 ~/.ssh/id_rsa
chmod 644 ~/.ssh/id_rsa.pub
```

#### 问题: SSH认证失败

**错误信息:**
```
Authentication failed: Invalid credentials
```

**解决方案:**
```bash
# 1. 验证用户名和密码
# 2. 检查SSH密钥
ssh-keygen -t rsa -b 4096 -C "your_email@example.com"

# 3. 添加公钥到服务器
ssh-copy-id username@hostname

# 4. 测试密钥认证
ssh -i ~/.ssh/id_rsa username@hostname
```

#### 问题: SSH会话超时

**错误信息:**
```
SSH session timeout
```

**解决方案:**
```bash
# 修改SSH客户端配置
cat >> ~/.ssh/config << EOF
Host *
    ServerAliveInterval 60
    ServerAliveCountMax 3
    TCPKeepAlive yes
EOF

# 修改应用配置
# 在 config.json 中增加 keep_alive 时间
```

### 4. WebSocket问题

#### 问题: WebSocket连接失败

**错误信息:**
```
WebSocket connection failed: Connection refused
```

**诊断步骤:**
```bash
# 1. 检查服务是否运行
curl -I http://localhost:8005

# 2. 检查WebSocket端点
curl -H "Connection: Upgrade" -H "Upgrade: websocket" http://localhost:8005/ws

# 3. 检查浏览器控制台错误
```

**解决方案:**
```bash
# 1. 确保服务正在运行
cargo run --release

# 2. 检查防火墙设置
sudo ufw allow 8005/tcp

# 3. 检查反向代理配置（如果使用）
```

#### 问题: WebSocket消息丢失

**错误信息:**
```
WebSocket message lost or corrupted
```

**解决方案:**
```bash
# 1. 实现重连机制
# 2. 添加消息确认
# 3. 增加心跳检测
# 4. 优化网络配置
```

### 5. AI集成问题

#### 问题: AI请求失败

**错误信息:**
```
AI request failed: API key invalid
```

**解决方案:**
```bash
# 1. 检查API密钥配置
# 2. 验证API密钥有效性
# 3. 检查网络连接
# 4. 查看API配额
```

#### 问题: AI响应超时

**错误信息:**
```
AI response timeout
```

**解决方案:**
```bash
# 1. 增加超时时间
# 2. 实现重试机制
# 3. 检查网络延迟
# 4. 优化请求大小
```

### 6. 性能问题

#### 问题: 高CPU使用率

**诊断:**
```bash
# 查看CPU使用情况
top -p $(pgrep ssh-ai-terminal)
htop

# 查看进程详情
ps aux | grep ssh-ai-terminal

# 使用perf分析
sudo perf top -p $(pgrep ssh-ai-terminal)
```

**解决方案:**
```bash
# 1. 优化代码逻辑
# 2. 减少不必要的计算
# 3. 使用缓存
# 4. 调整并发设置
```

#### 问题: 高内存使用率

**诊断:**
```bash
# 查看内存使用
free -h
cat /proc/meminfo

# 查看进程内存
ps aux | grep ssh-ai-terminal
cat /proc/$(pgrep ssh-ai-terminal)/status | grep VmRSS
```

**解决方案:**
```bash
# 1. 检查内存泄漏
# 2. 优化数据结构
# 3. 减少缓存大小
# 4. 增加系统内存
```

#### 问题: 连接数过多

**错误信息:**
```
Too many connections
```

**解决方案:**
```bash
# 1. 增加最大连接数
# 2. 实现连接池
# 3. 添加连接限制
# 4. 优化连接管理
```

### 7. 安全问题

#### 问题: 认证失败

**错误信息:**
```
Authentication failed: Invalid token
```

**解决方案:**
```bash
# 1. 检查JWT配置
# 2. 验证密钥设置
# 3. 检查时间同步
# 4. 重新生成密钥
```

#### 问题: 速率限制触发

**错误信息:**
```
Rate limit exceeded
```

**解决方案:**
```bash
# 1. 调整速率限制配置
# 2. 实现客户端重试
# 3. 优化请求频率
# 4. 增加限制阈值
```

## 调试技巧

### 1. 启用详细日志

```bash
# 设置详细日志级别
export RUST_LOG=debug
export RUST_BACKTRACE=1

# 启动应用
cargo run
```

### 2. 使用调试工具

```bash
# 使用strace跟踪系统调用
strace -f -e trace=network cargo run

# 使用gdb调试
gdb --args target/debug/ssh-ai-terminal

# 使用valgrind检查内存
valgrind --leak-check=full target/debug/ssh-ai-terminal
```

### 3. 网络调试

```bash
# 使用tcpdump捕获网络流量
sudo tcpdump -i any port 8005 -w capture.pcap

# 使用wireshark分析
wireshark capture.pcap

# 使用curl测试API
curl -v http://localhost:8005/api/performance/health
```

### 4. 性能分析

```bash
# 使用flamegraph生成火焰图
cargo install flamegraph
cargo flamegraph

# 使用perf分析性能
sudo perf record -g target/release/ssh-ai-terminal
sudo perf report
```

## 系统优化

### 1. 内核参数优化

```bash
# 编辑系统参数
sudo tee -a /etc/sysctl.conf << EOF
# 网络优化
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.tcp_fin_timeout = 30
net.ipv4.tcp_keepalive_time = 1200
net.ipv4.tcp_keepalive_intvl = 15
net.ipv4.tcp_keepalive_probes = 5

# 内存优化
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5

# 文件描述符
fs.file-max = 100000
EOF

# 应用参数
sudo sysctl -p
```

### 2. 文件描述符限制

```bash
# 编辑限制配置
sudo tee -a /etc/security/limits.conf << EOF
* soft nofile 65536
* hard nofile 65536
* soft nproc 32768
* hard nproc 32768
EOF

# 重新登录或重启
```

### 3. 系统服务优化

```bash
# 创建优化的systemd服务
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
Environment=RUST_BACKTRACE=1

# 资源限制
LimitNOFILE=65536
LimitNPROC=32768

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/ssh-ai-terminal/data /opt/ssh-ai-terminal/logs

[Install]
WantedBy=multi-user.target
EOF
```

## 监控和告警

### 1. 健康检查脚本

```bash
#!/bin/bash
# health_check.sh

HEALTH_URL="http://localhost:8005/api/performance/health"
LOG_FILE="/var/log/ssh-ai-health.log"

# 检查服务健康状态
response=$(curl -s -o /dev/null -w "%{http_code}" $HEALTH_URL)

if [ $response -eq 200 ]; then
    echo "$(date): Service is healthy" >> $LOG_FILE
    exit 0
else
    echo "$(date): Service is unhealthy (HTTP $response)" >> $LOG_FILE
    
    # 尝试重启服务
    sudo systemctl restart ssh-ai-terminal
    
    # 发送告警
    echo "SSH AI Terminal service is down!" | mail -s "Service Alert" admin@example.com
    
    exit 1
fi
```

### 2. 监控脚本

```bash
#!/bin/bash
# monitor.sh

# 检查CPU使用率
cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)

# 检查内存使用率
memory_usage=$(free | grep Mem | awk '{printf("%.2f", $3/$2 * 100.0)}')

# 检查磁盘使用率
disk_usage=$(df / | tail -1 | awk '{print $5}' | cut -d'%' -f1)

# 检查连接数
connections=$(netstat -an | grep :8005 | wc -l)

echo "CPU: ${cpu_usage}%"
echo "Memory: ${memory_usage}%"
echo "Disk: ${disk_usage}%"
echo "Connections: $connections"
```

### 3. 日志分析

```bash
#!/bin/bash
# log_analyzer.sh

LOG_FILE="logs/app.log"

# 统计错误数量
error_count=$(grep -c "ERROR" $LOG_FILE)

# 统计警告数量
warning_count=$(grep -c "WARN" $LOG_FILE)

# 统计请求数量
request_count=$(grep -c "Request" $LOG_FILE)

echo "Errors: $error_count"
echo "Warnings: $warning_count"
echo "Requests: $request_count"

# 分析错误模式
echo "Top errors:"
grep "ERROR" $LOG_FILE | cut -d' ' -f4- | sort | uniq -c | sort -nr | head -10
```

## 联系支持

如果以上解决方案无法解决您的问题，请提供以下信息：

1. **错误信息**: 完整的错误日志
2. **系统信息**: 操作系统版本、Rust版本
3. **配置信息**: 相关配置文件内容
4. **复现步骤**: 详细的问题复现步骤
5. **环境信息**: 开发/测试/生产环境

**联系方式:**
- GitHub Issues: [项目Issues页面](https://github.com/your-username/ssh-ai-terminal/issues)
- 邮件: support@example.com
- 文档: [项目文档](docs/)