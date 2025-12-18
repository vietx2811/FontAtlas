#!/usr/bin/env python3
"""
Migrate Font Profiles to Qdrant
================================

Re-index all existing font profiles into Qdrant with enhanced full payload.

Usage:
    python migrate_to_qdrant.py [directory]
    
Example:
    python migrate_to_qdrant.py ../fonts/samples
"""

import sys
from pathlib import Path
from database.vector_db import FontAtlasVectorDB

def migrate_directory(directory: str, recreate_collection: bool = False):
    """
    Migrate all font profiles from a directory to Qdrant.
    
    Args:
        directory: Path to directory containing .complete.json files
        recreate_collection: If True, delete and recreate collection first
    """
    profiles_path = Path(directory)
    
    if not profiles_path.exists():
        print(f"❌ Directory not found: {directory}")
        return False
    
    # Find all profile files
    profile_files = list(profiles_path.glob("*.complete.json"))
    
    if not profile_files:
        print(f"❌ No .complete.json files found in {directory}")
        return False
    
    print(f"\n{'='*70}")
    print(f"Qdrant Migration: {len(profile_files)} font profiles")
    print(f"Directory: {directory}")
    print(f"{'='*70}\n")
    
    # Initialize Qdrant
    try:
        vector_db = FontAtlasVectorDB()
        
        if recreate_collection:
            print("🔄 Recreating collection...\n")
            vector_db.create_collection(recreate=True)
        else:
            # Ensure collection exists
            try:
                vector_db.create_collection(recreate=False)
            except:
                pass
        
    except Exception as e:
        print(f"❌ Failed to connect to Qdrant: {e}")
        print("\nMake sure Qdrant is running:")
        print("  docker run -p 6333:6333 qdrant/qdrant")
        return False
    
    # Index each profile
    success_count = 0
    error_count = 0
    
    for i, profile_file in enumerate(sorted(profile_files), 1):
        try:
            print(f"[{i}/{len(profile_files)}] Indexing {profile_file.stem}...")
            
            point_id = vector_db.index_font_profile(str(profile_file))
            
            print(f"  ✓ Point ID: {point_id[:8]}...\n")
            success_count += 1
            
        except Exception as e:
            print(f"  ✗ Error: {e}\n")
            error_count += 1
    
    # Summary
    print(f"{'='*70}")
    print(f"Migration Complete!")
    print(f"  ✓ Success: {success_count}/{len(profile_files)}")
    if error_count > 0:
        print(f"  ✗ Errors: {error_count}")
    print(f"{'='*70}\n")
    
    # Verify collection
    try:
        info = vector_db.get_collection_info()
        print(f"Qdrant Collection Status:")
        print(f"  Points count: {info['points_count']}")
        print(f"  Status: {info['status']}")
        print()
    except:
        pass
    
    return success_count == len(profile_files)


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Migrate font profiles to Qdrant')
    parser.add_argument('directory', nargs='?', default='../fonts/samples', 
                       help='Directory containing .complete.json files')
    parser.add_argument('--recreate', action='store_true',
                       help='Delete and recreate collection (WARNING: deletes existing data)')
    
    args = parser.parse_args()
    
    # Confirm if recreating
    if args.recreate:
        response = input("⚠️  This will DELETE all existing data in Qdrant. Continue? (yes/no): ")
        if response.lower() != 'yes':
            print("Aborted.")
            return
    
    success = migrate_directory(args.directory, recreate_collection=args.recreate)
    
    if success:
        print("✅ Migration successful!")
        print("\nNext steps:")
        print("  1. Test API: http://localhost:8000/api/fonts/")
        print("  2. Compare speed: Qdrant vs JSON fallback")
        sys.exit(0)
    else:
        print("❌ Migration failed. Check errors above.")
        sys.exit(1)


if __name__ == '__main__':
    main()
