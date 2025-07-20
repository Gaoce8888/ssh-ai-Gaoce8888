// SSH连接管理模块
export class SSHConnection {
    constructor(terminal) {
        this.terminal = terminal;
        this.ws = null;
        this.sessionId = null;
        this.isConnected = false;
        this.connectionConfig = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 3000;
        this.pingInterval = null;
        this.terminalDataHandler = null;
        this.resizeHandler = null;
        this.connectionTimeout = null;
    }

    async init() {
        // 初始化时的设置
        console.log('SSH连接模块初始化');
    }

    async connect(config) {
        if (this.isConnected) {
            throw new Error('已经连接到服务器');
        }

        this.connectionConfig = config;
        this.updateConnectionStatus('connecting');
        
        // 在终端显示连接状态
        this.terminal.writeln('\r\n*** 正在连接到 SSH 服务器... ***');
        this.terminal.writeln(`*** 目标: ${config.username}@${config.host}:${config.port} ***\r\n`);

        return new Promise((resolve, reject) => {
            try {
                // 构建WebSocket URL
                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const wsUrl = `${protocol}//${window.location.host}/ws`;
                
                this.ws = new WebSocket(wsUrl);
                
                // 设置连接超时
                this.connectionTimeout = setTimeout(() => {
                    if (this.ws) {
                    this.ws.close();
                    }
                    this.terminal.writeln('\r\n*** 连接超时 ***\r\n');
                    reject(new Error('连接超时 - 请检查网络连接'));
                }, 15000); // 15秒超时

                this.ws.onopen = () => {
                    console.log('WebSocket连接已建立');
                    this.terminal.writeln('*** WebSocket连接已建立 ***');
                    
                    // 发送SSH连接请求
                    this.ws.send(JSON.stringify({
                        type: 'connect',
                        host: config.host,
                        port: parseInt(config.port) || 22,
                        username: config.username,
                        password: config.password
                    }));
                    
                    this.terminal.writeln('*** 正在进行SSH认证... ***');
                };

                this.ws.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        this.handleMessage(data, resolve, reject);
                    } catch (error) {
                        console.error('处理消息失败:', error);
                        this.terminal.writeln(`\r\n*** 消息处理错误: ${error.message} ***\r\n`);
                    }
                };

                this.ws.onerror = (error) => {
                    if (this.connectionTimeout) {
                        clearTimeout(this.connectionTimeout);
                        this.connectionTimeout = null;
                    }
                    console.error('WebSocket错误:', error);
                    this.terminal.writeln('\r\n*** WebSocket连接错误 ***\r\n');
                    this.handleError(error);
                    reject(new Error('WebSocket连接失败'));
                };

                this.ws.onclose = (event) => {
                    if (this.connectionTimeout) {
                        clearTimeout(this.connectionTimeout);
                        this.connectionTimeout = null;
                    }
                    console.log('WebSocket连接已关闭, 代码:', event.code, '原因:', event.reason);
                    this.terminal.writeln(`\r\n*** WebSocket连接已关闭 (${event.code}) ***\r\n`);
                    this.handleClose(event);
                };

            } catch (error) {
                if (this.connectionTimeout) {
                    clearTimeout(this.connectionTimeout);
                    this.connectionTimeout = null;
                }
                this.terminal.writeln(`\r\n*** 连接初始化失败: ${error.message} ***\r\n`);
                reject(error);
            }
        });
    }

    handleMessage(data, resolve, reject) {
        switch (data.type) {
            case 'connected':
                if (this.connectionTimeout) {
                    clearTimeout(this.connectionTimeout);
                    this.connectionTimeout = null;
                }
                this.sessionId = data.session_id;
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.updateConnectionStatus('connected');
                this.setupTerminalHandlers();
                this.startPingInterval();
                
                this.terminal.writeln('\r\n*** SSH连接成功建立! ***');
                this.terminal.writeln('*** 终端已准备就绪 ***\r\n');
                
                if (resolve) {
                    resolve();
                }
                break;

            case 'data':
                if (data.data) {
                    this.terminal.write(data.data);
                }
                break;

            case 'error':
                if (this.connectionTimeout) {
                    clearTimeout(this.connectionTimeout);
                    this.connectionTimeout = null;
                }
                
                let errorMsg = data.message;
                console.error('SSH错误:', errorMsg);
                
                // 提供更友好的错误信息
                if (errorMsg.includes('网络超时') || errorMsg.includes('NetworkTimeout')) {
                    errorMsg = '网络连接超时 - 请检查目标服务器是否可达，端口是否开放';
                } else if (errorMsg.includes('认证失败') || errorMsg.includes('AuthenticationFailed')) {
                    errorMsg = 'SSH认证失败 - 请检查用户名和密码是否正确';
                } else if (errorMsg.includes('握手失败') || errorMsg.includes('HandshakeFailed')) {
                    errorMsg = 'SSH握手失败 - 目标服务器可能不支持SSH协议或服务未启动';
                } else if (errorMsg.includes('通道创建失败') || errorMsg.includes('ChannelCreationFailed')) {
                    errorMsg = 'SSH通道创建失败 - 服务器配置问题';
                }
                
                this.terminal.writeln(`\r\n*** 连接失败: ${errorMsg} ***`);
                this.terminal.writeln('*** 请检查连接配置和网络状态 ***\r\n');
                
                this.updateConnectionStatus('disconnected');
                
                if (reject && !this.isConnected) {
                    reject(new Error(errorMsg));
                }
                break;

            case 'disconnected':
                this.handleDisconnect();
                break;

            case 'pong':
                // 心跳响应
                break;

            default:
                console.warn('未知消息类型:', data.type);
        }
    }

    setupTerminalHandlers() {
        // 处理终端输入
        this.terminalDataHandler = this.terminal.onData((data) => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN && this.sessionId) {
                this.ws.send(JSON.stringify({
                    type: 'data',
                    session_id: this.sessionId,
                    data: data
                }));
            }
        });

        // 处理终端大小调整
        this.resizeHandler = this.terminal.onResize((size) => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN && this.sessionId) {
                this.ws.send(JSON.stringify({
                    type: 'resize',
                    session_id: this.sessionId,
                    cols: size.cols,
                    rows: size.rows
                }));
            }
        });
    }

    startPingInterval() {
        // 定期发送心跳包保持连接
        this.pingInterval = setInterval(() => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                this.ws.send(JSON.stringify({
                    type: 'ping',
                    session_id: this.sessionId
                }));
            }
        }, 30000); // 每30秒发送一次
    }

    stopPingInterval() {
        if (this.pingInterval) {
            clearInterval(this.pingInterval);
            this.pingInterval = null;
        }
    }

    async disconnect() {
        if (!this.isConnected) {
            return;
        }

        this.isConnected = false;
        this.stopPingInterval();

        if (this.terminalDataHandler) {
            this.terminalDataHandler.dispose();
            this.terminalDataHandler = null;
        }

        if (this.resizeHandler) {
            this.resizeHandler.dispose();
            this.resizeHandler = null;
        }

        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            if (this.sessionId) {
                this.ws.send(JSON.stringify({
                    type: 'disconnect',
                    session_id: this.sessionId
                }));
            }
            this.ws.close();
        }

        this.ws = null;
        this.sessionId = null;
        this.updateConnectionStatus('disconnected');
        this.terminal.writeln('\r\n*** 连接已断开 ***\r\n');
    }

    handleError(error) {
        console.error('连接错误:', error);
        this.updateConnectionStatus('error');
    }

    handleClose(event) {
        this.isConnected = false;
        this.stopPingInterval();
        
        if (this.terminalDataHandler) {
            this.terminalDataHandler.dispose();
            this.terminalDataHandler = null;
        }

        if (this.resizeHandler) {
            this.resizeHandler.dispose();
            this.resizeHandler = null;
        }

        // 检查是否需要重连
        if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
            this.attemptReconnect();
        } else {
            this.updateConnectionStatus('disconnected');
        }
    }

    handleDisconnect() {
        this.isConnected = false;
        this.sessionId = null;
        this.updateConnectionStatus('disconnected');
        this.terminal.writeln('\r\n*** SSH连接已断开 ***\r\n');
    }

    async attemptReconnect() {
        if (!this.connectionConfig || this.reconnectAttempts >= this.maxReconnectAttempts) {
            return;
        }

        this.reconnectAttempts++;
        this.updateConnectionStatus('reconnecting');
        
        this.terminal.writeln(`\r\n*** 尝试重新连接 (${this.reconnectAttempts}/${this.maxReconnectAttempts}) ***\r\n`);

        setTimeout(async () => {
            try {
                await this.connect(this.connectionConfig);
            } catch (error) {
                console.error('重连失败:', error);
            }
        }, this.reconnectDelay);
    }

    updateConnectionStatus(status) {
        const statusElement = document.getElementById('connection-status');
        const connectBtn = document.getElementById('connect-btn');
        
        if (!statusElement) return;

        switch (status) {
            case 'connected':
                statusElement.className = 'connection-status connected';
                statusElement.querySelector('.status-text').textContent = '已连接';
                if (connectBtn) {
                    connectBtn.textContent = '断开';
                }
                break;

            case 'connecting':
                statusElement.className = 'connection-status connecting';
                statusElement.querySelector('.status-text').textContent = '连接中...';
                if (connectBtn) {
                    connectBtn.disabled = true;
                }
                break;

            case 'reconnecting':
                statusElement.className = 'connection-status connecting';
                statusElement.querySelector('.status-text').textContent = '重新连接中...';
                break;

            case 'disconnected':
                statusElement.className = 'connection-status disconnected';
                statusElement.querySelector('.status-text').textContent = '未连接';
                if (connectBtn) {
                    connectBtn.textContent = '连接';
                    connectBtn.disabled = false;
                }
                break;

            case 'error':
                statusElement.className = 'connection-status disconnected';
                statusElement.querySelector('.status-text').textContent = '连接错误';
                if (connectBtn) {
                    connectBtn.disabled = false;
                }
                break;
        }
    }

    sendCommand(command) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN && this.sessionId) {
            this.ws.send(JSON.stringify({
                type: 'data',
                session_id: this.sessionId,
                data: command + '\n'
            }));
            return true;
        }
        return false;
    }

    getSessionInfo() {
        return {
            sessionId: this.sessionId,
            isConnected: this.isConnected,
            config: this.connectionConfig
        };
    }

    // 添加连接测试功能
    async testConnection(config) {
        this.terminal.writeln('\r\n*** 开始连接诊断 ***\r\n');
        
        // 1. 测试WebSocket连接
        this.terminal.writeln('1. 测试WebSocket连接...');
        const wsTestResult = await this.testWebSocketConnection();
        if (wsTestResult.success) {
            this.terminal.writeln('   ✓ WebSocket连接正常');
        } else {
            this.terminal.writeln(`   ✗ WebSocket连接失败: ${wsTestResult.error}`);
            return false;
        }
        
        // 2. 测试SSH服务器可达性
        this.terminal.writeln('\r\n2. 测试SSH服务器连接...');
        this.terminal.writeln(`   目标: ${config.host}:${config.port}`);
        this.terminal.writeln('   注意: 如果目标服务器无法访问，连接将会失败');
        
        return true;
    }
    
    async testWebSocketConnection() {
        return new Promise((resolve) => {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;
            const testWs = new WebSocket(wsUrl);
            
            const timeout = setTimeout(() => {
                testWs.close();
                resolve({ success: false, error: '连接超时' });
            }, 5000);
            
            testWs.onopen = () => {
                clearTimeout(timeout);
                testWs.close();
                resolve({ success: true });
            };
            
            testWs.onerror = (error) => {
                clearTimeout(timeout);
                resolve({ success: false, error: '连接错误' });
            };
        });
    }

    // 改进的连接方法，添加诊断选项
    async connectWithDiagnostics(config, enableDiagnostics = true) {
        if (enableDiagnostics) {
            const testResult = await this.testConnection(config);
            if (!testResult) {
                throw new Error('连接诊断失败，请检查网络配置');
            }
            this.terminal.writeln('\r\n*** 诊断完成，开始建立连接 ***\r\n');
        }
        
        return this.connect(config);
    }
} 