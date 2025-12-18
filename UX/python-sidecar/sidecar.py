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

# Try to import classifier from metrics module
try:
    from metrics.font_classifier import FontClassifier
    HAS_CLASSIFIER = True
except ImportError as e:
    print(f"Warning: Could not import FontClassifier: {e}", file=sys.stderr)
    HAS_CLASSIFIER = False


def parse_font(file_path: str) -> dict:
    """Parse font file and return complete profile"""
    try:
        if HAS_PARSER:
            # Use full FontParser for comprehensive analysis
            parser = FontParser(file_path)
            profile = parser.extract_full_profile()
            
            # Extract metrics for the result
            basic_metrics = profile.get('basic_metrics', {})
            
            result = {
                "success": True,
                "name": basic_metrics.get('font_name', parser.font_name),
                "file_path": file_path,
                "basic_metrics": basic_metrics,
                "anatomy_metrics": {
                    "x_height": basic_metrics.get('x_height_ratio', 0) or 0,
                    "width_variance": basic_metrics.get('width_variance', 0) or 0,
                    "stroke_contrast": 0,  # Not in FontParser yet
                    "aperture": 0,  # Not in FontParser yet
                },
                "classification": "Unknown",
                "unicode_coverage": profile.get('unicode_coverage', {})
            }
            
            # Try classification
            if HAS_CLASSIFIER:
                try:
                    classifier = FontClassifier()
                    result["classification"] = classifier.classify(profile)
                except Exception as e:
                    result["classifier_error"] = str(e)
            
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
