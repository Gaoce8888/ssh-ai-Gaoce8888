# SSH AI Terminal 配置文件说明

## 服务器配置 (server)
```json
{
    "server": {
        "port": 8080,               // 服务端口
        "address": "127.0.0.1",    // 监听地址
        "log_level": "info"        // 日志级别: debug, info, warn, error
    }
}
```

## 数据库配置 (database)
```json
{
    "database": {
        "path": "data/db",          // 数据库存储路径
        "max_size": 2147483648,    // 最大存储大小 (字节)
        "sync_interval": 30        // 同步间隔 (秒)
    }
}
```

## 缓存配置 (cache)
```json
{
    "cache": {
        "capacity": 20000,          // 缓存容量
        "ttl": 7200                // 缓存过期时间 (秒)
    }
}
```

## AI配置 (ai)
```json
{
    "ai": {
        "providers": ["openai"],    // AI提供商列表
        "timeout": 30,             // 请求超时时间 (秒)
        "retry_count": 3           // 重试次数
    }
}
```

## SSH配置 (ssh)
```json
{
    "ssh": {
        "max_sessions": 500,       // 最大会话数
        "timeout": 600            // 会话超时时间 (秒)
    }
}
```

## 配置说明

### 服务器配置
- `port`: 服务监听的端口，默认8080
- `address`: 服务监听的地址，默认127.0.0.1（本地）
- `log_level`: 日志级别，可选值：debug, info, warn, error

### 数据库配置
- `path`: 数据库文件存储路径
- `max_size`: 数据库最大存储大小（字节）
- `sync_interval`: 数据同步间隔（秒）

### 缓存配置
- `capacity`: 缓存最大容量（条目数）
- `ttl`: 缓存项过期时间（秒）

### AI配置
- `providers`: 支持的AI提供商列表
- `timeout`: AI请求超时时间（秒）
- `retry_count`: 请求失败时的重试次数

### SSH配置
- `max_sessions`: 最大SSH会话数
- `timeout`: SSH会话超时时间（秒）

## 注意事项
1. 所有配置项都是可选的，未指定时使用默认值
2. 配置文件必须是有效的JSON格式
3. 修改配置后需要重启服务才能生效
4. 请确保配置文件的路径正确且可写
