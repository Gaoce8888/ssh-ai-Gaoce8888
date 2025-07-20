# 企业级监控脚本
param(
    [Parameter(Mandatory=$true)]
    [string]$Environment,
    
    [Parameter(Mandatory=$false)]
    [int]$Interval = 30,
    
    [Parameter(Mandatory=$false)]
    [string]$ConfigPath = "enterprise/build-config.json"
)

# 读取配置
Write-Host "读取监控配置..." -ForegroundColor Cyan
$config = Get-Content $ConfigPath | ConvertFrom-Json

# 初始化监控
Write-Host "初始化监控..." -ForegroundColor Cyan

# 创建监控目录
New-Item -ItemType Directory -Path "monitor/$Environment" -Force

# 主监控循环
while ($true) {
    Write-Host "执行监控检查..." -ForegroundColor Cyan
    
    # 健康检查
    Write-Host "执行健康检查..." -ForegroundColor White
    $healthCheck = Invoke-WebRequest -Uri "http://localhost:8080/api/health" -Method Get
    if ($healthCheck.StatusCode -ne 200) {
        Write-Error "健康检查失败: $($healthCheck.StatusCode)"
        Send-Alert -Type "health" -Message "健康检查失败" -Environment $Environment
    }
    
    # 性能监控
    if ($config.build.monitoring.metrics.performance) {
        Write-Host "执行性能监控..." -ForegroundColor White
        $performance = Invoke-WebRequest -Uri "http://localhost:8080/api/performance" -Method Get
        $performance | ConvertTo-Json | Set-Content -Path "monitor/$Environment/performance-$(Get-Date -Format "yyyyMMddHHmmss").json"
    }
    
    # 错误监控
    if ($config.build.monitoring.metrics.errors) {
        Write-Host "执行错误监控..." -ForegroundColor White
        $errors = Invoke-WebRequest -Uri "http://localhost:8080/api/errors" -Method Get
        $errors | ConvertTo-Json | Set-Content -Path "monitor/$Environment/errors-$(Get-Date -Format "yyyyMMddHHmmss").json"
    }
    
    # 用户行为监控
    if ($config.build.monitoring.metrics.user_actions) {
        Write-Host "执行用户行为监控..." -ForegroundColor White
        $actions = Invoke-WebRequest -Uri "http://localhost:8080/api/user-actions" -Method Get
        $actions | ConvertTo-Json | Set-Content -Path "monitor/$Environment/actions-$(Get-Date -Format "yyyyMMddHHss").json"
    }
    
    # 等待下一次检查
    Write-Host "等待下一次检查..." -ForegroundColor White
    Start-Sleep -Seconds $Interval
}

function Send-Alert {
    param(
        [Parameter(Mandatory=$true)]
        [string]$Type,
        
        [Parameter(Mandatory=$true)]
        [string]$Message,
        
        [Parameter(Mandatory=$true)]
        [string]$Environment
    )
    
    # 发送告警
    Write-Host "发送告警: $Type - $Message" -ForegroundColor Red
    
    # 发送到监控系统
    foreach ($provider in $config.build.monitoring.providers) {
        switch ($provider) {
            "newrelic" {
                Invoke-WebRequest -Uri "https://insights.newrelic.com/v1/accounts/123/events" -Method Post -Body @{"eventType"="alert";"message"=$Message;"environment"=$Environment}
            }
            "datadog" {
                Invoke-WebRequest -Uri "https://api.datadoghq.com/api/v1/events" -Method Post -Body @{"title"="Alert";"text"=$Message;"alert_type"="error";"environment"=$Environment}
            }
        }
    }
}
