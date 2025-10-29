# Alternative download script using different approach

param(
    [string]$ModelsDir = "models"
)

# Функция для скачивания с повторными попытками
function Download-FileWithRetry {
    param([string]$Url, [string]$OutputPath, [int]$RetryCount = 3)
    
    for ($i = 1; $i -le $RetryCount; $i++) {
        try {
            Write-Host "Attempt $i to download $(Split-Path $Url -Leaf)..." -ForegroundColor Yellow
            $progressPreference = 'silentlyContinue'
            Invoke-WebRequest -Uri $Url -OutFile $OutputPath -TimeoutSec 30
            $progressPreference = 'Continue'
            Write-Host "✅ Downloaded successfully" -ForegroundColor Green
            return $true
        }
        catch {
            Write-Host "❌ Attempt $i failed: $($_.Exception.Message)" -ForegroundColor Red
            if ($i -eq $RetryCount) {
                return $false
            }
            Start-Sleep -Seconds 5
        }
    }
}

# Создаем директорию
New-Item -ItemType Directory -Path $ModelsDir -Force | Out-Null

# Модели для скачивания
$models = @(
    @{
        Name = "all-MiniLM-L6-v2"
        Files = @(
            "model.onnx",
            "tokenizer.json", 
            "tokenizer_config.json",
            "vocab.txt",
            "special_tokens_map.json",
            "config.json"
        )
        BaseUrl = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main"
    }
)

foreach ($model in $models) {
    Write-Host "`n📦 Processing $($model.Name)..." -ForegroundColor Cyan
    
    foreach ($file in $model.Files) {
        $url = "$($model.BaseUrl)/$file"
        $outputPath = "$ModelsDir/$file"
        
        if (Test-Path $outputPath) {
            Write-Host "  ✅ $file already exists" -ForegroundColor Green
            continue
        }
        
        $success = Download-FileWithRetry -Url $url -OutputPath $outputPath
        if (!$success) {
            Write-Host "  🚨 Failed to download $file after multiple attempts" -ForegroundColor Red
        }
    }
}

Write-Host "`n🎉 Download process completed!" -ForegroundColor Green