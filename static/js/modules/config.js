// 配置管理模块
export class ConfigManager {
    constructor() {
        this.apiEndpoint = '/api_configs.json';
        this.configs = {
            ssh_configs: [],
            ai_config: null
        };
    }

    async init() {
        await this.loadConfigs();
        console.log('配置管理器初始化完成');
    }

    async loadConfigs() {
        try {
            const response = await fetch(this.apiEndpoint);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            
            const data = await response.json();
            this.configs = data;
            return data;
        } catch (error) {
            console.error('加载配置失败:', error);
            // 返回默认配置
            return {
                ssh_configs: [],
                ai_config: null
            };
        }
    }

    async saveConfigs() {
        try {
            const response = await fetch(this.apiEndpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(this.configs)
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const result = await response.json();
            if (!result.success) {
                throw new Error(result.message || '保存失败');
            }

            return true;
        } catch (error) {
            console.error('保存配置失败:', error);
            throw error;
        }
    }

    // SSH配置管理
    async getSSHConfigs() {
        // 优先从localStorage读取
        try {
            const savedConfigs = localStorage.getItem('sshConfigs');
            if (savedConfigs) {
                const configs = JSON.parse(savedConfigs);
                console.log('从localStorage加载SSH配置:', configs);
                this.configs.ssh_configs = configs;
                return configs;
            }
        } catch (error) {
            console.warn('从localStorage读取SSH配置失败:', error);
        }
        
        // 如果localStorage没有，则从默认配置加载
        await this.loadConfigs();
        return this.configs.ssh_configs || [];
    }

    async saveSSHConfig(config) {
        try {
            // 验证配置
            if (!config.name || !config.host || !config.username) {
                throw new Error('配置信息不完整');
            }

            // 获取现有配置
            await this.loadConfigs();
            if (!this.configs.ssh_configs) {
                this.configs.ssh_configs = [];
            }

            // 检查是否已存在同名配置
            const existingIndex = this.configs.ssh_configs.findIndex(c => c.name === config.name);
            if (existingIndex >= 0) {
                // 更新现有配置
                this.configs.ssh_configs[existingIndex] = config;
            } else {
                // 添加新配置
                this.configs.ssh_configs.push(config);
            }

            // 保存到localStorage
            localStorage.setItem('sshConfigs', JSON.stringify(this.configs.ssh_configs));
            
            console.log('SSH配置已保存到localStorage:', config);
            return true;
        } catch (error) {
            console.error('保存SSH配置失败:', error);
            throw new Error('保存SSH配置到本地存储失败');
        }
    }

    async deleteSSHConfig(index) {
        try {
            // 获取当前配置
            await this.getSSHConfigs();
            
            if (index < 0 || index >= this.configs.ssh_configs.length) {
                throw new Error('无效的配置索引');
            }

            // 从数组中删除配置
            this.configs.ssh_configs.splice(index, 1);
            
            // 保存到localStorage
            localStorage.setItem('sshConfigs', JSON.stringify(this.configs.ssh_configs));
            
            console.log('SSH配置已删除，索引:', index);
            return true;
        } catch (error) {
            console.error('删除SSH配置失败:', error);
            throw new Error('删除SSH配置失败');
        }
    }

    // AI配置管理
    async getAIConfig() {
        // 优先从localStorage读取
        try {
            const savedConfig = localStorage.getItem('aiConfig');
            if (savedConfig) {
                const config = JSON.parse(savedConfig);
                console.log('从localStorage加载AI配置:', config);
                return config;
            }
        } catch (error) {
            console.warn('从localStorage读取AI配置失败:', error);
        }
        
        // 如果localStorage没有，则使用默认配置
        await this.loadConfigs();
        return this.configs.ai_config || {
            provider: 'openai',
            apiKey: '',
            endpoint: 'https://api.openai.com/v1/chat/completions',
            model: 'gpt-3.5-turbo',
            maxTokens: 2048,
            temperature: 0.7
        };
    }

    async saveAIConfig(config) {
        try {
            // 直接保存到localStorage（客户端存储）
            localStorage.setItem('aiConfig', JSON.stringify(config));

            // 更新本地缓存
            this.configs.ai_config = config;
            
            console.log('AI配置已保存到localStorage:', config);
            return true;
        } catch (error) {
            console.error('保存AI配置失败:', error);
            throw new Error('保存AI配置到本地存储失败');
        }
    }

    // 导出配置
    exportConfigs() {
        const dataStr = JSON.stringify(this.configs, null, 2);
        const dataUri = 'data:application/json;charset=utf-8,' + encodeURIComponent(dataStr);
        
        const exportFileDefaultName = `ssh-ai-terminal-configs-${new Date().toISOString().slice(0, 10)}.json`;
        
        const linkElement = document.createElement('a');
        linkElement.setAttribute('href', dataUri);
        linkElement.setAttribute('download', exportFileDefaultName);
        linkElement.click();
    }

    // 导入配置
    async importConfigs(file) {
        return new Promise((resolve, reject) => {
            const reader = new FileReader();
            
            reader.onload = async (e) => {
                try {
                    const importedConfigs = JSON.parse(e.target.result);
                    
                    // 验证导入的数据结构
                    if (!importedConfigs.ssh_configs && !importedConfigs.ai_config) {
                        throw new Error('无效的配置文件格式');
                    }
                    
                    // 合并配置
                    if (importedConfigs.ssh_configs) {
                        this.configs.ssh_configs = [
                            ...this.configs.ssh_configs,
                            ...importedConfigs.ssh_configs
                        ];
                    }
                    
                    if (importedConfigs.ai_config) {
                        this.configs.ai_config = importedConfigs.ai_config;
                    }
                    
                    // 保存到服务器
                    await this.saveConfigs();
                    
                    resolve(true);
                } catch (error) {
                    reject(error);
                }
            };
            
            reader.onerror = () => {
                reject(new Error('读取文件失败'));
            };
            
            reader.readAsText(file);
        });
    }

    // 清除所有配置
    async clearAllConfigs() {
        if (!confirm('确定要清除所有配置吗？此操作不可恢复！')) {
            return false;
        }

        this.configs = {
            ssh_configs: [],
            ai_config: null
        };
        
        await this.saveConfigs();
        localStorage.removeItem('aiConfig');
        
        return true;
    }
} 