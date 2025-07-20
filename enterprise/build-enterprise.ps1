# 企业级构建脚本
param(
    [Parameter(Mandatory=$false)]
    [string]$ConfigPath = "enterprise/build-config.json",
    
    [Parameter(Mandatory=$false)]
    [string]$OutputPath = "dist"
)

# 读取配置
Write-Host "读取构建配置..." -ForegroundColor Cyan
$config = Get-Content $ConfigPath | ConvertFrom-Json

# 创建输出目录
Write-Host "创建输出目录..." -ForegroundColor Cyan
New-Item -ItemType Directory -Path $OutputPath -Force
foreach ($dir in $config.build.output.assets.PSObject.Properties.Value) {
    New-Item -ItemType Directory -Path "$OutputPath/$dir" -Force
}

# Rust编译优化
Write-Host "优化Rust编译..." -ForegroundColor Cyan
$rustFlags = "-C target-cpu=$($config.build.optimizations.rust.target_cpu) " +
            "-C opt-level=$($config.build.optimizations.rust.opt_level) " +
            "-C debug-assertions=$($config.build.optimizations.rust.debug_assertions) " +
            "-C overflow-checks=$($config.build.optimizations.rust.overflow_checks)"

Write-Host "编译Rust项目..." -ForegroundColor Cyan
cargo build --release --features="enterprise" --target-cpu=$rustFlags

# JavaScript优化
Write-Host "优化JavaScript..." -ForegroundColor Cyan
$jsFiles = Get-ChildItem -Path "static/js" -Filter "*.js"
foreach ($file in $jsFiles) {
    $outputPath = Join-Path $OutputPath "js/$($file.Name)"
    Write-Host "处理: $($file.Name)" -ForegroundColor White
    
    # 压缩
    if ($config.build.optimizations.javascript.minify) {
        npx terser "$file.FullName" -c -m -o $outputPath
    } else {
        Copy-Item $file.FullName $outputPath
    }
}

# CSS优化
Write-Host "优化CSS..." -ForegroundColor Cyan
$cssFiles = Get-ChildItem -Path "static/css" -Filter "*.css"
foreach ($file in $cssFiles) {
    $outputPath = Join-Path $OutputPath "css/$($file.Name)"
    Write-Host "处理: $($file.Name)" -ForegroundColor White
    
    # 压缩
    if ($config.build.optimizations.css.minify) {
        npx cssnano "$file.FullName" $outputPath
    } else {
        Copy-Item $file.FullName $outputPath
    }
}

# 版本控制
Write-Host "添加版本控制..." -ForegroundColor Cyan
if ($config.build.versioning.enabled) {
    $version = "v" + (Get-Date).ToString("yyyyMMddHHmmss")
    
    # 更新HTML文件中的资源引用
    $htmlFiles = Get-ChildItem -Path "static" -Filter "*.html"
    foreach ($file in $htmlFiles) {
        $content = Get-Content $file.FullName -Raw
        $content = $content -replace "(\.js|\.css)", "`$1?$version"
        $outputPath = Join-Path $OutputPath $file.Name
        Set-Content -Path $outputPath -Value $content
    }
}

# 缓存控制
Write-Host "设置缓存控制..." -ForegroundColor Cyan
if ($config.build.caching.enabled) {
    $cacheControl = "max-age=$($config.build.caching.max_age)"
    
    # 更新HTML头部
    $htmlFiles = Get-ChildItem -Path $OutputPath -Filter "*.html"
    foreach ($file in $htmlFiles) {
        $content = Get-Content $file.FullName -Raw
        $content = $content -replace "<head>", "<head>`n<meta http-equiv="Cache-Control" content="$cacheControl">"
        Set-Content -Path $file.FullName -Value $content
    }
}

# 安全策略
Write-Host "应用安全策略..." -ForegroundColor Cyan
if ($config.build.security.csp.enabled) {
    $csp = "default-src 'self';"
    foreach ($policy in $config.build.security.csp.policies.PSObject.Properties) {
        $csp += " $policy.Name 'self'"
        foreach ($src in $policy.Value) {
            $csp += " $src"
        }
    }
    
    # 更新HTML头部
    $htmlFiles = Get-ChildItem -Path $OutputPath -Filter "*.html"
    foreach ($file in $htmlFiles) {
        $content = Get-Content $file.FullName -Raw
        $content = $content -replace "<head>", "<head>`n<meta http-equiv="Content-Security-Policy" content="$csp">"
        Set-Content -Path $file.FullName -Value $content
    }
}

# 文档生成
Write-Host "生成文档..." -ForegroundColor Cyan
if ($config.build.documentation.enabled) {
    New-Item -ItemType Directory -Path "$OutputPath/docs" -Force
    
    # 生成API文档
    cargo doc --open
    
    # 生成配置文档
    $config | ConvertTo-Json -Depth 10 | Set-Content -Path "$OutputPath/docs/config.json"
}

# 完成
Write-Host "构建完成!" -ForegroundColor Green
Write-Host "输出目录: $OutputPath" -ForegroundColor Green
Write-Host "构建时间: $(Get-Date)" -ForegroundColor Green
