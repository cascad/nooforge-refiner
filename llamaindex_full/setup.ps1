# RAG System Setup Script for Windows

Write-Host "===================================" -ForegroundColor Cyan
Write-Host "RAG System Setup" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

# Create directories
Write-Host "Creating directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path data | Out-Null
New-Item -ItemType Directory -Force -Path .cache | Out-Null
New-Item -ItemType Directory -Force -Path qdrant_storage | Out-Null
Write-Host "✓ Directories created" -ForegroundColor Green

# Install dependencies
Write-Host ""
Write-Host "Installing Python dependencies..." -ForegroundColor Yellow
python -m pip install -r requirements.txt
if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ Failed to install dependencies" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Dependencies installed" -ForegroundColor Green

# Start Qdrant
Write-Host ""
Write-Host "Starting Qdrant..." -ForegroundColor Yellow
docker-compose up -d
if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ Failed to start Qdrant" -ForegroundColor Red
    Write-Host "Make sure Docker Desktop is running!" -ForegroundColor Yellow
    exit 1
}
Write-Host "✓ Qdrant started" -ForegroundColor Green

# Wait for Qdrant
Write-Host ""
Write-Host "Waiting for Qdrant to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check Qdrant
Write-Host "Checking Qdrant..." -ForegroundColor Yellow
$qdrantCheck = Invoke-WebRequest -Uri "http://localhost:6333/healthz" -UseBasicParsing -TimeoutSec 5 -ErrorAction SilentlyContinue
if ($qdrantCheck) {
    Write-Host "✓ Qdrant is responding" -ForegroundColor Green
} else {
    Write-Host "⚠ Warning: Qdrant not responding yet" -ForegroundColor Yellow
    Write-Host "  Wait a moment and check: http://localhost:6333/dashboard" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✅ Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Copy .env.example to .env and add your OPENROUTER_API_KEY"
Write-Host "  2. Put documents in .\data\"
Write-Host "  3. Run: python ingest.py"
Write-Host "  4. Run: python query.py --interactive"
Write-Host ""