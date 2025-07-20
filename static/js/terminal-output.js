// 终端输出处理模块
class TerminalOutput {
    constructor() {
        this.outputElement = document.querySelector('.terminal-output');
        this.maxLines = 1000;
        this.lineHistory = [];
        this.scrollLock = false;
        this.scrollThreshold = 50;
        
        this.init();
    }

    init() {
        // 添加滚动事件监听
        this.outputElement.addEventListener('scroll', () => {
            this.handleScroll();
        });
    }

    handleScroll() {
        const { scrollTop, scrollHeight, clientHeight } = this.outputElement;
        const isAtBottom = scrollHeight - scrollTop <= clientHeight + this.scrollThreshold;
        
        this.scrollLock = !isAtBottom;
    }

    append(text, type = 'default') {
        // 创建新行
        const line = document.createElement('div');
        line.className = `output-line ${type}`;
        line.textContent = text;
        
        // 添加到历史
        this.lineHistory.push({
            text: text,
            type: type,
            timestamp: Date.now()
        });
        
        // 限制行数
        if (this.lineHistory.length > this.maxLines) {
            this.lineHistory.shift();
            this.outputElement.removeChild(this.outputElement.firstChild);
        }
        
        // 添加到DOM
        this.outputElement.appendChild(line);
        
        // 自动滚动
        if (!this.scrollLock) {
            this.scrollToBottom();
        }
    }

    scrollToBottom() {
        this.outputElement.scrollTop = this.outputElement.scrollHeight;
    }

    clear() {
        this.outputElement.innerHTML = '';
        this.lineHistory = [];
    }

    search(pattern) {
        const regex = new RegExp(pattern, 'i');
        const results = this.lineHistory.filter(line => 
            regex.test(line.text)
        );
        
        return results;
    }

    highlight(pattern) {
        const regex = new RegExp(pattern, 'gi');
        const lines = this.outputElement.querySelectorAll('.output-line');
        
        lines.forEach(line => {
            const text = line.textContent;
            const highlighted = text.replace(regex, match => 
                `<span class="highlight">${match}</span>`
            );
            
            line.innerHTML = highlighted;
        });
    }

    formatOutput(text) {
        // 格式化特殊字符
        return text
            .replace(/\n/g, '<br>')
            .replace(/\t/g, '    ')
            .replace(/\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[m|K]/g, '');
    }

    getHistory() {
        return this.lineHistory;
    }

    exportHistory() {
        const history = this.lineHistory.map(line => 
            `[${new Date(line.timestamp).toLocaleTimeString()}] ${line.text}`
        );
        return history.join('\n');
    }
}

// 导出单例
const terminalOutput = new TerminalOutput();
export default terminalOutput;
