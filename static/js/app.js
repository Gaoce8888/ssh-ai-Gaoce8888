// 主应用模块
import { TerminalManager } from './modules/terminal.js';
import { SSHConnection } from './modules/ssh.js';
import { AIChat } from './modules/ai-chat.js';
import { ConfigManager } from './modules/config.js';
import { UIManager } from './modules/ui.js';
import { Utils } from './modules/utils.js';

class SSHAITerminalApp {
    constructor() {
        this.terminal = null;
        this.sshConnection = null;
        this.aiChat = null;
        this.configManager = null;
        this.uiManager = null;
        this.isInitialized = false;
    }

    async init() {
        try {
            // 显示加载屏幕
            this.showLoadingScreen();

            // 初始化工具类
            Utils.init();

            // 初始化UI管理器
            this.uiManager = new UIManager();
            await this.uiManager.init();

            // 初始化配置管理器
            this.configManager = new ConfigManager();
            await this.configManager.init();

            // 初始化终端
            this.terminal = new TerminalManager('terminal');
            await this.terminal.init();

            // 初始化SSH连接
            this.sshConnection = new SSHConnection(this.terminal);
            await this.sshConnection.init();

            // 设置全局SSH连接引用，供AI聊天使用
            window.sshConnection = this.sshConnection;

            // 初始化AI聊天
            this.aiChat = new AIChat(this.terminal, this.sshConnection);
            await this.aiChat.init();

            // 绑定事件
            this.bindEvents();

            // 加载保存的配置
            await this.loadSavedConfigs();

            // 标记初始化完成
            this.isInitialized = true;

            // 隐藏加载屏幕
            this.hideLoadingScreen();

            // 显示主应用
            this.showApp();

        } catch (error) {
            console.error('应用初始化失败:', error);
            this.showError('应用初始化失败，请刷新页面重试');
        }
    }

    bindEvents() {
        // SSH表单提交
        const sshForm = document.getElementById('ssh-form');
        sshForm.addEventListener('submit', (e) => {
            e.preventDefault();
            this.handleSSHConnect();
        });

        // 保存配置按钮
        const saveConfigBtn = document.getElementById('save-config-btn');
        saveConfigBtn.addEventListener('click', () => this.handleSaveConfig());

        // AI表单提交
        const aiForm = document.getElementById('ai-form');
        aiForm.addEventListener('submit', (e) => {
            e.preventDefault();
            this.handleAISettingsSave();
        });

        // AI提供商切换
        const aiProvider = document.getElementById('ai-provider');
        aiProvider.addEventListener('change', (e) => {
            this.handleProviderChange(e.target.value);
        });

        // 标签切换
        const navTabs = document.querySelectorAll('.nav-tab');
        navTabs.forEach(tab => {
            tab.addEventListener('click', () => this.handleTabSwitch(tab));
        });

        // 菜单切换
        const menuToggle = document.getElementById('menu-toggle');
        menuToggle.addEventListener('click', () => this.uiManager.toggleSidebar());

        // 侧边栏关闭按钮
        const sidebarClose = document.querySelector('.sidebar-close');
        sidebarClose.addEventListener('click', () => this.uiManager.toggleSidebar());

        // 聊天窗口切换
        const chatHeader = document.querySelector('.chat-header');
        chatHeader.addEventListener('click', () => this.uiManager.toggleChat());

        // 快速操作按钮
        const quickActions = document.querySelectorAll('.quick-action');
        quickActions.forEach(action => {
            action.addEventListener('click', () => {
                const prompt = action.dataset.prompt;
                this.aiChat.sendQuickPrompt(prompt);
            });
        });

        // 窗口大小调整
        window.addEventListener('resize', Utils.debounce(() => {
            this.terminal.fit();
        }, 300));

        // 页面卸载前清理
        window.addEventListener('beforeunload', () => {
            this.cleanup();
        });
    }

    async handleSSHConnect() {
        const formData = Utils.getFormData('ssh-form');
        
        if (!formData.host || !formData.username || !formData.password) {
            this.showError('请填写所有必填字段');
            return;
        }

        try {
            if (this.sshConnection.isConnected) {
                await this.sshConnection.disconnect();
            } else {
                // 使用带诊断功能的连接方法
                await this.sshConnection.connectWithDiagnostics(formData, true);
            }
        } catch (error) {
            this.showError('连接失败: ' + error.message);
            // 在终端中也显示错误信息
            if (this.terminal) {
                this.terminal.writeln(`\r\n*** 连接失败: ${error.message} ***\r\n`);
            }
        }
    }

    async handleSaveConfig() {
        const name = prompt('请输入配置名称:');
        if (!name) return;

        const formData = Utils.getFormData('ssh-form');
        formData.name = name;

        try {
            await this.configManager.saveSSHConfig(formData);
            await this.loadSavedConfigs();
            this.showSuccess('配置保存成功');
        } catch (error) {
            this.showError('保存失败: ' + error.message);
        }
    }

    async handleAISettingsSave() {
        const formData = Utils.getFormData('ai-form');

        try {
            await this.configManager.saveAIConfig(formData);
            this.aiChat.updateConfig(formData);
            this.showSuccess('AI设置保存成功');
        } catch (error) {
            this.showError('保存失败: ' + error.message);
        }
    }

    handleProviderChange(provider) {
        const modelSelect = document.getElementById('ai-model');
        const openaiModels = document.getElementById('openai-models');
        const claudeModels = document.getElementById('claude-models');
        
        // 显示/隐藏相应的模型组
        if (provider === 'openai') {
            openaiModels.style.display = '';
            claudeModels.style.display = 'none';
            // 设置默认OpenAI模型
            modelSelect.value = 'gpt-3.5-turbo';
        } else if (provider === 'claude') {
            openaiModels.style.display = 'none';
            claudeModels.style.display = '';
            // 设置默认Claude模型为Claude 3.7 Sonnet (最新版本)
            modelSelect.value = 'claude-3.7-sonnet-20250224';
        }
        
        // 更新API Key占位符
        const apiKeyInput = document.getElementById('api-key');
        if (provider === 'openai') {
            apiKeyInput.placeholder = '输入您的OpenAI API Key';
        } else if (provider === 'claude') {
            apiKeyInput.placeholder = '输入您的Anthropic API Key';
        }
    }

    handleTabSwitch(tab) {
        const tabName = tab.dataset.tab;
        this.uiManager.switchTab(tabName);
    }

    async loadSavedConfigs() {
        try {
            // 加载SSH配置列表
            const sshConfigs = await this.configManager.getSSHConfigs();
            this.renderConfigList(sshConfigs);

            // 加载AI配置
            const aiConfig = await this.configManager.getAIConfig();
            if (aiConfig) {
                Utils.setFormData('ai-form', aiConfig);
                this.aiChat.updateConfig(aiConfig);
                
                // 触发提供商切换以正确显示模型选项
                if (aiConfig.provider) {
                    this.handleProviderChange(aiConfig.provider);
                }
            }
        } catch (error) {
            console.error('加载配置失败:', error);
        }
    }

    renderConfigList(configs) {
        const configList = document.getElementById('config-list');
        configList.innerHTML = '';

        configs.forEach((config, index) => {
            const configItem = document.createElement('div');
            configItem.className = 'config-item';
            configItem.innerHTML = `
                <span class="config-name">${Utils.escapeHtml(config.name)}</span>
                <div class="config-actions">
                    <button class="btn btn-secondary" data-index="${index}">加载</button>
                    <button class="btn btn-secondary" data-index="${index}">删除</button>
                </div>
            `;

            // 绑定加载按钮
            const loadBtn = configItem.querySelector('.btn:first-child');
            loadBtn.addEventListener('click', async () => {
                Utils.setFormData('ssh-form', config);
                this.showSuccess('配置已加载');
            });

            // 绑定删除按钮
            const deleteBtn = configItem.querySelector('.btn:last-child');
            deleteBtn.addEventListener('click', async () => {
                if (confirm('确定要删除这个配置吗？')) {
                    try {
                        await this.configManager.deleteSSHConfig(index);
                        await this.loadSavedConfigs();
                        this.showSuccess('配置已删除');
                    } catch (error) {
                        this.showError('删除失败: ' + error.message);
                    }
                }
            });

            configList.appendChild(configItem);
        });
    }

    showLoadingScreen() {
        const loadingScreen = document.getElementById('loading-screen');
        loadingScreen.style.display = 'flex';
    }

    hideLoadingScreen() {
        const loadingScreen = document.getElementById('loading-screen');
        loadingScreen.style.opacity = '0';
        setTimeout(() => {
            loadingScreen.style.display = 'none';
        }, 300);
    }

    showApp() {
        const app = document.getElementById('app');
        app.style.display = 'flex';
        app.style.opacity = '0';
        setTimeout(() => {
            app.style.opacity = '1';
        }, 10);
    }

    showError(message) {
        this.uiManager.showNotification(message, 'error');
    }

    showSuccess(message) {
        this.uiManager.showNotification(message, 'success');
    }

    cleanup() {
        if (this.sshConnection) {
            this.sshConnection.disconnect();
        }
        if (this.terminal) {
            this.terminal.dispose();
        }
    }
}

// 启动应用
document.addEventListener('DOMContentLoaded', () => {
    const app = new SSHAITerminalApp();
    app.init();
});

// 注册Service Worker (PWA支持)
if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
        navigator.serviceWorker.register('/sw.js')
            .then(registration => console.log('ServiceWorker 注册成功'))
            .catch(err => console.log('ServiceWorker 注册失败:', err));
    });
} 