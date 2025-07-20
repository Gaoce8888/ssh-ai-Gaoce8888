// WebSocket处理模块
class WebSocketManager {
    constructor() {
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 1000;
        this.isConnected = false;
        this.messageQueue = [];
        this.listeners = {
            open: [],
            message: [],
            error: [],
            close: []
        };
        
        this.init();
    }

    init() {
        this.connect();
    }

    connect() {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            return;
        }

        const token = localStorage.getItem('token');
        if (!token) {
            this.trigger('error', new Error('No authentication token'));
            return;
        }

        try {
            this.ws = new WebSocket(`ws://localhost:8080/ws?token=${token}`);
            
            this.ws.onopen = () => {
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.trigger('open');
                this.processQueue();
            };

            this.ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    this.trigger('message', data);
                } catch (error) {
                    console.error('Failed to parse WebSocket message:', error);
                    this.trigger('error', error);
                }
            };

            this.ws.onerror = (error) => {
                console.error('WebSocket error:', error);
                this.trigger('error', error);
                this.reconnect();
            };

            this.ws.onclose = () => {
                this.isConnected = false;
                this.trigger('close');
                this.reconnect();
            };
        } catch (error) {
            console.error('WebSocket connection failed:', error);
            this.trigger('error', error);
            this.reconnect();
        }
    }

    reconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            this.trigger('error', new Error('Max reconnect attempts reached'));
            return;
        }

        this.reconnectAttempts++;
        setTimeout(() => {
            this.connect();
        }, this.reconnectDelay * this.reconnectAttempts);
    }

    send(message) {
        if (!this.isConnected) {
            this.messageQueue.push(message);
            return;
        }

        try {
            this.ws.send(JSON.stringify(message));
        } catch (error) {
            console.error('Failed to send message:', error);
            this.messageQueue.push(message);
            this.reconnect();
        }
    }

    processQueue() {
        while (this.messageQueue.length > 0 && this.isConnected) {
            const message = this.messageQueue.shift();
            this.send(message);
        }
    }

    addListener(eventType, callback) {
        if (!this.listeners[eventType]) {
            throw new Error(`Invalid event type: ${eventType}`);
        }
        this.listeners[eventType].push(callback);
    }

    removeListener(eventType, callback) {
        if (!this.listeners[eventType]) {
            return;
        }
        this.listeners[eventType] = this.listeners[eventType].filter(
            listener => listener !== callback
        );
    }

    trigger(eventType, data) {
        const callbacks = this.listeners[eventType] || [];
        callbacks.forEach(callback => callback(data));
    }

    close() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.isConnected = false;
        this.messageQueue = [];
    }
}

// 导出单例
const websocketManager = new WebSocketManager();
export default websocketManager;
