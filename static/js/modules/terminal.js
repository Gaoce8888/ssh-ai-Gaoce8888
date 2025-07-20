// 终端管理模块
export class TerminalManager {
    constructor(elementId) {
        this.elementId = elementId;
        this.terminal = null;
        this.fitAddon = null;
        this.webLinksAddon = null;
        this.searchAddon = null;
        this.isInitialized = false;
    }

    async init() {
        try {
            // 等待xterm.js加载完成
            await this.waitForXterm();

            // 创建终端实例
            this.terminal = new Terminal({
                fontSize: 14,
                fontFamily: 'Consolas, Monaco, "Courier New", monospace',
                cursorBlink: true,
                cursorStyle: 'block',
                bellStyle: 'none',
                scrollback: 10000,
                tabStopWidth: 4,
                theme: {
                    background: '#000000',
                    foreground: '#ffffff',
                    cursor: '#4a9eff',
                    cursorAccent: '#000000',
                    selection: 'rgba(74, 158, 255, 0.3)',
                    black: '#000000',
                    red: '#cd3131',
                    green: '#0dbc79',
                    yellow: '#e5e510',
                    blue: '#2472c8',
                    magenta: '#bc3fbc',
                    cyan: '#11a8cd',
                    white: '#e5e5e5',
                    brightBlack: '#666666',
                    brightRed: '#f14c4c',
                    brightGreen: '#23d18b',
                    brightYellow: '#f5f543',
                    brightBlue: '#3b8eea',
                    brightMagenta: '#d670d6',
                    brightCyan: '#29b8db',
                    brightWhite: '#ffffff'
                }
            });

            // 加载插件
            this.fitAddon = new FitAddon.FitAddon();
            this.terminal.loadAddon(this.fitAddon);

            // 打开终端
            const element = document.getElementById(this.elementId);
            if (!element) {
                throw new Error(`找不到元素: ${this.elementId}`);
            }
            this.terminal.open(element);

            // 适应容器大小
            this.fit();

            // 监听窗口大小变化
            this.setupResizeObserver();

            this.isInitialized = true;
            console.log('终端初始化成功');

        } catch (error) {
            console.error('终端初始化失败:', error);
            throw error;
        }
    }

    waitForXterm() {
        return new Promise((resolve, reject) => {
            let attempts = 0;
            const maxAttempts = 50;
            
            const checkXterm = () => {
                if (typeof Terminal !== 'undefined' && typeof FitAddon !== 'undefined') {
                    resolve();
                } else if (attempts >= maxAttempts) {
                    reject(new Error('xterm.js 加载超时'));
                } else {
                    attempts++;
                    setTimeout(checkXterm, 100);
                }
            };
            
            checkXterm();
        });
    }

    setupResizeObserver() {
        // 使用 ResizeObserver 监听容器大小变化
        if ('ResizeObserver' in window) {
            const container = document.getElementById(this.elementId).parentElement;
            const resizeObserver = new ResizeObserver(() => {
                this.fit();
            });
            resizeObserver.observe(container);
        }
    }

    fit() {
        if (this.fitAddon && this.isInitialized) {
            try {
                this.fitAddon.fit();
            } catch (error) {
                console.error('调整终端大小失败:', error);
            }
        }
    }

    write(data) {
        if (this.terminal) {
            this.terminal.write(data);
        }
    }

    writeln(data) {
        if (this.terminal) {
            this.terminal.writeln(data);
        }
    }

    clear() {
        if (this.terminal) {
            this.terminal.clear();
        }
    }

    focus() {
        if (this.terminal) {
            this.terminal.focus();
        }
    }

    blur() {
        if (this.terminal) {
            this.terminal.blur();
        }
    }

    onData(callback) {
        if (this.terminal) {
            return this.terminal.onData(callback);
        }
    }

    onResize(callback) {
        if (this.terminal) {
            return this.terminal.onResize(callback);
        }
    }

    getSize() {
        if (this.terminal) {
            return {
                cols: this.terminal.cols,
                rows: this.terminal.rows
            };
        }
        return { cols: 80, rows: 24 };
    }

    setOption(key, value) {
        if (this.terminal) {
            this.terminal.setOption(key, value);
        }
    }

    getOption(key) {
        if (this.terminal) {
            return this.terminal.getOption(key);
        }
        return null;
    }

    scrollToBottom() {
        if (this.terminal) {
            this.terminal.scrollToBottom();
        }
    }

    scrollToTop() {
        if (this.terminal) {
            this.terminal.scrollToTop();
        }
    }

    selectAll() {
        if (this.terminal) {
            this.terminal.selectAll();
        }
    }

    hasSelection() {
        if (this.terminal) {
            return this.terminal.hasSelection();
        }
        return false;
    }

    getSelection() {
        if (this.terminal) {
            return this.terminal.getSelection();
        }
        return '';
    }

    clearSelection() {
        if (this.terminal) {
            this.terminal.clearSelection();
        }
    }

    dispose() {
        if (this.terminal) {
            this.terminal.dispose();
            this.terminal = null;
            this.fitAddon = null;
            this.isInitialized = false;
        }
    }
} 