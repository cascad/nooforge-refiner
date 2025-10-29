# RAG System Setup - Minimal Version

Write-Host "===================================" -ForegroundColor Cyan
Write-Host "RAG System Setup" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan

# Create directories
Write-Host "`nCreating directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path data | Out-Null
New-Item -ItemType Directory -Force -Path .cache | Out-Null
New-Item -ItemType Directory -Force -Path qdrant_storage | Out-Null
Write-Host "Done" -ForegroundColor Green

# Install dependencies
Write-Host "`nInstalling Python dependencies..." -ForegroundColor Yellow
python -m pip install -r requirements.txt

# Start Qdrant
Write-Host "`nStarting Qdrant..." -ForegroundColor Yellow
docker-compose -f llamaindex_full/docker-compose.yml up -d

# Wait
Write-Host "`nWaiting for Qdrant..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check
Write-Host "Checking Qdrant at http://localhost:6333/healthz ..." -ForegroundColor Yellow

Write-Host "`n===================================" -ForegroundColor Green
Write-Host "Setup Complete!" -ForegroundColor Green
Write-Host "===================================" -ForegroundColor Green

Write-Host "`nNext steps:"
Write-Host "  1. Copy .env.example to .env"
Write-Host "  2. Edit .env and add your OPENROUTER_API_KEY"
Write-Host "  3. Put documents in .\data\"
Write-Host "  4. Run: python ingest.py"
Write-Host "  5. Run: python query.py --interactive"