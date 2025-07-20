# API 参考文档

## 概述

SSH AI Terminal 提供RESTful API和WebSocket接口，支持SSH会话管理、AI交互和系统监控。

## 基础信息

- **基础URL**: `http://localhost:8005`
- **WebSocket URL**: `ws://localhost:8005/ws`
- **API版本**: v1
- **内容类型**: `application/json`

## 认证

### JWT Token认证

所有API请求（除了登录）都需要在请求头中包含JWT token：

```
Authorization: Bearer <your-jwt-token>
```

### 获取Token

```http
POST /api/auth/login
Content-Type: application/json

{
    "username": "your-username",
    "password": "your-password"
}
```

响应：
```json
{
    "success": true,
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600
}
```

## REST API

### 认证相关

#### 登录
```http
POST /api/auth/login
```

#### 登出
```http
POST /api/auth/logout
```

#### 刷新Token
```http
POST /api/auth/refresh
```

#### 验证Token
```http
GET /api/auth/verify
```

### SSH会话管理

#### 创建SSH会话
```http
POST /api/ssh/sessions
Content-Type: application/json

{
    "host": "192.168.1.100",
    "port": 22,
    "username": "user",
    "password": "password",
    "private_key": "optional-private-key-path"
}
```

响应：
```json
{
    "session_id": "uuid-string",
    "status": "connected",
    "created_at": "2024-01-01T00:00:00Z"
}
```

#### 获取会话列表
```http
GET /api/ssh/sessions
```

#### 获取会话详情
```http
GET /api/ssh/sessions/{session_id}
```

#### 关闭会话
```http
DELETE /api/ssh/sessions/{session_id}
```

#### 执行命令
```http
POST /api/ssh/sessions/{session_id}/execute
Content-Type: application/json

{
    "command": "ls -la",
    "timeout": 30
}
```

### AI交互

#### 发送AI请求
```http
POST /api/ai/chat
Content-Type: application/json

{
    "message": "解释这个命令的作用",
    "context": "ls -la",
    "session_id": "optional-session-id"
}
```

响应：
```json
{
    "response": "这个命令的作用是...",
    "suggestions": ["相关命令1", "相关命令2"],
    "confidence": 0.95
}
```

#### 获取AI历史
```http
GET /api/ai/history?session_id={session_id}&limit=50
```

#### 清除AI历史
```http
DELETE /api/ai/history?session_id={session_id}
```

### 性能监控

#### 获取系统状态
```http
GET /api/performance/status
```

响应：
```json
{
    "cpu_usage": 45.2,
    "memory_usage": 67.8,
    "active_connections": 25,
    "total_requests": 1234,
    "uptime": 3600
}
```

#### 获取性能指标
```http
GET /api/performance/metrics?duration=1h
```

#### 健康检查
```http
GET /api/performance/health
```

### 缓存管理

#### 获取缓存状态
```http
GET /api/cache/status
```

#### 清除缓存
```http
DELETE /api/cache/clear
```

#### 预热缓存
```http
POST /api/cache/warmup
```

## WebSocket API

### 连接

```javascript
const ws = new WebSocket('ws://localhost:8005/ws');
```

### 消息格式

所有WebSocket消息都使用JSON格式：

```json
{
    "type": "message_type",
    "data": {},
    "timestamp": "2024-01-01T00:00:00Z"
}
```

### 消息类型

#### 1. 认证消息
```json
{
    "type": "auth",
    "data": {
        "token": "jwt-token"
    }
}
```

#### 2. SSH连接
```json
{
    "type": "ssh_connect",
    "data": {
        "host": "192.168.1.100",
        "port": 22,
        "username": "user",
        "password": "password"
    }
}
```

#### 3. SSH命令执行
```json
{
    "type": "ssh_execute",
    "data": {
        "session_id": "uuid",
        "command": "ls -la"
    }
}
```

#### 4. AI请求
```json
{
    "type": "ai_request",
    "data": {
        "message": "解释这个命令",
        "context": "当前终端输出"
    }
}
```

#### 5. 终端输入
```json
{
    "type": "terminal_input",
    "data": {
        "session_id": "uuid",
        "input": "ls -la\n"
    }
}
```

### 响应消息

#### 1. SSH输出
```json
{
    "type": "ssh_output",
    "data": {
        "session_id": "uuid",
        "output": "total 8\ndrwxr-xr-x 2 user user 4096 Jan 1 00:00 .",
        "is_error": false
    }
}
```

#### 2. AI响应
```json
{
    "type": "ai_response",
    "data": {
        "response": "这个命令的作用是...",
        "suggestions": ["相关命令"],
        "confidence": 0.95
    }
}
```

#### 3. 错误消息
```json
{
    "type": "error",
    "data": {
        "code": "CONNECTION_FAILED",
        "message": "连接失败",
        "details": {}
    }
}
```

#### 4. 状态更新
```json
{
    "type": "status_update",
    "data": {
        "session_id": "uuid",
        "status": "connected",
        "timestamp": "2024-01-01T00:00:00Z"
    }
}
```

## 数据模型

### Session
```rust
pub struct Session {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}
```

### AIRequest
```rust
pub struct AIRequest {
    pub message: String,
    pub context: Option<String>,
    pub session_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

### AIResponse
```rust
pub struct AIResponse {
    pub response: String,
    pub suggestions: Vec<String>,
    pub confidence: f64,
    pub tokens_used: u32,
    pub processing_time: Duration,
}
```

### PerformanceMetrics
```rust
pub struct PerformanceMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: u32,
    pub total_requests: u64,
    pub average_response_time: Duration,
    pub error_rate: f64,
}
```

## 错误代码

| 代码 | 描述 | HTTP状态码 |
|------|------|------------|
| `AUTH_FAILED` | 认证失败 | 401 |
| `INVALID_TOKEN` | 无效的Token | 401 |
| `TOKEN_EXPIRED` | Token已过期 | 401 |
| `PERMISSION_DENIED` | 权限不足 | 403 |
| `SESSION_NOT_FOUND` | 会话不存在 | 404 |
| `CONNECTION_FAILED` | 连接失败 | 500 |
| `TIMEOUT` | 请求超时 | 408 |
| `RATE_LIMITED` | 请求过于频繁 | 429 |
| `INTERNAL_ERROR` | 内部服务器错误 | 500 |

## 速率限制

- **认证请求**: 5次/分钟
- **SSH连接**: 10次/分钟
- **AI请求**: 100次/小时
- **API请求**: 1000次/小时

## 示例代码

### JavaScript客户端

```javascript
class SSHClient {
    constructor(url) {
        this.ws = new WebSocket(url);
        this.setupEventHandlers();
    }

    setupEventHandlers() {
        this.ws.onopen = () => {
            console.log('Connected to SSH AI Terminal');
            this.authenticate();
        };

        this.ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            this.handleMessage(message);
        };
    }

    authenticate(token) {
        this.ws.send(JSON.stringify({
            type: 'auth',
            data: { token }
        }));
    }

    connectSSH(host, port, username, password) {
        this.ws.send(JSON.stringify({
            type: 'ssh_connect',
            data: { host, port, username, password }
        }));
    }

    executeCommand(sessionId, command) {
        this.ws.send(JSON.stringify({
            type: 'ssh_execute',
            data: { session_id: sessionId, command }
        }));
    }

    handleMessage(message) {
        switch (message.type) {
            case 'ssh_output':
                console.log('SSH Output:', message.data.output);
                break;
            case 'ai_response':
                console.log('AI Response:', message.data.response);
                break;
            case 'error':
                console.error('Error:', message.data.message);
                break;
        }
    }
}
```

### Python客户端

```python
import websocket
import json
import threading

class SSHClient:
    def __init__(self, url):
        self.ws = websocket.WebSocketApp(url)
        self.setup_handlers()
        
    def setup_handlers(self):
        self.ws.on_open = self.on_open
        self.ws.on_message = self.on_message
        self.ws.on_error = self.on_error
        
    def on_open(self, ws):
        print("Connected to SSH AI Terminal")
        
    def on_message(self, ws, message):
        data = json.loads(message)
        self.handle_message(data)
        
    def handle_message(self, message):
        msg_type = message.get('type')
        if msg_type == 'ssh_output':
            print(f"SSH Output: {message['data']['output']}")
        elif msg_type == 'ai_response':
            print(f"AI Response: {message['data']['response']}")
        elif msg_type == 'error':
            print(f"Error: {message['data']['message']}")
            
    def authenticate(self, token):
        self.ws.send(json.dumps({
            'type': 'auth',
            'data': {'token': token}
        }))
        
    def connect_ssh(self, host, port, username, password):
        self.ws.send(json.dumps({
            'type': 'ssh_connect',
            'data': {
                'host': host,
                'port': port,
                'username': username,
                'password': password
            }
        }))
        
    def run(self):
        self.ws.run_forever()
```

## 最佳实践

1. **错误处理**: 始终检查响应状态和错误代码
2. **重连机制**: 实现WebSocket自动重连
3. **速率限制**: 遵守API速率限制
4. **资源清理**: 及时关闭不需要的SSH会话
5. **日志记录**: 记录重要的操作和错误信息