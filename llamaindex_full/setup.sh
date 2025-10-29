#!/bin/bash

set -e

echo "==================================="
echo "RAG System Setup"
echo "==================================="

# Create directories
echo "Creating directories..."
mkdir -p data .cache qdrant_storage

# Install dependencies
echo "Installing Python dependencies..."
pip install -r requirements.txt

# Start Qdrant
echo "Starting Qdrant..."
docker-compose up -d

# Wait for Qdrant
echo "Waiting for Qdrant to start..."
sleep 5

# Check Qdrant
echo "Checking Qdrant..."
curl -s http://localhost:6333/healthz || echo "Warning: Qdrant not responding"

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Put documents in ./data/"
echo "  2. Run: python ingest.py"
echo "  3. Run: python query.py --interactive"