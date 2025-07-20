// AI助手模块
class AIHelper {
    constructor() {
        this.providers = {
            openai: {
                model: 'gpt-3.5-turbo',
                maxTokens: 2000,
                temperature: 0.7
            }
        };
        this.conversationHistory = [];
        this.isProcessing = false;
        this.processingTimeout = null;
        
        this.init();
    }

    init() {
        // 初始化WebSocket监听
        websocketManager.addListener('message', (data) => {
            if (data.type === 'ai_response') {
                this.handleAIResponse(data);
            }
        });
    }

    async processCommand(command) {
        if (this.isProcessing) {
            return;
        }

        this.isProcessing = true;
        
        try {
            // 添加到对话历史
            this.conversationHistory.push({
                role: 'user',
                content: command
            });

            // 构建请求
            const request = {
                type: 'ai_process',
                data: {
                    providers: Object.keys(this.providers),
                    conversation: this.conversationHistory,
                    timeout: 30
                }
            };

            // 发送请求
            websocketManager.send(request);

            // 设置超时处理
            this.processingTimeout = setTimeout(() => {
                this.isProcessing = false;
                this.showError('AI处理超时');
            }, 30000);
        } catch (error) {
            this.isProcessing = false;
            this.showError('AI处理失败: ' + error.message);
        }
    }

    handleAIResponse(response) {
        clearTimeout(this.processingTimeout);
        this.isProcessing = false;

        if (response.error) {
            this.showError(response.error);
            return;
        }

        // 添加AI回复到对话历史
        this.conversationHistory.push({
            role: 'assistant',
            content: response.message
        });

        // 显示AI回复
        terminalManager.appendOutput(
            `AI: ${response.message}\n`,
            'ai-response'
        );
    }

    showError(message) {
        terminalManager.appendOutput(
            `Error: ${message}\n`,
            'error'
        );
    }

    clearHistory() {
        this.conversationHistory = [];
    }

    getProviders() {
        return Object.keys(this.providers);
    }

    setProviderConfig(provider, config) {
        if (this.providers[provider]) {
            Object.assign(this.providers[provider], config);
        }
    }
}

// 导出单例
const aiHelper = new AIHelper();
export default aiHelper;
