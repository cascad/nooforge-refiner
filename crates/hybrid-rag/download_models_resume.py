#!/usr/bin/env python3
"""
Download with resume support for large files
"""

import requests
from pathlib import Path

def download_with_resume(url: str, filename: Path):
    """Download with resume support"""
    if filename.exists():
        # Получаем текущий размер файла
        downloaded = filename.stat().st_size
        headers = {'Range': f'bytes={downloaded}-'}
    else:
        downloaded = 0
        headers = {}
    
    try:
        response = requests.get(url, headers=headers, stream=True, timeout=30)
        
        if response.status_code == 416:  # Range Not Satisfiable
            print(f"✅ Download already complete: {filename.name}")
            return True
            
        response.raise_for_status()
        
        mode = 'ab' if downloaded > 0 else 'wb'
        total_size = downloaded + int(response.headers.get('content-length', 0))
        
        print(f"📥 Downloading {filename.name}...")
        
        with open(filename, mode) as f:
            for chunk in response.iter_content(chunk_size=8192):
                if chunk:
                    f.write(chunk)
                    downloaded += len(chunk)
                    print(f"   Progress: {downloaded}/{total_size} bytes", end='\r')
        
        print(f"\n✅ Downloaded: {filename.name}")
        return True
        
    except Exception as e:
        print(f"\n❌ Error downloading {filename.name}: {e}")
        return False

def main():
    models_dir = Path("models")
    models_dir.mkdir(exist_ok=True)
    
    files = [
        ("https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx", 
         models_dir / "all-MiniLM-L6-v2.onnx"),
        ("https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
         models_dir / "tokenizer.json"),
    ]
    
    for url, path in files:
        download_with_resume(url, path)

if __name__ == "__main__":
    main()