# Download ONNX Models for RAG System
# Запустить от администратора: Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

param(
    [string]$ModelsDir = "models",
    [switch]$Force = $false
)

# Создаем директорию если не существует
if (!(Test-Path $ModelsDir)) {
    New-Item -ItemType Directory -Path $ModelsDir -Force
}

Write-Host "🔍 Downloading ONNX models for RAG system..." -ForegroundColor Green

# 1. Sentence Transformer для эмбеддингов
$embeddingModelUrl = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/model.onnx"
$embeddingModelPath = "$ModelsDir/all-MiniLM-L6-v2.onnx"

if ($Force -or !(Test-Path $embeddingModelPath)) {
    Write-Host "📥 Downloading embedding model..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri $embeddingModelUrl -OutFile $embeddingModelPath -UserAgent "Wget"
        Write-Host "✅ Embedding model downloaded: $embeddingModelPath" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Failed to download embedding model: $_" -ForegroundColor Red
    }
} else {
    Write-Host "✅ Embedding model already exists: $embeddingModelPath" -ForegroundColor Green
}

# 2. Cross-Encoder для реранжирования
$crossEncoderUrl = "https://huggingface.co/cross-encoder/ms-marco-MiniLM-L-6-v2/resolve/main/model.onnx"
$crossEncoderPath = "$ModelsDir/cross-encoder.onnx"

if ($Force -or !(Test-Path $crossEncoderPath)) {
    Write-Host "📥 Downloading cross-encoder model..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri $crossEncoderUrl -OutFile $crossEncoderPath -UserAgent "Wget"
        Write-Host "✅ Cross-encoder model downloaded: $crossEncoderPath" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Failed to download cross-encoder: $_" -ForegroundColor Red
    }
} else {
    Write-Host "✅ Cross-encoder model already exists: $crossEncoderPath" -ForegroundColor Green
}

# 3. Токенизаторы (конфиги)
$tokenizerFiles = @(
    @{Name = "tokenizer.json"; Url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json"},
    @{Name = "tokenizer_config.json"; Url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer_config.json"},
    @{Name = "vocab.txt"; Url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/vocab.txt"},
    @{Name = "special_tokens_map.json"; Url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/special_tokens_map.json"},
    @{Name = "config.json"; Url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json"}
)

foreach ($file in $tokenizerFiles) {
    $filePath = "$ModelsDir/$($file.Name)"
    if ($Force -or !(Test-Path $filePath)) {
        Write-Host "📥 Downloading $($file.Name)..." -ForegroundColor Yellow
        try {
            Invoke-WebRequest -Uri $file.Url -OutFile $filePath -UserAgent "Wget"
            Write-Host "✅ $($file.Name) downloaded" -ForegroundColor Green
        }
        catch {
            Write-Host "❌ Failed to download $($file.Name): $_" -ForegroundColor Red
        }
    } else {
        Write-Host "✅ $($file.Name) already exists" -ForegroundColor Green
    }
}

# 4. Проверяем размеры файлов
Write-Host "`n📊 Model files summary:" -ForegroundColor Cyan
Get-ChildItem $ModelsDir | ForEach-Object {
    $sizeMB = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  $($_.Name): $sizeMB MB" -ForegroundColor White
}

Write-Host "`n🎉 All models downloaded successfully!" -ForegroundColor Green
Write-Host "📍 Models directory: $(Resolve-Path $ModelsDir)" -ForegroundColor Yellow