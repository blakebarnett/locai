#!/usr/bin/env python3
"""Verify embeddings are valid without involving SurrealDB"""

import json
import os
import sys
import math
from pathlib import Path

def calculate_norm(embedding):
    """Calculate L2 norm of embedding vector"""
    return math.sqrt(sum(x * x for x in embedding))

def validate_embedding(embedding, text=""):
    """Validate an embedding vector"""
    issues = []
    
    # Check dimensions
    if len(embedding) != 1024:
        issues.append(f"Expected 1024 dimensions, got {len(embedding)}")
    
    # Check for NaN or Infinity
    for i, val in enumerate(embedding):
        if math.isnan(val):
            issues.append(f"NaN value at index {i}")
        if math.isinf(val):
            issues.append(f"Infinite value at index {i}")
    
    # Check normalization (should be close to unit length for cosine similarity)
    norm = calculate_norm(embedding)
    if norm < 0.9 or norm > 1.1:
        issues.append(f"Norm is {norm:.6}, expected ~1.0 (normalized)")
    
    # Check value range (should be reasonable for normalized vectors)
    min_val = min(embedding)
    max_val = max(embedding)
    if min_val < -10.0 or max_val > 10.0:
        issues.append(f"Values out of reasonable range: [{min_val:.6}, {max_val:.6}]")
    
    # Check that not all values are zero
    all_zero = all(abs(x) < 1e-6 for x in embedding)
    if all_zero:
        issues.append("All values are zero (or very close to zero)")
    
    return len(issues) == 0, issues

def verify_quickstart_embeddings():
    """Verify embeddings from quickstart_embeddings.json"""
    print("1. Checking quickstart_embeddings.json...")
    
    script_dir = Path(__file__).parent
    embeddings_path = script_dir.parent / "src" / "quickstart_embeddings.json"
    
    if not embeddings_path.exists():
        print(f"   ⚠️  File not found: {embeddings_path}")
        return
    
    try:
        with open(embeddings_path, 'r') as f:
            data = json.load(f)
        
        print(f"   ✓ File loaded successfully")
        print(f"   ✓ Found {len(data)} embedding entries\n")
        
        valid_count = 0
        invalid_count = 0
        
        for idx, item in enumerate(data):
            text = item.get("text", "")
            embedding_raw = item.get("embedding", [])
            
            if not text or not embedding_raw:
                print(f"   ⚠️  Entry {idx + 1}: Missing 'text' or 'embedding' field")
                invalid_count += 1
                continue
            
            # Convert to list of floats
            embedding = [float(x) for x in embedding_raw]
            
            if len(embedding) == 0:
                print(f"   ⚠️  Entry {idx + 1}: Empty embedding for text: {text}")
                invalid_count += 1
                continue
            
            is_valid, issues = validate_embedding(embedding, text)
            
            if is_valid:
                valid_count += 1
                norm = calculate_norm(embedding)
                min_val = min(embedding)
                max_val = max(embedding)
                print(f"   ✓ Entry {idx + 1}: Valid embedding for '{text[:50]}...'")
                print(f"      Dimensions: {len(embedding)}, Norm: {norm:.6}, Range: [{min_val:.6}, {max_val:.6}]")
            else:
                invalid_count += 1
                print(f"   ⚠️  Entry {idx + 1}: Invalid embedding for '{text[:50]}...'")
                for issue in issues:
                    print(f"      - {issue}")
        
        print(f"\n   Summary: {valid_count} valid, {invalid_count} invalid")
        
    except json.JSONDecodeError as e:
        print(f"   ❌ Failed to parse JSON: {e}")
    except Exception as e:
        print(f"   ❌ Error: {e}")

def generate_mock_embedding(text, dimensions):
    """Generate mock embedding using the same algorithm as Rust code"""
    embedding = [0.0] * dimensions
    
    # Create deterministic values based on text content
    for i, c in enumerate(text):
        idx = i % dimensions
        char_val = ord(c) % 255
        embedding[idx] += (char_val / 255.0) * 0.1
    
    # Add some variation based on text length and hash
    text_hash = sum(ord(c) for c in text)
    for i in range(dimensions):
        embedding[i] += ((i + text_hash) % 100) / 1000.0
    
    # Normalize to unit length
    norm = calculate_norm(embedding)
    if norm > 0.0:
        embedding = [x / norm for x in embedding]
    
    return embedding

def verify_mock_embeddings():
    """Verify mock embedding generation"""
    print("\n2. Testing mock embedding generation...\n")
    
    test_texts = [
        "The protagonist is a skilled warrior named John",
        "John met Alice in the tavern last week",
        "The kingdom has been at war for three years",
        "warrior",
        "character",
    ]
    
    for text in test_texts:
        embedding = generate_mock_embedding(text, 1024)
        is_valid, issues = validate_embedding(embedding, text)
        
        if is_valid:
            norm = calculate_norm(embedding)
            min_val = min(embedding)
            max_val = max(embedding)
            print(f"   ✓ Mock embedding for '{text[:50]}...': Valid")
            print(f"      Dimensions: {len(embedding)}, Norm: {norm:.6}, Range: [{min_val:.6}, {max_val:.6}]")
        else:
            print(f"   ⚠️  Mock embedding for '{text[:50]}...': Invalid")
            for issue in issues:
                print(f"      - {issue}")
    
    # Test determinism
    print("\n   Testing determinism...")
    text = "test text"
    emb1 = generate_mock_embedding(text, 1024)
    emb2 = generate_mock_embedding(text, 1024)
    if emb1 == emb2:
        print("   ✓ Mock embeddings are deterministic")
    else:
        print("   ⚠️  Mock embeddings are NOT deterministic (unexpected)")
        # Check how different they are
        diff = sum(abs(a - b) for a, b in zip(emb1, emb2))
        print(f"      Total difference: {diff:.10f}")

def main():
    print("🔍 Verifying Embeddings\n")
    print("=" * 60)
    
    verify_quickstart_embeddings()
    verify_mock_embeddings()
    
    print("\n" + "=" * 60)
    print("✅ Verification complete!")

if __name__ == "__main__":
    main()

