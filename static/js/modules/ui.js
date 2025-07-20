// UI管理模块
export class UIManager {
    constructor() {
        this.sidebar = null;
        this.aiChat = null;
        this.notificationQueue = [];
        this.isProcessingNotification = false;
    }

    async init() {
        this.sidebar = document.getElementById('sidebar');
        this.aiChat = document.getElementById('ai-chat');
        
        // 恢复UI状态
        this.restoreUIState();
        
        // 初始化主题
        this.initTheme();
        
        console.log('UI管理器初始化完成');
    }

    // 侧边栏管理
    toggleSidebar() {
        if (!this.sidebar) return;
        
        const isCollapsed = this.sidebar.classList.toggle('collapsed');
        
        // 保存状态
        localStorage.setItem('sidebarCollapsed', isCollapsed);
        
        // 触发resize事件
        window.dispatchEvent(new Event('resize'));
        
        // 在移动端关闭其他面板
        if (this.isMobile() && !isCollapsed) {
            this.collapseChat();
        }
    }

    showSidebar() {
        if (this.sidebar) {
            this.sidebar.classList.remove('collapsed');
            localStorage.setItem('sidebarCollapsed', false);
        }
    }

    hideSidebar() {
        if (this.sidebar) {
            this.sidebar.classList.add('collapsed');
            localStorage.setItem('sidebarCollapsed', true);
        }
    }

    // AI聊天窗口管理
    toggleChat() {
        if (!this.aiChat) return;
        
        const isCollapsed = this.aiChat.classList.toggle('collapsed');
        const toggleIcon = document.getElementById('chat-toggle');
        
        if (toggleIcon) {
            toggleIcon.textContent = isCollapsed ? '▲' : '▼';
        }
        
        // 保存状态
        localStorage.setItem('chatCollapsed', isCollapsed);
        
        // 在移动端关闭其他面板
        if (this.isMobile() && !isCollapsed) {
            this.hideSidebar();
        }
    }

    expandChat() {
        if (this.aiChat) {
            this.aiChat.classList.remove('collapsed');
            const toggleIcon = document.getElementById('chat-toggle');
            if (toggleIcon) {
                toggleIcon.textContent = '▼';
            }
            localStorage.setItem('chatCollapsed', false);
        }
    }

    collapseChat() {
        if (this.aiChat) {
            this.aiChat.classList.add('collapsed');
            const toggleIcon = document.getElementById('chat-toggle');
            if (toggleIcon) {
                toggleIcon.textContent = '▲';
            }
            localStorage.setItem('chatCollapsed', true);
        }
    }

    // 标签切换
    switchTab(tabName) {
        // 更新标签按钮状态
        const tabs = document.querySelectorAll('.nav-tab');
        tabs.forEach(tab => {
            if (tab.dataset.tab === tabName) {
                tab.classList.add('active');
                tab.setAttribute('aria-selected', 'true');
            } else {
                tab.classList.remove('active');
                tab.setAttribute('aria-selected', 'false');
            }
        });
        
        // 更新标签内容
        const contents = document.querySelectorAll('.tab-content');
        contents.forEach(content => {
            if (content.id === `${tabName}-tab`) {
                content.classList.add('active');
            } else {
                content.classList.remove('active');
            }
        });
        
        // 保存当前标签
        localStorage.setItem('activeTab', tabName);
    }

    // 通知系统
    showNotification(message, type = 'info', duration = 3000) {
        const notification = {
            id: Date.now(),
            message,
            type,
            duration
        };
        
        this.notificationQueue.push(notification);
        
        if (!this.isProcessingNotification) {
            this.processNotificationQueue();
        }
    }

    async processNotificationQueue() {
        if (this.notificationQueue.length === 0) {
            this.isProcessingNotification = false;
            return;
        }
        
        this.isProcessingNotification = true;
        const notification = this.notificationQueue.shift();
        
        // 创建通知元素
        const notificationEl = this.createNotificationElement(notification);
        document.body.appendChild(notificationEl);
        
        // 动画显示
        setTimeout(() => {
            notificationEl.classList.add('show');
        }, 10);
        
        // 自动关闭
        setTimeout(() => {
            notificationEl.classList.remove('show');
            setTimeout(() => {
                notificationEl.remove();
                this.processNotificationQueue();
            }, 300);
        }, notification.duration);
    }

    createNotificationElement(notification) {
        const div = document.createElement('div');
        div.className = `notification notification-${notification.type}`;
        div.id = `notification-${notification.id}`;
        
        // 添加样式
        div.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            padding: 12px 20px;
            background: ${this.getNotificationColor(notification.type)};
            color: white;
            border-radius: 8px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
            z-index: 9999;
            max-width: 350px;
            transform: translateX(400px);
            transition: transform 0.3s ease;
            font-size: 14px;
        `;
        
        // 添加关闭按钮
        const closeBtn = document.createElement('button');
        closeBtn.innerHTML = '×';
        closeBtn.style.cssText = `
            position: absolute;
            top: 4px;
            right: 8px;
            background: none;
            border: none;
            color: white;
            font-size: 20px;
            cursor: pointer;
            opacity: 0.8;
        `;
        closeBtn.onclick = () => {
            div.classList.remove('show');
            setTimeout(() => div.remove(), 300);
        };
        
        div.textContent = notification.message;
        div.appendChild(closeBtn);
        
        // 添加显示类
        div.classList.add('notification-enter');
        
        return div;
    }

    getNotificationColor(type) {
        const colors = {
            success: '#4ade80',
            error: '#ef4444',
            warning: '#f59e0b',
            info: '#3b82f6'
        };
        return colors[type] || colors.info;
    }

    // 主题管理
    initTheme() {
        const savedTheme = localStorage.getItem('theme') || 'dark';
        this.setTheme(savedTheme);
    }

    setTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('theme', theme);
    }

    toggleTheme() {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
        this.setTheme(newTheme);
    }

    // UI状态恢复
    restoreUIState() {
        // 恢复侧边栏状态
        const sidebarCollapsed = localStorage.getItem('sidebarCollapsed') === 'true';
        if (sidebarCollapsed) {
            this.hideSidebar();
        }
        
        // 恢复聊天窗口状态
        const chatCollapsed = localStorage.getItem('chatCollapsed') !== 'false';
        if (chatCollapsed) {
            this.collapseChat();
        }
        
        // 恢复活动标签
        const activeTab = localStorage.getItem('activeTab') || 'ssh';
        this.switchTab(activeTab);
    }

    // 工具方法
    isMobile() {
        return window.innerWidth <= 768;
    }

    isTablet() {
        return window.innerWidth > 768 && window.innerWidth <= 1024;
    }

    isDesktop() {
        return window.innerWidth > 1024;
    }

    // 全屏管理
    toggleFullscreen() {
        if (!document.fullscreenElement) {
            document.documentElement.requestFullscreen().catch(err => {
                console.error('无法进入全屏模式:', err);
            });
        } else {
            document.exitFullscreen().catch(err => {
                console.error('无法退出全屏模式:', err);
            });
        }
    }

    // 加载指示器
    showLoading(message = '加载中...') {
        const loadingEl = document.createElement('div');
        loadingEl.id = 'global-loading';
        loadingEl.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: rgba(0, 0, 0, 0.8);
            color: white;
            padding: 20px 40px;
            border-radius: 8px;
            z-index: 10000;
            display: flex;
            align-items: center;
            gap: 12px;
        `;
        
        loadingEl.innerHTML = `
            <div class="loading-spinner" style="
                width: 20px;
                height: 20px;
                border: 2px solid #ffffff40;
                border-top-color: white;
                border-radius: 50%;
                animation: spin 1s linear infinite;
            "></div>
            <span>${message}</span>
        `;
        
        document.body.appendChild(loadingEl);
    }

    hideLoading() {
        const loadingEl = document.getElementById('global-loading');
        if (loadingEl) {
            loadingEl.remove();
        }
    }

    // 确认对话框
    async confirm(message, title = '确认') {
        return new Promise((resolve) => {
            const modal = document.createElement('div');
            modal.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                background: rgba(0, 0, 0, 0.5);
                display: flex;
                align-items: center;
                justify-content: center;
                z-index: 10000;
            `;
            
            modal.innerHTML = `
                <div style="
                    background: var(--bg-secondary);
                    padding: 24px;
                    border-radius: 8px;
                    max-width: 400px;
                    width: 90%;
                ">
                    <h3 style="margin-bottom: 16px; color: var(--text-primary);">${title}</h3>
                    <p style="margin-bottom: 24px; color: var(--text-secondary);">${message}</p>
                    <div style="display: flex; gap: 12px; justify-content: flex-end;">
                        <button class="btn btn-secondary" onclick="this.closest('div').remove(); window.uiConfirmResolve(false);">取消</button>
                        <button class="btn btn-primary" onclick="this.closest('div').remove(); window.uiConfirmResolve(true);">确认</button>
                    </div>
                </div>
            `;
            
            window.uiConfirmResolve = resolve;
            document.body.appendChild(modal);
        });
    }
}

// 添加通知显示样式
const style = document.createElement('style');
style.textContent = `
.notification.show {
    transform: translateX(0) !important;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}

@media (max-width: 768px) {
    .notification {
        right: 10px !important;
        left: 10px !important;
        max-width: none !important;
    }
}
`;
document.head.appendChild(style); 