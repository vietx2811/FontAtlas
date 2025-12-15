"""
Test Similarity Search
=======================

Validate that Qdrant similarity search works correctly.
"""

import sys
sys.path.insert(0, 'src')

from database.vector_db import FontAtlasVectorDB
import json


def test_similarity_search():
    """Test similarity search with Roboto as query."""
    
    print("\n" + "="*70)
    print("Testing Similarity Search")
    print("="*70 + "\n")
    
    # Load Roboto as query font
    print("1. Loading Roboto as query font...")
    with open('fonts/samples/roboto.complete.json', 'r') as f:
        roboto = json.load(f)
    
    query_vector = roboto['perceptual_layer']['aggregated_embedding']
    print(f"   Query vector: {len(query_vector)} dims")
    print(f"   First 5 values: {query_vector[:5]}\n")
    
    # Initialize database
    print("2. Connecting to Qdrant...")
    db = FontAtlasVectorDB()
    
    # Get collection info
    info = db.get_collection_info()
    print(f"   Collection: {info['name']}")
    print(f"   Points: {info['points_count']}")
    print(f"   Status: {info['status']}\n")
    
    # Search for similar fonts
    print("3. Searching for top 3 similar fonts...\n")
    results = db.search_similar(query_vector, limit=3)
    
    # Display results
    print(f"{'='*70}")
    print("Results:")
    print(f"{'='*70}\n")
    
    for i, result in enumerate(results, 1):
        print(f"{i}. {result['font_name']}")
        print(f"   Score: {result['score']:.6f}")
        
        payload = result['payload']
        print(f"   Category: {payload.get('category', 'N/A')}")
        print(f"   x-height ratio: {payload.get('x_height_ratio', 0):.4f}")
        print(f"   Width variance: {payload.get('width_variance', 0):.4f}")
        print()
    
    # Validate results
    print(f"{'='*70}")
    print("Validation:")
    print(f"{'='*70}\n")
    
    assert len(results) >= 1, "❌ Should return at least 1 result"
    print("✓ Returned results")
    
    assert results[0]['font_name'] == 'Roboto Regular', f"❌ Top result should be Roboto, got {results[0]['font_name']}"
    print(f"✓ Top result is Roboto Regular")
    
    assert results[0]['score'] > 0.99, f"❌ Self-similarity should be ~1.0, got {results[0]['score']}"
    print(f"✓ Self-similarity score: {results[0]['score']:.6f} (>0.99)")
    
    print("\n" + "="*70)
    print("✅ All tests passed!")
    print("="*70 + "\n")
    
    # Show other similar fonts
    if len(results) > 1:
        print("Most similar fonts to Roboto:")
        for i, result in enumerate(results[1:], 2):
            print(f"  {i}. {result['font_name']} (score: {result['score']:.4f})")


if __name__ == '__main__':
    test_similarity_search()
