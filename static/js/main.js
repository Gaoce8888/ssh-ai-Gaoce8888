// 登录处理
class LoginManager {
    constructor() {
        this.form = document.getElementById('login-form');
        this.username = document.getElementById('username');
        this.password = document.getElementById('password');
        this.error = document.getElementById('login-error');
        
        this.init();
    }

    init() {
        if (this.form) {
            this.form.addEventListener('submit', (e) => this.handleSubmit(e));
        }
    }

    async handleSubmit(e) {
        e.preventDefault();
        
        try {
            const response = await fetch('http://localhost:8080/api/auth/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    username: this.username.value,
                    password: this.password.value
                })
            });

            if (response.ok) {
                const data = await response.json();
                localStorage.setItem('token', data.token);
                this.showMainPage();
            } else {
                this.showError('登录失败，请检查用户名和密码');
            }
        } catch (error) {
            this.showError('网络错误，请稍后重试');
            console.error('Login error:', error);
        }
    }

    showError(message) {
        this.error.textContent = message;
        this.error.style.display = 'block';
        setTimeout(() => {
            this.error.style.display = 'none';
        }, 3000);
    }

    showMainPage() {
        document.getElementById('login-page').style.display = 'none';
        document.getElementById('main-page').style.display = 'block';
    }
}

// 终端处理
class TerminalManager {
    constructor() {
        this.input = document.getElementById('command-input');
        this.sendBtn = document.getElementById('send-btn');
        this.output = document.querySelector('.terminal-output');
        this.ws = null;
        
        this.init();
    }

    init() {
        if (this.input) {
            this.input.addEventListener('keydown', (e) => this.handleKeyDown(e));
        }
        if (this.sendBtn) {
            this.sendBtn.addEventListener('click', () => this.handleSubmit());
        }
        this.connectWebSocket();
    }

    handleKeyDown(e) {
        if (e.key === 'Enter') {
            e.preventDefault();
            this.handleSubmit();
        }
    }

    handleSubmit() {
        const command = this.input.value.trim();
        if (command) {
            this.sendCommand(command);
            this.input.value = '';
        }
    }

    async sendCommand(command) {
        try {
            if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
                await this.connectWebSocket();
            }
            
            this.ws.send(JSON.stringify({
                type: 'command',
                data: command
            }));
            
            this.appendOutput(`> ${command}\n`, 'command');
        } catch (error) {
            this.appendOutput('Error: Connection failed', 'error');
            console.error('Send command error:', error);
        }
    }

    async connectWebSocket() {
        try {
            if (this.ws) {
                this.ws.close();
            }
            
            const token = localStorage.getItem('token');
            const wsUrl = `ws://localhost:8080/ws?token=${token}`;
            this.ws = new WebSocket(wsUrl);
            
            this.ws.onopen = () => {
                this.appendOutput('Connected to server', 'success');
            };
            
            this.ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                this.appendOutput(data.message, 'response');
            };
            
            this.ws.onclose = () => {
                this.appendOutput('Disconnected from server', 'error');
                setTimeout(() => this.connectWebSocket(), 5000);
            };
            
            this.ws.onerror = (error) => {
                this.appendOutput('WebSocket error', 'error');
                console.error('WebSocket error:', error);
            };
        } catch (error) {
            this.appendOutput('Connection failed', 'error');
            console.error('WebSocket connection error:', error);
        }
    }

    appendOutput(message, type = 'default') {
        const div = document.createElement('div');
        div.textContent = message;
        div.className = `output-${type}`;
        this.output.appendChild(div);
        this.scrollToBottom();
    }

    scrollToBottom() {
        this.output.scrollTop = this.output.scrollHeight;
    }
}

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    const loginManager = new LoginManager();
    const terminalManager = new TerminalManager();

    // 检查是否已登录
    const token = localStorage.getItem('token');
    if (token) {
        loginManager.showMainPage();
    }
});

// 性能优化
const performanceOptimization = {
    init() {
        // 减少重绘和重排
        this.optimizePainting();
        
        // 优化触摸事件
        this.optimizeTouch();
        
        // 优化滚动
        this.optimizeScroll();
    },

    optimizePainting() {
        // 添加will-change属性
        const elements = document.querySelectorAll('.terminal-output, .terminal-input');
        elements.forEach(el => {
            el.style.willChange = 'transform';
        });
    },

    optimizeTouch() {
        // 添加触摸事件优化
        const elements = document.querySelectorAll('input, button');
        elements.forEach(el => {
            el.style.touchAction = 'manipulation';
        });
    },

    optimizeScroll() {
        // 优化滚动行为
        const scrollable = document.querySelector('.terminal-output');
        if (scrollable) {
            scrollable.style.webkitOverflowScrolling = 'touch';
        }
    }
};

// 初始化性能优化
performanceOptimization.init();
