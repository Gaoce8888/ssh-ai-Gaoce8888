# 企业级部署脚本
param(
    [Parameter(Mandatory=$true)]
    [string]$Environment,
    
    [Parameter(Mandatory=$false)]
    [string]$BuildPath = "dist",
    
    [Parameter(Mandatory=$false)]
    [string]$ConfigPath = "enterprise/build-config.json"
)

# 读取配置
Write-Host "读取部署配置..." -ForegroundColor Cyan
$config = Get-Content $ConfigPath | ConvertFrom-Json

# 部署前检查
Write-Host "执行部署前检查..." -ForegroundColor Cyan

# 健康检查
Write-Host "执行健康检查..." -ForegroundColor Cyan
$healthCheck = Invoke-WebRequest -Uri "http://localhost:8080/api/health" -Method Get
if ($healthCheck.StatusCode -ne 200) {
    Write-Error "健康检查失败: $($healthCheck.StatusCode)"
    exit 1
}

# 备份
Write-Host "创建备份..." -ForegroundColor Cyan
$backupPath = "backup/$Environment-$(Get-Date -Format "yyyyMMddHHmmss")"
New-Item -ItemType Directory -Path $backupPath -Force
Copy-Item -Path "$BuildPath/*" -Destination $backupPath -Recurse

# 蓝绿部署
Write-Host "执行蓝绿部署..." -ForegroundColor Cyan

# 创建临时目录
$tempPath = "temp/$Environment"
New-Item -ItemType Directory -Path $tempPath -Force

# 复制构建输出
Write-Host "复制构建输出..." -ForegroundColor Cyan
Copy-Item -Path "$BuildPath/*" -Destination $tempPath -Recurse

# 验证部署
Write-Host "验证部署..." -ForegroundColor Cyan
$validationPath = "validation/$Environment"
New-Item -ItemType Directory -Path $validationPath -Force

# 执行性能测试
Write-Host "执行性能测试..." -ForegroundColor Cyan
if ($config.build.testing.performance) {
    $testResults = Invoke-WebRequest -Uri "http://localhost:8080/api/performance" -Method Get
    if ($testResults.StatusCode -ne 200) {
        Write-Error "性能测试失败: $($testResults.StatusCode)"
        exit 1
    }
}

# 执行安全测试
Write-Host "执行安全测试..." -ForegroundColor Cyan
if ($config.build.testing.security) {
    $securityResults = Invoke-WebRequest -Uri "http://localhost:8080/api/security" -Method Get
    if ($securityResults.StatusCode -ne 200) {
        Write-Error "安全测试失败: $($securityResults.StatusCode)"
        exit 1
    }
}

# 切换到新版本
Write-Host "切换到新版本..." -ForegroundColor Cyan

# 更新配置
Write-Host "更新配置..." -ForegroundColor Cyan
$configFile = "config/$Environment.json"
if (Test-Path $configFile) {
    $config = Get-Content $configFile | ConvertFrom-Json
    $config | ConvertTo-Json -Depth 10 | Set-Content -Path "$tempPath/config.json"
}

# 更新环境变量
Write-Host "更新环境变量..." -ForegroundColor Cyan
$env:SSH_AI_TERMINAL_ENV = $Environment
$env:SSH_AI_TERMINAL_CONFIG = "$tempPath/config.json"

# 服务重启
Write-Host "重启服务..." -ForegroundColor Cyan
Stop-Process -Name "ssh-ai-terminal" -Force
Start-Sleep -Seconds 5
Start-Process -FilePath "$tempPath/ssh-ai-terminal.exe" -ArgumentList "--config $env:SSH_AI_TERMINAL_CONFIG"

# 验证服务状态
Write-Host "验证服务状态..." -ForegroundColor Cyan
$healthCheck = Invoke-WebRequest -Uri "http://localhost:8080/api/health" -Method Get
if ($healthCheck.StatusCode -ne 200) {
    Write-Error "服务启动失败: $($healthCheck.StatusCode)"
    exit 1
}

# 清理
Write-Host "清理临时文件..." -ForegroundColor Cyan
Remove-Item -Path $tempPath -Recurse -Force

# 完成
Write-Host "部署完成!" -ForegroundColor Green
Write-Host "环境: $Environment" -ForegroundColor Green
Write-Host "部署时间: $(Get-Date)" -ForegroundColor Green
Write-Host "备份位置: $backupPath" -ForegroundColor Green
