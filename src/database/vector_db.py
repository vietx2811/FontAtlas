"""
Qdrant Vector Database Client
==============================

Manages Qdrant collections and indexing for FontAtlas.

Collection Schema:
- Collection: "font_profiles"
- Vector: 512-dim (OpenCLIP embeddings)
- Payload: basic_metrics, unicode_coverage, metadata_layer
"""

from qdrant_client import QdrantClient
from qdrant_client.models import Distance, VectorParams, PointStruct
from typing import List, Dict
import json
from pathlib import Path
import uuid


class FontAtlasVectorDB:
    """Qdrant vector database client for font profiles."""
    
    COLLECTION_NAME = "font_profiles"
    VECTOR_SIZE = 512  # OpenCLIP ViT-B-32 dimension
    
    def __init__(self, host: str = "localhost", port: int = 6333):
        """
        Initialize Qdrant client.
        
        Args:
            host: Qdrant server host
            port: Qdrant HTTP API port
        """
        self.client = QdrantClient(host=host, port=port)
        self.collection_name = self.COLLECTION_NAME
    
    def create_collection(self, recreate: bool = False):
        """
        Create font profiles collection.
        
        Args:
            recreate: If True, delete existing collection first
        """
        # Check if collection exists
        collections = self.client.get_collections().collections
        collection_exists = any(c.name == self.collection_name for c in collections)
        
        if collection_exists:
            if recreate:
                print(f"⚠️  Deleting existing collection: {self.collection_name}")
                self.client.delete_collection(self.collection_name)
            else:
                print(f"✓ Collection already exists: {self.collection_name}")
                return
        
        # Create collection
        print(f"Creating collection: {self.collection_name}")
        print(f"  Vector size: {self.VECTOR_SIZE}")
        print(f"  Distance metric: Cosine")
        
        self.client.create_collection(
            collection_name=self.collection_name,
            vectors_config=VectorParams(
                size=self.VECTOR_SIZE,
                distance=Distance.COSINE  # Per spec: cosine similarity
            )
        )
        
        print(f"✓ Collection created successfully\n")
    
    def index_font_profile(self, profile_path: str) -> str:
        """
        Index a single font profile with FULL payload.
        
        Args:
            profile_path: Path to complete profile JSON
        
        Returns:
            Point ID (UUID)
        """
        from datetime import datetime
        
        # Load profile
        with open(profile_path, 'r', encoding='utf-8') as f:
            profile = json.load(f)
        
        # Extract vector (perceptual embedding)
        vector = profile['perceptual_layer']['aggregated_embedding']
        
        # Prepare FULL payload (everything for fast queries)
        basic_metrics = profile.get('basic_metrics', {})
        anatomy_metrics = profile.get('anatomy_metrics', {})
        metadata_layer = profile.get('metadata_layer', {})
        classification = metadata_layer.get('classification', {})
        google_fonts = metadata_layer.get('google_fonts', {})
        
        payload = {
            # Metadata
            'font_name': profile['metadata']['font_name'],
            'file_path': profile['metadata']['file_path'],
            'directory': str(Path(profile['metadata']['file_path']).parent),
            
            # Basic Metrics (FULL)
            'basic_metrics': {
                'x_height': basic_metrics.get('x_height'),
                'cap_height': basic_metrics.get('cap_height'),
                'x_height_ratio': basic_metrics.get('x_height_ratio'),
                'optical_x_ratio': basic_metrics.get('optical_x_ratio'),
                'x_height_source': basic_metrics.get('x_height_source'),
                'virtual_x_height': basic_metrics.get('virtual_x_height'),
                'virtual_x_ratio': basic_metrics.get('virtual_x_ratio'),
                'width_variance': basic_metrics.get('width_variance'),
                'width_mean': basic_metrics.get('width_mean'),
                'width_min': basic_metrics.get('width_min'),
                'width_max': basic_metrics.get('width_max'),
                'units_per_em': basic_metrics.get('units_per_em'),
                'ascender': basic_metrics.get('ascender'),
                'descender': basic_metrics.get('descender'),
            },
            
            # Anatomy Metrics (summary only, full data in JSON)
            'anatomy_metrics': {
                'stroke_contrast': {
                    'mean_contrast': anatomy_metrics.get('stroke_contrast', {}).get('mean_contrast'),
                    'classification': anatomy_metrics.get('stroke_contrast', {}).get('classification'),
                },
                'aperture_openness': {
                    'mean_aperture': anatomy_metrics.get('aperture_openness', {}).get('mean_aperture'),
                    'classification': anatomy_metrics.get('aperture_openness', {}).get('classification'),
                }
            },
            
            # Classification
            'classification': {
                'auto_category': classification.get('auto_category'),
                'confidence': classification.get('confidence'),
                'reasoning': classification.get('reasoning'),
                'google_fonts_category': classification.get('google_fonts_category'),
            },
            
            # Unicode Coverage
            'unicode_coverage': {
                'total_glyphs': profile.get('unicode_coverage', {}).get('total_glyphs'),
                'supports_vietnamese': profile.get('unicode_coverage', {}).get('supports_vietnamese'),
                'supports_latin_extended': profile.get('unicode_coverage', {}).get('supports_latin_extended'),
            },
            
            # Google Fonts Metadata (if available)
            'google_fonts': {
                'category': google_fonts.get('category'),
                'designers': google_fonts.get('designers', []),
                'popularity_rank': google_fonts.get('popularity_rank'),
            },
            
            # Timestamps
            'indexed_at': datetime.utcnow().isoformat() + 'Z',
        }
        
        # Generate point ID from font file path (consistent ID)
        point_id = str(uuid.uuid5(uuid.NAMESPACE_DNS, profile['metadata']['file_path']))
        
        # Upload to Qdrant
        self.client.upsert(
            collection_name=self.collection_name,
            points=[
                PointStruct(
                    id=point_id,
                    vector=vector,
                    payload=payload
                )
            ]
        )
        
        return point_id
    
    def batch_index_profiles(self, profiles_dir: str = 'fonts/samples') -> List[str]:
        """
        Index all complete profiles in directory.
        
        Args:
            profiles_dir: Directory containing .complete.json files
        
        Returns:
            List of indexed point IDs
        """
        profiles_path = Path(profiles_dir)
        profile_files = list(profiles_path.glob('*.complete.json'))
        
        if not profile_files:
            print(f"No complete profiles found in {profiles_dir}")
            return []
        
        print(f"\n{'='*70}")
        print(f"Batch Indexing: {len(profile_files)} font profiles")
        print(f"{'='*70}\n")
        
        point_ids = []
        
        for i, profile_file in enumerate(sorted(profile_files), 1):
            try:
                print(f"[{i}/{len(profile_files)}] Indexing {profile_file.stem}...")
                
                point_id = self.index_font_profile(str(profile_file))
                point_ids.append(point_id)
                
                print(f"  ✓ Point ID: {point_id[:8]}...\n")
                
            except Exception as e:
                print(f"  ✗ Error: {e}\n")
        
        print(f"{'='*70}")
        print(f"✓ Indexed {len(point_ids)}/{len(profile_files)} profiles")
        print(f"{'='*70}\n")
        
        return point_ids
    
    def get_font_by_id(self, font_id: str) -> Dict | None:
        """
        Retrieve full font profile from Qdrant by ID.
        
        Args:
            font_id: Point ID (UUID)
        
        Returns:
            Font profile dict or None if not found
        """
        try:
            result = self.client.retrieve(
                collection_name=self.collection_name,
                ids=[font_id]
            )
            
            if not result:
                return None
            
            point = result[0]
            return {
                'id': point.id,
                **point.payload  # Unpack full payload
            }
        except Exception as e:
            print(f"Error retrieving font {font_id}: {e}")
            return None
    
    def get_fonts_by_directory(self, directory: str, limit: int = 1000) -> List[Dict]:
        """
        List all fonts in a directory from Qdrant.
        
        Args:
            directory: Directory path to filter by
            limit: Maximum number of results
        
        Returns:
            List of font profiles
        """
        from qdrant_client.models import Filter, FieldCondition, MatchValue
        
        # Query Qdrant with directory filter
        results = self.client.scroll(
            collection_name=self.collection_name,
            scroll_filter=Filter(
                must=[
                    FieldCondition(
                        key="directory",
                        match=MatchValue(value=directory)
                    )
                ]
            ),
            limit=limit
        )
        
        fonts = []
        for point in results[0]:  # scroll returns (points, next_page_offset)
            fonts.append({
                'id': str(point.id),
                **point.payload
            })
        
        return fonts
    
    def search_fonts_with_filters(
        self,
        category: str | None = None,
        min_confidence: float = 0.0,
        supports_vietnamese: bool | None = None,
        limit: int = 100
    ) -> List[Dict]:
        """
        Advanced search with multiple filters.
        
        Args:
            category: Filter by auto_category
            min_confidence: Minimum classification confidence
            supports_vietnamese: Filter Vietnamese support
            limit: Max results
        
        Returns:
            List of matching fonts
        """
        from qdrant_client.models import Filter, FieldCondition, MatchValue, Range
        
        conditions = []
        
        if category:
            conditions.append(
                FieldCondition(
                    key="classification.auto_category",
                    match=MatchValue(value=category)
                )
            )
        
        if min_confidence > 0:
            conditions.append(
                FieldCondition(
                    key="classification.confidence",
                    range=Range(gte=min_confidence)
                )
            )
        
        if supports_vietnamese is not None:
            conditions.append(
                FieldCondition(
                    key="unicode_coverage.supports_vietnamese",
                    match=MatchValue(value=supports_vietnamese)
                )
            )
        
        if not conditions:
            # No filters, return all
            results = self.client.scroll(
                collection_name=self.collection_name,
                limit=limit
            )
        else:
            results = self.client.scroll(
                collection_name=self.collection_name,
                scroll_filter=Filter(must=conditions),
                limit=limit
            )
        
        fonts = []
        for point in results[0]:
            fonts.append({
                'id': str(point.id),
                **point.payload
            })
        
        return fonts
    
    def search_similar(self, query_vector: List[float], limit: int = 5, filter_dict: Dict = None) -> List[Dict]:
        """
        Search for similar fonts.
        
        Args:
            query_vector: 512-dim query embedding
            limit: Number of results
            filter_dict: Optional filters (e.g., {'category': 'Sans Serif'})
        
        Returns:
            List of results with scores and payloads
        """
        results = self.client.query_points(
            collection_name=self.collection_name,
            query=query_vector,
            limit=limit,
            query_filter=filter_dict
        ).points
        
        return [
            {
                'id': result.id,
                'score': result.score,
                'font_name': result.payload['font_name'],
                'payload': result.payload
            }
            for result in results
        ]
    
    def get_collection_info(self) -> Dict:
        """Get collection statistics."""
        info = self.client.get_collection(self.collection_name)
        
        return {
            'name': self.collection_name,
            'points_count': info.points_count,
            'status': info.status,
            'vector_size': self.VECTOR_SIZE,
        }


def main():
    """CLI for Qdrant operations."""
    import argparse
    
    parser = argparse.ArgumentParser(description='FontAtlas Vector Database')
    parser.add_argument('--create', action='store_true', help='Create collection')
    parser.add_argument('--recreate', action='store_true', help='Recreate collection (delete + create)')
    parser.add_argument('--index', metavar='DIR', help='Index profiles from directory')
    parser.add_argument('--info', action='store_true', help='Show collection info')
    
    args = parser.parse_args()
    
    # Initialize client
    db = FontAtlasVectorDB(host='localhost', port=6333)
    
    if args.create or args.recreate:
        db.create_collection(recreate=args.recreate)
    
    if args.index:
        db.batch_index_profiles(args.index)
    
    if args.info:
        try:
            info = db.get_collection_info()
            print("\n" + "="*70)
            print("Collection Info")
            print("="*70)
            print(f"Points count: {info['points_count']}")
            print(f"Vectors count: {info['vectors_count']}")
            print(f"Status: {info['status']}")
            print()
        except Exception as e:
            print(f"✗ Error getting collection info: {e}")
    
    # Default: create + index if no args
    if not any([args.create, args.recreate, args.index, args.info]):
        print("Creating collection and indexing fonts/samples/\n")
        db.create_collection(recreate=False)
        db.batch_index_profiles('fonts/samples')
        
        # Show info
        info = db.get_collection_info()
        print(f"Collection: {db.collection_name}")
        print(f"Points indexed: {info['points_count']}")


if __name__ == '__main__':
    main()
