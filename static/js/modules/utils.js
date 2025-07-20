// 工具模块
export class Utils {
    // 初始化
    static init() {
        // 添加全局错误处理
        window.addEventListener('error', (event) => {
            console.error('Global error:', event.error);
        });
        
        window.addEventListener('unhandledrejection', (event) => {
            console.error('Unhandled promise rejection:', event.reason);
        });
    }

    // 防抖函数
    static debounce(func, wait, immediate = false) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                timeout = null;
                if (!immediate) func(...args);
            };
            
            const callNow = immediate && !timeout;
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
            
            if (callNow) func(...args);
        };
    }

    // 节流函数
    static throttle(func, limit) {
        let inThrottle;
        return function(...args) {
            if (!inThrottle) {
                func.apply(this, args);
                inThrottle = true;
                setTimeout(() => inThrottle = false, limit);
            }
        };
    }

    // 获取表单数据
    static getFormData(formId) {
        const form = document.getElementById(formId);
        if (!form) return {};
        
        const formData = new FormData(form);
        const data = {};
        
        for (const [key, value] of formData.entries()) {
            data[key] = value;
        }
        
        return data;
    }

    // 设置表单数据
    static setFormData(formId, data) {
        const form = document.getElementById(formId);
        if (!form) return;
        
        Object.keys(data).forEach(key => {
            const element = form.elements[key];
            if (element) {
                if (element.type === 'checkbox') {
                    element.checked = data[key];
                } else if (element.type === 'radio') {
                    const radio = form.querySelector(`input[name="${key}"][value="${data[key]}"]`);
                    if (radio) radio.checked = true;
                } else {
                    element.value = data[key];
                }
            }
        });
    }

    // HTML转义
    static escapeHtml(text) {
        const map = {
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#039;'
        };
        return text.replace(/[&<>"']/g, m => map[m]);
    }

    // HTML反转义
    static unescapeHtml(text) {
        const map = {
            '&amp;': '&',
            '&lt;': '<',
            '&gt;': '>',
            '&quot;': '"',
            '&#039;': "'"
        };
        return text.replace(/&amp;|&lt;|&gt;|&quot;|&#039;/g, m => map[m]);
    }

    // 复制到剪贴板
    static async copyToClipboard(text) {
        try {
            if (navigator.clipboard && window.isSecureContext) {
                await navigator.clipboard.writeText(text);
                return true;
            } else {
                // 降级方案
                const textArea = document.createElement('textarea');
                textArea.value = text;
                textArea.style.position = 'fixed';
                textArea.style.left = '-999999px';
                textArea.style.top = '-999999px';
                document.body.appendChild(textArea);
                textArea.focus();
                textArea.select();
                
                try {
                    document.execCommand('copy');
                    return true;
                } catch (err) {
                    console.error('复制失败:', err);
                    return false;
                } finally {
                    textArea.remove();
                }
            }
        } catch (err) {
            console.error('复制到剪贴板失败:', err);
            return false;
        }
    }

    // 格式化文件大小
    static formatBytes(bytes, decimals = 2) {
        if (bytes === 0) return '0 Bytes';
        
        const k = 1024;
        const dm = decimals < 0 ? 0 : decimals;
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
        
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        
        return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
    }

    // 格式化时间
    static formatDate(date, format = 'YYYY-MM-DD HH:mm:ss') {
        const d = new Date(date);
        
        const year = d.getFullYear();
        const month = String(d.getMonth() + 1).padStart(2, '0');
        const day = String(d.getDate()).padStart(2, '0');
        const hours = String(d.getHours()).padStart(2, '0');
        const minutes = String(d.getMinutes()).padStart(2, '0');
        const seconds = String(d.getSeconds()).padStart(2, '0');
        
        return format
            .replace('YYYY', year)
            .replace('MM', month)
            .replace('DD', day)
            .replace('HH', hours)
            .replace('mm', minutes)
            .replace('ss', seconds);
    }

    // 相对时间格式化
    static formatRelativeTime(date) {
        const now = new Date();
        const past = new Date(date);
        const diffMs = now - past;
        const diffSecs = Math.floor(diffMs / 1000);
        const diffMins = Math.floor(diffSecs / 60);
        const diffHours = Math.floor(diffMins / 60);
        const diffDays = Math.floor(diffHours / 24);
        
        if (diffSecs < 60) return '刚刚';
        if (diffMins < 60) return `${diffMins}分钟前`;
        if (diffHours < 24) return `${diffHours}小时前`;
        if (diffDays < 30) return `${diffDays}天前`;
        
        return this.formatDate(past, 'YYYY-MM-DD');
    }

    // 生成唯一ID
    static generateId(prefix = 'id') {
        return `${prefix}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    }

    // 深拷贝
    static deepClone(obj) {
        if (obj === null || typeof obj !== 'object') return obj;
        if (obj instanceof Date) return new Date(obj.getTime());
        if (obj instanceof Array) return obj.map(item => this.deepClone(item));
        if (obj instanceof Object) {
            const clonedObj = {};
            for (const key in obj) {
                if (obj.hasOwnProperty(key)) {
                    clonedObj[key] = this.deepClone(obj[key]);
                }
            }
            return clonedObj;
        }
    }

    // 检测是否为移动设备
    static isMobile() {
        return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
    }

    // 检测是否为触摸设备
    static isTouchDevice() {
        return 'ontouchstart' in window || navigator.maxTouchPoints > 0;
    }

    // 获取URL参数
    static getUrlParams() {
        const params = {};
        const searchParams = new URLSearchParams(window.location.search);
        for (const [key, value] of searchParams) {
            params[key] = value;
        }
        return params;
    }

    // 设置URL参数
    static setUrlParam(key, value) {
        const url = new URL(window.location);
        url.searchParams.set(key, value);
        window.history.pushState({}, '', url);
    }

    // 删除URL参数
    static removeUrlParam(key) {
        const url = new URL(window.location);
        url.searchParams.delete(key);
        window.history.pushState({}, '', url);
    }

    // 等待元素出现
    static waitForElement(selector, timeout = 5000) {
        return new Promise((resolve, reject) => {
            const element = document.querySelector(selector);
            if (element) {
                resolve(element);
                return;
            }
            
            const observer = new MutationObserver((mutations, obs) => {
                const element = document.querySelector(selector);
                if (element) {
                    obs.disconnect();
                    resolve(element);
                }
            });
            
            observer.observe(document.body, {
                childList: true,
                subtree: true
            });
            
            setTimeout(() => {
                observer.disconnect();
                reject(new Error(`Element ${selector} not found within ${timeout}ms`));
            }, timeout);
        });
    }

    // 平滑滚动到元素
    static scrollToElement(element, options = {}) {
        const defaultOptions = {
            behavior: 'smooth',
            block: 'center',
            inline: 'nearest'
        };
        
        element.scrollIntoView({ ...defaultOptions, ...options });
    }

    // 下载文件
    static downloadFile(content, filename, contentType = 'text/plain') {
        const blob = new Blob([content], { type: contentType });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);
    }

    // 读取文件
    static readFile(file, asText = true) {
        return new Promise((resolve, reject) => {
            const reader = new FileReader();
            
            reader.onload = (e) => resolve(e.target.result);
            reader.onerror = reject;
            
            if (asText) {
                reader.readAsText(file);
            } else {
                reader.readAsDataURL(file);
            }
        });
    }

    // 延迟执行
    static sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    // 重试函数
    static async retry(fn, retries = 3, delay = 1000) {
        try {
            return await fn();
        } catch (error) {
            if (retries === 0) throw error;
            await this.sleep(delay);
            return this.retry(fn, retries - 1, delay * 2);
        }
    }

    // 批量执行Promise
    static async batchPromises(promises, batchSize = 5) {
        const results = [];
        for (let i = 0; i < promises.length; i += batchSize) {
            const batch = promises.slice(i, i + batchSize);
            const batchResults = await Promise.all(batch);
            results.push(...batchResults);
        }
        return results;
    }

    // 事件发射器
    static createEventEmitter() {
        const events = {};
        
        return {
            on(event, listener) {
                if (!events[event]) events[event] = [];
                events[event].push(listener);
            },
            
            off(event, listener) {
                if (!events[event]) return;
                events[event] = events[event].filter(l => l !== listener);
            },
            
            emit(event, ...args) {
                if (!events[event]) return;
                events[event].forEach(listener => listener(...args));
            },
            
            once(event, listener) {
                const wrapper = (...args) => {
                    listener(...args);
                    this.off(event, wrapper);
                };
                this.on(event, wrapper);
            }
        };
    }
} 