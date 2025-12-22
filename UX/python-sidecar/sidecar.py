#!/usr/bin/env python3
"""
FontAtlas Python Sidecar
=========================

Handles font parsing and analysis for Tauri desktop app.
Communicates with Rust backend via stdin/stdout JSON protocol.
"""

import sys
import json
import os
from pathlib import Path
from typing import Dict, Any, Optional

# Get absolute path to FontAtlas src directory
SCRIPT_DIR = Path(__file__).parent.absolute()
FONTATLAS_ROOT = SCRIPT_DIR.parent.parent  # UX/../.. = FontAtlas
SRC_DIR = FONTATLAS_ROOT / 'src'

# Add src to Python path
sys.path.insert(0, str(SRC_DIR))

from fontTools.ttLib import TTFont

# Try to import FontParser from parsers module
try:
    from parsers.font_parser import FontParser
    HAS_PARSER = True
except ImportError as e:
    print(f"Warning: Could not import FontParser: {e}", file=sys.stderr)
    HAS_PARSER = False

# Try to import classifier - bundled locally to avoid heavy deps from metrics/__init__.py
try:
    from font_classifier import FontClassifier
    HAS_CLASSIFIER = True
except Exception as e:
    print(f"Warning: Could not import FontClassifier: {e}", file=sys.stderr)
    HAS_CLASSIFIER = False

# Qdrant import will be lazy-loaded in _upsert_to_qdrant to speed up script startup
HAS_QDRANT = True # Assumed true until proven otherwise during execution


def parse_font(file_path: str, font_id: Optional[str] = None) -> dict:
    """Parse font file and return complete profile"""
    global HAS_QDRANT
    try:
        if HAS_PARSER:
            # Use full FontParser for comprehensive analysis
            parser = FontParser(file_path)
            profile = parser.extract_full_profile()
            
            # Extract metrics for the result
            basic_metrics = profile.get('basic_metrics', {})
            anatomy_metrics = profile.get('anatomy_metrics', {})
            
            result = {
                "success": True,
                "name": basic_metrics.get('font_name', parser.font_name),
                "file_path": file_path,
                "basic_metrics": basic_metrics,
                "anatomy_metrics": {
                    "x_height": basic_metrics.get('x_height_ratio', 0) or 0,
                    "width_variance": basic_metrics.get('width_variance', 0) or 0,
                    "stroke_contrast": anatomy_metrics.get('stroke_contrast', 0) or 0,
                    "aperture": anatomy_metrics.get('aperture', 0) or 0,
                    "terminal_sharpness": anatomy_metrics.get('terminal_sharpness', 0) or 0,
                    "stress_angle": anatomy_metrics.get('stress_angle', 0) or 0,
                    "curvature_ratio": anatomy_metrics.get('curvature_ratio', 0) or 0,
                    "counter_density": anatomy_metrics.get('counter_density', 0) or 0,
                },
                "classification": "Unknown",
                "unicode_coverage": profile.get('unicode_coverage', {}),
                # New Metadata
                "foundry": profile.get('metadata', {}).get('foundry'),
                "weight": profile.get('weight_style', {}).get('weight', 400),
                "is_italic": profile.get('weight_style', {}).get('is_italic', False),
                # Family Info
                "family_name": profile.get('family_info', {}).get('family_name'),
                "subfamily": profile.get('family_info', {}).get('subfamily'),
                "weight_class": profile.get('family_info', {}).get('weight_class', 400),
                "width_class": profile.get('family_info', {}).get('width_class', 5),
                # Flatted language support for easier DB insertion
                "supports_vietnamese": profile.get('unicode_coverage', {}).get('supports_vietnamese', False),
                "panose": profile.get('panose'),
                "id": font_id
            }
            
            # Try classification
            if HAS_CLASSIFIER:
                try:
                    # Use static method classify_with_confidence if available, otherwise instance
                    if hasattr(FontClassifier, 'classify_with_confidence'):
                         cls_res = FontClassifier.classify_with_confidence(profile)
                         result["classification"] = cls_res['category']
                         result["classification_confidence"] = cls_res['confidence']
                         
                         # Enhanced taxonomy (new system)
                         if 'enhanced' in cls_res:
                             enhanced = cls_res['enhanced']
                             result["primary_class"] = enhanced.get('primary_class')
                             result["secondary_tags"] = enhanced.get('secondary_tags')  # List
                             result["serif_subtype"] = enhanced.get('serif_subtype')
                             result["sans_subtype"] = enhanced.get('sans_subtype')
                             result["script_subtype"] = enhanced.get('script_subtype')
                    else:
                        classifier = FontClassifier()
                        result["classification"] = classifier.classify(profile)
                except Exception as e:
                    result["classifier_error"] = str(e)

            # Upsert to Qdrant (Lazy attempt)
            if HAS_QDRANT:
                try:
                    _upsert_to_qdrant(result, font_id)
                    result["qdrant_status"] = "synced"
                except ImportError:
                    # If lazy import fails
                    result["qdrant_status"] = "skipped_no_client_lib"
                    HAS_QDRANT = False
                except Exception as e:
                    # Log error but don't fail the whole parse
                    result["qdrant_status"] = f"error: {str(e)}"
            else:
                result["qdrant_status"] = "skipped_previously_failed"
            
            return result
        else:
            # Fallback: minimal parsing with fontTools
            font = TTFont(file_path)
            font_name = _get_font_name(font)
            
            return {
                "success": True,
                "name": font_name,
                "file_path": file_path,
                "basic_metrics": {},
                "anatomy_metrics": {},
                "classification": "Unknown",
                "unicode_coverage": {}
            }
        
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "file_path": file_path
        }


def generate_embedding(profile: dict) -> dict:
    """Generate UMAP 2D coordinates for font map"""
    try:
        # TODO: Implement UMAP reduction
        # For now, return placeholder
        return {
            "success": True,
            "coords": [0.0, 0.0]  # Will be actual UMAP coords
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e)
        }


def _get_font_name(font: TTFont) -> str:
    """Extract font name from name table"""
    name_table = font.get('name')
    if not name_table:
        return "Unknown"
    
    # Try to get full font name (ID 4) or family name (ID 1)
    for name_id in [4, 1]:
        for record in name_table.names:
            if record.nameID == name_id and record.platformID == 3:
                return record.toUnicode()
    
    return "Unknown"


def generate_visual_embedding(file_path: str, font_id: Optional[str] = None) -> Dict[str, Any]:
    """
    Generate 512D visual embedding using OpenCLIP and upsert to Qdrant.
    
    This is an optional background process for enhanced visual similarity search.
    """
    import sys
    import os
    from pathlib import Path
    from contextlib import redirect_stdout, redirect_stderr
    import io
    
    try:
        # Add FontAtlas src to path for embeddings module
        FONTATLAS_ROOT = Path(__file__).parent.parent.parent
        SRC_DIR = FONTATLAS_ROOT / 'src'
        sys.path.insert(0, str(SRC_DIR))
        
        from embeddings.perceptual_embedder import PerceptualEmbedder
        from embeddings.glyph_renderer import GlyphRenderer
        
        # Render glyphs (H O n a e g)
        renderer = GlyphRenderer(file_path, size=128)
        glyph_images = renderer.render_standard_set(image_size=(256, 256))
        
        if not glyph_images:
            return {
                "success": False,
                "error": "Failed to render glyphs",
                "file_path": file_path
            }
        
        # Extract 512D embedding - suppress OpenCLIP's stdout/stderr during loading
        stdout_buffer = io.StringIO()
        stderr_buffer = io.StringIO()
        with redirect_stdout(stdout_buffer), redirect_stderr(stderr_buffer):
            embedder = PerceptualEmbedder(model_name='ViT-B-32', pretrained='openai')
            embedding_data = embedder.embed_glyph_set(glyph_images)
        
        # Log any output from OpenCLIP to stderr for debugging
        if stdout_output := stdout_buffer.getvalue():
            print(f"OpenCLIP stdout: {stdout_output}", file=sys.stderr)
        if stderr_output := stderr_buffer.getvalue():
            print(f"OpenCLIP stderr: {stderr_output}", file=sys.stderr)
        
        perceptual_vector = embedding_data['aggregated_embedding']
        
        # Upsert to Qdrant with named vector
        upsert_res = _upsert_visual_to_qdrant(file_path, perceptual_vector, font_id)
        
        if not upsert_res.get("success", False):
            return {
                "success": False,
                "error": upsert_res.get("error", "Unknown Qdrant error"),
                "file_path": file_path
            }

        return {
            "success": True,
            "file_path": file_path,
            "font_id": font_id,
            "embedding_dim": embedding_data['embedding_dim'],
            "num_glyphs": embedding_data['num_glyphs']
        }

        
    except ImportError as e:
        return {
            "success": False,
            "error": f"OpenCLIP not available: {str(e)}",
            "file_path": file_path
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "file_path": file_path
        }


def _upsert_visual_to_qdrant(file_path: str, perceptual_vector: list, font_id: Optional[str] = None):
    """Upsert 512D visual embedding to Qdrant with named vector 'perceptual'."""
    from qdrant_client import QdrantClient
    from qdrant_client.models import PointVectors
    import uuid
    import os
    
    font_name = os.path.basename(file_path)
    
    try:
        client = QdrantClient(url="http://localhost:6333", timeout=5.0)
        collection_name = "font_profiles"
        
        # Use provided font_id if available, otherwise fallback to path-based UUID
        point_id = font_id if font_id else str(uuid.uuid5(uuid.NAMESPACE_URL, file_path))
        
        try:
            # Check if point exists first
            existing = client.retrieve(collection_name=collection_name, ids=[point_id])
            
            if existing:
                # Update the 'perceptual' named vector using update_vectors API
                client.update_vectors(
                    collection_name=collection_name,
                    points=[
                        PointVectors(
                            id=point_id,
                            vector={"perceptual": perceptual_vector}
                        )
                    ]
                )
                
                # Also update payload to mark has_visual_embedding
                client.set_payload(
                    collection_name=collection_name,
                    payload={"has_visual_embedding": True},
                    points=[point_id]
                )
                print(f"[QDRANT] ✓ Upserted perceptual(512D) for {font_name} (id: {point_id[:8]}...)", file=sys.stderr)
                return {"success": True}
            else:
                # Point is missing in Qdrant! Step 1 (metrics) may have failed.
                print(f"[QDRANT] ✗ Point not found for {font_name} - metrics upsert may have failed", file=sys.stderr)
                return {
                    "success": False, 
                    "error": f"Font point {point_id} not found in Qdrant. Step 1 (metrics) may have failed."
                }
                
        except Exception as e:
            print(f"[QDRANT] ✗ update_vectors failed for {font_name}: {e}", file=sys.stderr)
            return {"success": False, "error": f"Qdrant update_vectors failed: {str(e)}"}
                
    except Exception as e:
        print(f"[QDRANT] ✗ Connection failed: {e}", file=sys.stderr)
        return {"success": False, "error": f"Qdrant connection failed: {str(e)}"}



def _upsert_to_qdrant(data: Dict[str, Any], font_id: Optional[str] = None):
    """Upsert font metrics and metadata to Qdrant."""
    # Lazy import to reduce startup time for the script
    from qdrant_client import QdrantClient
    from qdrant_client.models import PointStruct, VectorParams, Distance
    import httpx

    # Connect to Qdrant with short timeout to prevent hanging
    # Use localhost:6333 default from docker-compose
    try:
        client = QdrantClient(
            url="http://localhost:6333", 
            timeout=5.0 
        )
        collection_name = "font_profiles"
        
        # Optimistic upsert: try to upload first
        # If collection doesn't exist, we'll get an error, then we create it.
        # This saves a network round-trip "get_collections" on every single file.
        
        # Prepare vector and payload first
        metrics = data.get('anatomy_metrics', {})
        weight = data.get('weight', 400)
        
        def safe_float(val, default=0.0):
            try:
                return float(val) if val is not None else default
            except (ValueError, TypeError):
                return default

        # 9-dimensional vector with all metrics
        vector = [
            safe_float(weight) / 1000.0,                              # 0: weight (normalized)
            safe_float(metrics.get('width_variance'), 0.5),           # 1: width_variance
            safe_float(metrics.get('stroke_contrast'), 1.0) / 5.0,    # 2: stroke_contrast (normalized)
            safe_float(metrics.get('x_height'), 0.5),                 # 3: x_height
            safe_float(metrics.get('aperture'), 0.5),                 # 4: aperture
            safe_float(metrics.get('curvature_ratio'), 0.4),          # 5: curvature_ratio
            safe_float(metrics.get('terminal_sharpness'), 0.5),       # 6: terminal_sharpness
            safe_float(metrics.get('stress_angle'), 90.0) / 180.0,    # 7: stress_angle (normalized)
            safe_float(metrics.get('counter_density'), 0.5),          # 8: counter_density
        ]
        
        # Use provided font_id if available, otherwise fallback to path-based UUID
        point_id = font_id if font_id else str(uuid.uuid5(uuid.NAMESPACE_URL, data['file_path']))
        
        payload = {
            "name": data['name'],
            "path": data['file_path'],
            "foundry": data.get('foundry'),
            "category": data.get('classification'),
            "primary_class": data.get('primary_class'),
            "weight": weight,
            "is_italic": data.get('is_italic'),
            "supports_vietnamese": data.get('supports_vietnamese'),
            # Include all metrics in payload for filtering/display
            "width_variance": safe_float(metrics.get('width_variance'), 0),
            "stroke_contrast": safe_float(metrics.get('stroke_contrast'), 0),
            "x_height": safe_float(metrics.get('x_height'), 0),
            "aperture": safe_float(metrics.get('aperture'), 0),
            "curvature_ratio": safe_float(metrics.get('curvature_ratio'), 0),
            "terminal_sharpness": safe_float(metrics.get('terminal_sharpness'), 0),
            "stress_angle": safe_float(metrics.get('stress_angle'), 0),
            "counter_density": safe_float(metrics.get('counter_density'), 0),
            # For font pairing feature
            "suitable_roles": data.get('suitable_roles', []),  # ["body", "display", "accent", "ui"]
            "secondary_tags": data.get('secondary_tags', []),  # Classification tags
        }

        try:
           client.upsert(
                collection_name=collection_name,
                points=[
                    PointStruct(
                        id=point_id,
                        vector={"metrics": vector},  # Named vector
                        payload=payload
                    )
                ]
            )
           print(f"[QDRANT] ✓ Upserted metrics for {data['name']} (id: {point_id[:8]}...)", file=sys.stderr)
        except Exception as e:
            # Check if error is due to missing collection
            # "Not found" or 404
            err_str = str(e).lower()
            if "not found" in err_str or "404" in err_str:
                # Create collection with NAMED vectors (metrics 9D + perceptual 512D)
                print(f"[QDRANT] Creating collection '{collection_name}' with named vectors...", file=sys.stderr)
                client.create_collection(
                    collection_name=collection_name,
                    vectors_config={
                        "metrics": VectorParams(size=9, distance=Distance.COSINE),
                        "perceptual": VectorParams(size=512, distance=Distance.COSINE),
                    }
                )
                print(f"[QDRANT] ✓ Collection created with metrics(9D) + perceptual(512D)", file=sys.stderr)
                client.upsert(
                    collection_name=collection_name,
                    points=[
                        PointStruct(
                            id=point_id,
                            vector={"metrics": vector},  # Named vector
                            payload=payload
                        )
                    ]
                )
                print(f"[QDRANT] ✓ Upserted metrics for {data['name']} (id: {point_id[:8]}...)", file=sys.stderr)
            else:
                print(f"[QDRANT] ✗ Failed to upsert {data['name']}: {e}", file=sys.stderr)
                raise e

    except Exception:
        # Re-raise to be caught in main loop
        raise


def handle_command(command: dict) -> dict:
    """Process incoming command and return response"""
    cmd_type = command.get('command')
    
    if cmd_type == 'parse':
        file_path = command.get('filepath')
        font_id = command.get('font_id')
        return parse_font(file_path, font_id)
    
    elif cmd_type == 'embed':
        profile = command.get('profile')
        return generate_embedding(profile)
    
    elif cmd_type == 'embed_visual':
        # Generate 512D visual embedding and upsert to Qdrant
        file_path = command.get('filepath')
        font_id = command.get('font_id')  # Optional: for tracking
        return generate_visual_embedding(file_path, font_id)
    
    elif cmd_type == 'ping':
        return {"success": True, "message": "pong"}
    
    else:
        return {
            "success": False,
            "error": f"Unknown command: {cmd_type}"
        }


def main():
    """Main loop - read from stdin, write to stdout"""
    # Unbuffered I/O
    sys.stdout = os.fdopen(sys.stdout.fileno(), 'w', buffering=1)
    sys.stderr = os.fdopen(sys.stderr.fileno(), 'w', buffering=1)
    
    # Ready signal
    print(json.dumps({"ready": True}), flush=True)
    
    # Process commands
    for line in sys.stdin:
        try:
            command = json.loads(line.strip())
            response = handle_command(command)
            print(json.dumps(response), flush=True)
        except json.JSONDecodeError as e:
            error_response = {
                "success": False,
                "error": f"Invalid JSON: {str(e)}"
            }
            print(json.dumps(error_response), flush=True)
        except Exception as e:
            error_response = {
                "success": False,
                "error": f"Unexpected error: {str(e)}"
            }
            print(json.dumps(error_response), flush=True)


if __name__ == '__main__':
    main()
