#!/bin/bash

# 静态资源优化脚本
# 功能：压缩CSS、JS、HTML，优化图片，生成资源清单

set -e

echo "🚀 开始优化静态资源..."

# 创建优化后的目录
OPTIMIZED_DIR="static_optimized"
mkdir -p "$OPTIMIZED_DIR"

# 复制目录结构
cp -r static/* "$OPTIMIZED_DIR/"

# 安装必要的工具（如果没有）
install_tools() {
    echo "📦 检查并安装优化工具..."
    
    # 检查是否有 npm
    if ! command -v npm &> /dev/null; then
        echo "❌ 需要 npm，请先安装 Node.js"
        exit 1
    fi
    
    # 安装全局工具
    npm install -g terser csso-cli html-minifier-terser &>/dev/null || true
}

# CSS优化
optimize_css() {
    echo "🎨 优化CSS文件..."
    
    find "$OPTIMIZED_DIR" -name "*.css" -type f | while read -r file; do
        echo "  处理: $file"
        
        # 创建备份
        cp "$file" "$file.bak"
        
        # 使用csso压缩CSS
        if command -v csso &> /dev/null; then
            csso "$file" --output "$file.min"
            mv "$file.min" "$file"
        else
            # 简单的CSS压缩（去除注释和空白）
            sed -e 's|/\*[^*]*\*\+\([^/*][^*]*\*\+\)*/||g' \
                -e 's/[[:space:]]\+/ /g' \
                -e 's/; /;/g' \
                -e 's/: /:/g' \
                -e 's/{ /{/g' \
                -e 's/ }/}/g' \
                "$file.bak" > "$file"
        fi
        
        # 计算压缩率
        ORIGINAL_SIZE=$(wc -c < "$file.bak")
        COMPRESSED_SIZE=$(wc -c < "$file")
        RATIO=$(echo "scale=2; ($ORIGINAL_SIZE - $COMPRESSED_SIZE) * 100 / $ORIGINAL_SIZE" | bc -l 2>/dev/null || echo "0")
        echo "    压缩率: ${RATIO}%"
        
        rm "$file.bak"
    done
}

# JavaScript优化
optimize_js() {
    echo "⚡ 优化JavaScript文件..."
    
    find "$OPTIMIZED_DIR" -name "*.js" -type f | while read -r file; do
        echo "  处理: $file"
        
        # 创建备份
        cp "$file" "$file.bak"
        
        # 使用terser压缩JS
        if command -v terser &> /dev/null; then
            terser "$file" --compress --mangle --output "$file"
        else
            # 简单的JS压缩（去除注释和多余空白）
            sed -e 's|//.*$||g' \
                -e 's|/\*[^*]*\*\+\([^/*][^*]*\*\+\)*/||g' \
                -e 's/[[:space:]]\+/ /g' \
                -e 's/; /;/g' \
                "$file.bak" > "$file"
        fi
        
        # 计算压缩率
        ORIGINAL_SIZE=$(wc -c < "$file.bak")
        COMPRESSED_SIZE=$(wc -c < "$file")
        RATIO=$(echo "scale=2; ($ORIGINAL_SIZE - $COMPRESSED_SIZE) * 100 / $ORIGINAL_SIZE" | bc -l 2>/dev/null || echo "0")
        echo "    压缩率: ${RATIO}%"
        
        rm "$file.bak"
    done
}

# HTML优化
optimize_html() {
    echo "📄 优化HTML文件..."
    
    find "$OPTIMIZED_DIR" -name "*.html" -type f | while read -r file; do
        echo "  处理: $file"
        
        # 创建备份
        cp "$file" "$file.bak"
        
        # 使用html-minifier压缩HTML
        if command -v html-minifier-terser &> /dev/null; then
            html-minifier-terser \
                --collapse-whitespace \
                --remove-comments \
                --remove-optional-tags \
                --remove-redundant-attributes \
                --remove-script-type-attributes \
                --remove-tag-whitespace \
                --use-short-doctype \
                --minify-css true \
                --minify-js true \
                "$file" > "$file.min"
            mv "$file.min" "$file"
        else
            # 简单的HTML压缩
            sed -e 's|<!--[^>]*-->||g' \
                -e 's/[[:space:]]\+/ /g' \
                -e 's/> </></g' \
                "$file.bak" > "$file"
        fi
        
        # 计算压缩率
        ORIGINAL_SIZE=$(wc -c < "$file.bak")
        COMPRESSED_SIZE=$(wc -c < "$file")
        RATIO=$(echo "scale=2; ($ORIGINAL_SIZE - $COMPRESSED_SIZE) * 100 / $ORIGINAL_SIZE" | bc -l 2>/dev/null || echo "0")
        echo "    压缩率: ${RATIO}%"
        
        rm "$file.bak"
    done
}

# 图片优化
optimize_images() {
    echo "🖼️  优化图片文件..."
    
    # PNG优化
    if command -v optipng &> /dev/null; then
        find "$OPTIMIZED_DIR" -name "*.png" -type f | while read -r file; do
            echo "  优化PNG: $file"
            optipng -quiet -o2 "$file"
        done
    fi
    
    # JPG优化
    if command -v jpegoptim &> /dev/null; then
        find "$OPTIMIZED_DIR" -name "*.jpg" -o -name "*.jpeg" -type f | while read -r file; do
            echo "  优化JPG: $file"
            jpegoptim --quiet --strip-all "$file"
        done
    fi
}

# 生成Gzip压缩文件
generate_gzip() {
    echo "📦 生成Gzip压缩文件..."
    
    find "$OPTIMIZED_DIR" \( -name "*.css" -o -name "*.js" -o -name "*.html" -o -name "*.json" \) -type f | while read -r file; do
        echo "  压缩: $file"
        gzip -9 -c "$file" > "$file.gz"
    done
}

# 生成Brotli压缩文件
generate_brotli() {
    echo "🗜️  生成Brotli压缩文件..."
    
    if command -v brotli &> /dev/null; then
        find "$OPTIMIZED_DIR" \( -name "*.css" -o -name "*.js" -o -name "*.html" -o -name "*.json" \) -type f | while read -r file; do
            echo "  压缩: $file"
            brotli -9 -c "$file" > "$file.br"
        done
    else
        echo "  ⚠️  brotli 未安装，跳过Brotli压缩"
    fi
}

# 生成资源清单
generate_manifest() {
    echo "📋 生成资源清单..."
    
    MANIFEST_FILE="$OPTIMIZED_DIR/manifest.json"
    echo "{" > "$MANIFEST_FILE"
    echo "  \"generated\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"," >> "$MANIFEST_FILE"
    echo "  \"assets\": {" >> "$MANIFEST_FILE"
    
    FIRST=true
    find "$OPTIMIZED_DIR" -type f \( -name "*.css" -o -name "*.js" -o -name "*.html" -o -name "*.png" -o -name "*.jpg" -o -name "*.jpeg" -o -name "*.gif" -o -name "*.svg" \) | while read -r file; do
        REL_PATH=$(echo "$file" | sed "s|$OPTIMIZED_DIR/||")
        SIZE=$(wc -c < "$file")
        HASH=$(md5sum "$file" | cut -d' ' -f1)
        
        if [ "$FIRST" = true ]; then
            FIRST=false
        else
            echo "," >> "$MANIFEST_FILE"
        fi
        
        echo -n "    \"$REL_PATH\": {" >> "$MANIFEST_FILE"
        echo -n "\"size\": $SIZE, \"hash\": \"$HASH\"}" >> "$MANIFEST_FILE"
    done
    
    echo "" >> "$MANIFEST_FILE"
    echo "  }" >> "$MANIFEST_FILE"
    echo "}" >> "$MANIFEST_FILE"
}

# 生成优化报告
generate_report() {
    echo "📊 生成优化报告..."
    
    REPORT_FILE="optimization_report.txt"
    {
        echo "静态资源优化报告"
        echo "================="
        echo "生成时间: $(date)"
        echo ""
        
        echo "文件统计："
        echo "CSS文件: $(find "$OPTIMIZED_DIR" -name "*.css" | wc -l)"
        echo "JS文件:  $(find "$OPTIMIZED_DIR" -name "*.js" | wc -l)"
        echo "HTML文件: $(find "$OPTIMIZED_DIR" -name "*.html" | wc -l)"
        echo "图片文件: $(find "$OPTIMIZED_DIR" \( -name "*.png" -o -name "*.jpg" -o -name "*.jpeg" -o -name "*.gif" -o -name "*.svg" \) | wc -l)"
        echo ""
        
        ORIGINAL_SIZE=$(du -sb static/ | cut -f1)
        OPTIMIZED_SIZE=$(du -sb "$OPTIMIZED_DIR/" | cut -f1)
        SAVINGS=$(echo "scale=2; ($ORIGINAL_SIZE - $OPTIMIZED_SIZE) * 100 / $ORIGINAL_SIZE" | bc -l 2>/dev/null || echo "0")
        
        echo "大小对比："
        echo "原始大小: $(numfmt --to=iec-i --suffix=B $ORIGINAL_SIZE)"
        echo "优化后大小: $(numfmt --to=iec-i --suffix=B $OPTIMIZED_SIZE)"
        echo "节省空间: ${SAVINGS}%"
        echo ""
        
        echo "压缩文件统计："
        echo "Gzip文件: $(find "$OPTIMIZED_DIR" -name "*.gz" | wc -l)"
        echo "Brotli文件: $(find "$OPTIMIZED_DIR" -name "*.br" | wc -l)"
    } > "$REPORT_FILE"
    
    echo "📄 优化报告已保存到: $REPORT_FILE"
}

# 主执行流程
main() {
    # 检查bc命令（用于计算）
    if ! command -v bc &> /dev/null; then
        echo "⚠️  bc 命令未找到，某些计算功能可能不可用"
    fi
    
    install_tools
    optimize_css
    optimize_js
    optimize_html
    optimize_images
    generate_gzip
    generate_brotli
    generate_manifest
    generate_report
    
    echo ""
    echo "✅ 静态资源优化完成！"
    echo "优化后的文件保存在: $OPTIMIZED_DIR/"
    echo "建议在生产环境中使用优化后的资源。"
    echo ""
    echo "📌 使用说明："
    echo "1. 将 Rust 配置中的静态文件路径改为 '$OPTIMIZED_DIR'"
    echo "2. 配置Web服务器支持 .gz 和 .br 文件的自动服务"
    echo "3. 设置适当的缓存头以提高性能"
}

# 运行主函数
main "$@"