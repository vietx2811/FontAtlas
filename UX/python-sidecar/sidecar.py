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


def parse_font(file_path: str) -> dict:
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
                "panose": profile.get('panose')
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
                    _upsert_to_qdrant(result)
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


def _upsert_to_qdrant(data: Dict[str, Any]):
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
            timeout=1.0 # 1 second timeout for connection
        )
        collection_name = "fonts"
        
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

        vector = [
            safe_float(weight) / 1000.0,
            safe_float(metrics.get('width_variance'), 0.5), 
            safe_float(metrics.get('stroke_contrast'), 1.0) / 5.0, 
            safe_float(metrics.get('x_height'), 0.5),
            safe_float(metrics.get('aperture'), 0.5)
        ]
        
        # Generate stable ID from file path
        import uuid
        point_id = str(uuid.uuid5(uuid.NAMESPACE_URL, data['file_path']))
        
        payload = {
            "name": data['name'],
            "path": data['file_path'],
            "foundry": data.get('foundry'),
            "category": data.get('classification'),
            "weight": weight,
            "is_italic": data.get('is_italic'),
            "supports_vietnamese": data.get('supports_vietnamese'),
        }

        try:
           client.upsert(
                collection_name=collection_name,
                points=[
                    PointStruct(
                        id=point_id,
                        vector=vector,
                        payload=payload
                    )
                ]
            )
        except Exception as e:
            # Check if error is due to missing collection
            # "Not found" or 404
            err_str = str(e).lower()
            if "not found" in err_str or "404" in err_str:
                # Create collection and retry
                client.create_collection(
                    collection_name=collection_name,
                    vectors_config=VectorParams(size=5, distance=Distance.COSINE)
                )
                client.upsert(
                    collection_name=collection_name,
                    points=[
                        PointStruct(
                            id=point_id,
                            vector=vector,
                            payload=payload
                        )
                    ]
                )
            else:
                raise e

    except Exception:
        # Re-raise to be caught in main loop
        raise


def handle_command(command: dict) -> dict:
    """Process incoming command and return response"""
    cmd_type = command.get('command')
    
    if cmd_type == 'parse':
        file_path = command.get('filepath')
        return parse_font(file_path)
    
    elif cmd_type == 'embed':
        profile = command.get('profile')
        return generate_embedding(profile)
    
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
