#!/bin/bash

# SSH AI Terminal 性能测试脚本

set -e

echo "🚀 开始性能测试..."

# 检查依赖
command -v curl >/dev/null 2>&1 || { echo "❌ curl 未安装"; exit 1; }
command -v ab >/dev/null 2>&1 || { echo "❌ Apache Bench (ab) 未安装"; exit 1; }

# 配置
SERVER_URL="http://localhost:8005"
TEST_DURATION=30
CONCURRENT_USERS=100
TOTAL_REQUESTS=10000

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 函数：打印带颜色的消息
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查服务器是否运行
check_server() {
    print_status "检查服务器状态..."
    if curl -s -f "$SERVER_URL/health" >/dev/null; then
        print_success "服务器正在运行"
        return 0
    else
        print_error "服务器未运行或无法访问"
        return 1
    fi
}

# 获取系统信息
get_system_info() {
    print_status "获取系统信息..."
    echo "CPU: $(nproc) 核心"
    echo "内存: $(free -h | awk '/^Mem:/{print $2}')"
    echo "系统: $(uname -a)"
    echo ""
}

# 健康检查测试
test_health_endpoint() {
    print_status "测试健康检查端点..."
    
    local start_time=$(date +%s.%N)
    local response=$(curl -s "$SERVER_URL/health")
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc)
    
    echo "响应时间: ${duration}s"
    echo "响应内容: $response"
    echo ""
}

# HTTP 性能测试
test_http_performance() {
    print_status "执行 HTTP 性能测试..."
    
    # 测试静态文件
    print_status "测试静态文件性能..."
    ab -n $TOTAL_REQUESTS -c $CONCURRENT_USERS -t $TEST_DURATION "$SERVER_URL/" > static_benchmark.txt 2>&1
    
    # 测试健康检查端点
    print_status "测试健康检查端点性能..."
    ab -n $TOTAL_REQUESTS -c $CONCURRENT_USERS -t $TEST_DURATION "$SERVER_URL/health" > health_benchmark.txt 2>&1
    
    # 测试指标端点
    print_status "测试指标端点性能..."
    ab -n $TOTAL_REQUESTS -c $CONCURRENT_USERS -t $TEST_DURATION "$SERVER_URL/metrics" > metrics_benchmark.txt 2>&1
    
    echo ""
}

# WebSocket 连接测试
test_websocket() {
    print_status "测试 WebSocket 连接..."
    
    # 使用 wscat 或类似工具测试 WebSocket
    if command -v wscat >/dev/null 2>&1; then
        echo "WebSocket 测试需要 wscat 工具"
        echo "安装: npm install -g wscat"
    else
        print_warning "wscat 未安装，跳过 WebSocket 测试"
    fi
    echo ""
}

# 内存和 CPU 使用测试
test_resource_usage() {
    print_status "测试资源使用情况..."
    
    # 获取进程 ID
    local pid=$(pgrep ssh-ai-terminal || echo "")
    if [ -n "$pid" ]; then
        echo "进程 ID: $pid"
        echo "内存使用:"
        ps -p $pid -o pid,ppid,cmd,%mem,%cpu --no-headers
        echo ""
    else
        print_warning "无法找到 ssh-ai-terminal 进程"
    fi
}

# 并发连接测试
test_concurrent_connections() {
    print_status "测试并发连接..."
    
    local max_connections=1000
    local current_connections=0
    
    echo "测试最大并发连接数..."
    
    # 使用 curl 创建多个并发连接
    for i in $(seq 1 $max_connections); do
        if curl -s -f "$SERVER_URL/health" >/dev/null 2>&1; then
            current_connections=$i
        else
            break
        fi
        
        if [ $((i % 100)) -eq 0 ]; then
            echo "已测试 $i 个连接..."
        fi
    done
    
    echo "最大并发连接数: $current_connections"
    echo ""
}

# 缓存性能测试
test_cache_performance() {
    print_status "测试缓存性能..."
    
    # 测试 AI 端点（需要认证）
    local test_data='{"session_id":"test","message":"hello world"}'
    
    echo "测试 AI 端点响应时间..."
    for i in {1..10}; do
        local start_time=$(date +%s.%N)
        curl -s -X POST "$SERVER_URL/api/ai/chat" \
             -H "Content-Type: application/json" \
             -d "$test_data" >/dev/null 2>&1
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc)
        echo "请求 $i: ${duration}s"
    done
    echo ""
}

# 生成测试报告
generate_report() {
    print_status "生成测试报告..."
    
    local report_file="benchmark_report_$(date +%Y%m%d_%H%M%S).txt"
    
    {
        echo "SSH AI Terminal 性能测试报告"
        echo "================================"
        echo "测试时间: $(date)"
        echo "服务器地址: $SERVER_URL"
        echo "测试配置:"
        echo "  - 并发用户数: $CONCURRENT_USERS"
        echo "  - 总请求数: $TOTAL_REQUESTS"
        echo "  - 测试时长: ${TEST_DURATION}s"
        echo ""
        
        echo "系统信息:"
        echo "  - CPU: $(nproc) 核心"
        echo "  - 内存: $(free -h | awk '/^Mem:/{print $2}')"
        echo "  - 系统: $(uname -a)"
        echo ""
        
        echo "静态文件性能测试结果:"
        if [ -f "static_benchmark.txt" ]; then
            grep -E "(Requests per second|Time per request|Transfer rate)" static_benchmark.txt
        fi
        echo ""
        
        echo "健康检查端点性能测试结果:"
        if [ -f "health_benchmark.txt" ]; then
            grep -E "(Requests per second|Time per request|Transfer rate)" health_benchmark.txt
        fi
        echo ""
        
        echo "指标端点性能测试结果:"
        if [ -f "metrics_benchmark.txt" ]; then
            grep -E "(Requests per second|Time per request|Transfer rate)" metrics_benchmark.txt
        fi
        echo ""
        
        echo "资源使用情况:"
        local pid=$(pgrep ssh-ai-terminal || echo "")
        if [ -n "$pid" ]; then
            ps -p $pid -o pid,ppid,cmd,%mem,%cpu --no-headers
        fi
        echo ""
        
        echo "测试完成时间: $(date)"
        
    } > "$report_file"
    
    print_success "测试报告已生成: $report_file"
    echo ""
    
    # 显示关键指标
    echo "关键性能指标:"
    if [ -f "static_benchmark.txt" ]; then
        echo "静态文件 RPS: $(grep 'Requests per second' static_benchmark.txt | awk '{print $4}')"
    fi
    if [ -f "health_benchmark.txt" ]; then
        echo "健康检查 RPS: $(grep 'Requests per second' health_benchmark.txt | awk '{print $4}')"
    fi
    echo ""
}

# 清理测试文件
cleanup() {
    print_status "清理测试文件..."
    rm -f static_benchmark.txt health_benchmark.txt metrics_benchmark.txt
}

# 主函数
main() {
    echo "SSH AI Terminal 性能测试"
    echo "========================"
    echo ""
    
    # 检查服务器
    if ! check_server; then
        print_error "请先启动服务器"
        exit 1
    fi
    
    # 获取系统信息
    get_system_info
    
    # 执行各种测试
    test_health_endpoint
    test_http_performance
    test_websocket
    test_resource_usage
    test_concurrent_connections
    test_cache_performance
    
    # 生成报告
    generate_report
    
    # 清理
    cleanup
    
    print_success "性能测试完成！"
}

# 处理信号
trap cleanup EXIT

# 运行主函数
main "$@"