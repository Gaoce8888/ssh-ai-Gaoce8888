// AI聊天模块
export class AIChat {
    constructor(terminal, sshConnection) {
        this.terminal = terminal;
        this.sshConnection = sshConnection;
        this.config = {
            provider: 'openai',
            apiKey: '',
            model: 'gpt-3.5-turbo',
            maxTokens: 1000,
            endpoint: 'https://api.openai.com/v1/chat/completions'
        };
        this.autoExecute = false;
        this.terminalHistory = [];
        this.maxHistoryLength = 100;
        this.commandQueue = [];
    }

    async init() {
        this.setupEventListeners();
        this.loadAutoExecuteState();
        console.log('AI聊天模块初始化完成');
    }

    setupEventListeners() {
        // 聊天输入
        const chatInput = document.getElementById('chat-input');
        const chatSend = document.getElementById('chat-send');
        
        chatInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                this.sendMessage();
            }
        });
        
        chatSend.addEventListener('click', () => this.sendMessage());

        // 自动执行开关
        const autoExecuteToggle = document.getElementById('auto-execute');
        autoExecuteToggle.addEventListener('change', (e) => {
            this.autoExecute = e.target.checked;
            localStorage.setItem('aiAutoExecute', this.autoExecute);
            this.addSystemMessage(
                this.autoExecute ? 
                '自动执行模式已开启 - AI建议的命令将自动执行' : 
                '自动执行模式已关闭 - 需要手动确认执行命令'
            );
        });
    }

    loadAutoExecuteState() {
        const saved = localStorage.getItem('aiAutoExecute');
        this.autoExecute = saved === 'true';
        document.getElementById('auto-execute').checked = this.autoExecute;
    }

    updateConfig(config) {
        // 确保数值类型正确
        const processedConfig = { ...config };
        if (processedConfig.temperature !== undefined) {
            processedConfig.temperature = parseFloat(processedConfig.temperature);
        }
        if (processedConfig.maxTokens !== undefined) {
            processedConfig.maxTokens = parseInt(processedConfig.maxTokens);
        }
        
        this.config = { ...this.config, ...processedConfig };
        this.updateProviderDisplay();
    }

    updateProviderDisplay() {
        const currentProvider = document.getElementById('current-ai-provider');
        if (currentProvider) {
            currentProvider.textContent = this.config.provider === 'claude' ? 'Claude' : 'OpenAI';
        }
    }

    async sendMessage() {
        const input = document.getElementById('chat-input');
        const message = input.value.trim();
        
        if (!message) return;
        
        // 添加用户消息
        this.addMessage(message, 'user');
        input.value = '';
        
        // 发送到AI
        await this.processMessage(message);
    }

    async processMessage(message) {
        if (!this.config.apiKey) {
            this.addMessage('请先在AI设置中配置API Key', 'ai');
            return;
        }

        // 显示加载状态
        const loadingId = this.addLoadingMessage();

        try {
            // 获取终端上下文
            const context = this.getTerminalContext();
            
            // 调用AI API
            const response = await this.callAIAPI(message, context);
            
            // 移除加载消息
            this.removeMessage(loadingId);
            
            // 处理AI响应
            this.handleAIResponse(response);
            
        } catch (error) {
            console.error('AI处理失败:', error);
            this.removeMessage(loadingId);
            this.addMessage(`错误: ${error.message}`, 'ai');
        }
    }

    async callAIAPI(message, context) {
        const systemPrompt = `你是一个SSH终端助手。用户可能会询问关于终端操作的问题。
如果用户要求执行命令，请返回JSON格式：{"type": "command", "command": "要执行的命令", "description": "命令说明"}。
否则返回JSON格式：{"type": "text", "content": "回答内容"}。

当前终端上下文：
${context}`;

        // 使用后端统一API端点
        const response = await fetch('/api/ai/chat', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                message: message,
                session_id: this.getCurrentSessionId(),
                ai_config: {
                    provider: this.config.provider || 'openai',
                    apiKey: this.config.apiKey,
                model: this.config.model || 'gpt-3.5-turbo',
                    temperature: parseFloat(this.config.temperature) || 0.7,
                    maxTokens: parseInt(this.config.maxTokens) || 2048,
                    systemPrompt: systemPrompt
                }
            })
        });

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`API请求失败: ${response.status} ${response.statusText} - ${errorText}`);
        }

        const data = await response.json();
        return data.response;
    }

    getCurrentSessionId() {
        // 尝试从SSH连接获取会话ID
        if (window.sshConnection && window.sshConnection.sessionId) {
            return window.sshConnection.sessionId;
        }
        return null;
    }

    handleAIResponse(responseText) {
        try {
            const response = JSON.parse(responseText);
            
            if (response.type === 'command' && response.command) {
                // 处理命令响应
                this.addMessage(
                    `建议执行命令: \`${response.command}\`\n${response.description || ''}`, 
                    'ai'
                );
                
                if (this.autoExecute) {
                    this.executeCommand(response.command);
                    this.addSystemMessage(`正在自动执行命令: ${response.command}`);
                } else {
                    // 添加执行按钮
                    this.addCommandMessage(response.command);
                }
            } else if (response.type === 'text') {
                // 处理文本响应
                this.addMessage(response.content, 'ai');
            } else {
                // 兜底处理
                this.addMessage(responseText, 'ai');
            }
        } catch (e) {
            // 如果不是JSON，作为普通文本处理
            this.addMessage(responseText, 'ai');
        }
    }

    executeCommand(command) {
        if (!this.sshConnection.isConnected) {
            this.addSystemMessage('错误: SSH未连接');
            return;
        }

        // 记录命令
        this.commandQueue.push({
            command: command,
            timestamp: new Date(),
            output: ''
        });

        // 执行命令
        const success = this.sshConnection.sendCommand(command);
        if (success) {
            this.addToHistory(`$ ${command}\n`);
        }
    }

    addCommandMessage(command) {
        const messagesContainer = document.getElementById('chat-messages');
        const messageId = `msg-${Date.now()}`;
        
        const messageDiv = document.createElement('div');
        messageDiv.className = 'chat-message system';
        messageDiv.id = messageId;
        messageDiv.innerHTML = `
            <div class="message-content">
                <p>是否执行命令: <code>${this.escapeHtml(command)}</code>?</p>
                <div style="margin-top: 8px; display: flex; gap: 8px; justify-content: center;">
                    <button class="btn btn-primary" data-command="${this.escapeHtml(command)}">执行</button>
                    <button class="btn btn-secondary">取消</button>
                </div>
            </div>
        `;
        
        // 绑定按钮事件
        const executeBtn = messageDiv.querySelector('.btn-primary');
        const cancelBtn = messageDiv.querySelector('.btn-secondary');
        
        executeBtn.addEventListener('click', () => {
            this.executeCommand(command);
            messageDiv.remove();
            this.addSystemMessage(`执行命令: ${command}`);
        });
        
        cancelBtn.addEventListener('click', () => {
            messageDiv.remove();
        });
        
        messagesContainer.appendChild(messageDiv);
        this.scrollToBottom();
    }

    sendQuickPrompt(prompt) {
        const input = document.getElementById('chat-input');
        input.value = prompt;
        this.sendMessage();
    }

    addMessage(content, type) {
        const messagesContainer = document.getElementById('chat-messages');
        const messageId = `msg-${Date.now()}`;
        
        const messageDiv = document.createElement('div');
        messageDiv.className = `chat-message ${type}`;
        messageDiv.id = messageId;
        
        const messageContent = document.createElement('div');
        messageContent.className = 'message-content';
        messageContent.innerHTML = this.formatMessage(content);
        
        messageDiv.appendChild(messageContent);
        messagesContainer.appendChild(messageDiv);
        
        this.scrollToBottom();
        return messageId;
    }

    addSystemMessage(content) {
        return this.addMessage(content, 'system');
    }

    addLoadingMessage() {
        const messagesContainer = document.getElementById('chat-messages');
        const messageId = `msg-${Date.now()}`;
        
        const messageDiv = document.createElement('div');
        messageDiv.className = 'chat-message ai';
        messageDiv.id = messageId;
        messageDiv.innerHTML = `
            <div class="message-content">
                <div class="message-loading">
                    <span></span>
                    <span></span>
                    <span></span>
                </div>
            </div>
        `;
        
        messagesContainer.appendChild(messageDiv);
        this.scrollToBottom();
        return messageId;
    }

    removeMessage(messageId) {
        const message = document.getElementById(messageId);
        if (message) {
            message.remove();
        }
    }

    formatMessage(content) {
        // 转义HTML
        let formatted = this.escapeHtml(content);
        
        // 格式化代码块
        formatted = formatted.replace(/```(\w+)?\n([\s\S]*?)```/g, (match, lang, code) => {
            return `<pre><code class="language-${lang || 'plaintext'}">${code.trim()}</code></pre>`;
        });
        
        // 格式化内联代码
        formatted = formatted.replace(/`([^`]+)`/g, '<code>$1</code>');
        
        // 格式化链接
        formatted = formatted.replace(/https?:\/\/[^\s]+/g, (url) => {
            return `<a href="${url}" target="_blank" rel="noopener">${url}</a>`;
        });
        
        // 格式化换行
        formatted = formatted.replace(/\n/g, '<br>');
        
        return formatted;
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    scrollToBottom() {
        const messagesContainer = document.getElementById('chat-messages');
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }

    getTerminalContext() {
        // 获取最近的终端历史
        return this.terminalHistory.slice(-20).join('');
    }

    addToHistory(data) {
        this.terminalHistory.push(data);
        if (this.terminalHistory.length > this.maxHistoryLength) {
            this.terminalHistory.shift();
        }
    }

    clearHistory() {
        this.terminalHistory = [];
    }
} 