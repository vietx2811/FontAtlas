"""
Fonts Router
=============

Endpoints for font management and retrieval.
"""

from fastapi import APIRouter, HTTPException, Query
from pathlib import Path
import json
from typing import List, Optional
from pydantic import BaseModel

from api.models import FontSummary, FontListResponse

router = APIRouter()

# Default font directory (relative to project root, not src/)
DEFAULT_FONT_DIR = "../fonts/samples"


def load_font_profiles(directory: str = DEFAULT_FONT_DIR) -> List[dict]:
    """Load all font profiles from disk."""
    profiles_dir = Path(directory)
    
    # Return empty list if directory doesn't exist
    if not profiles_dir.exists():
        print(f"Warning: Directory '{directory}' does not exist")
        return []
    
    if not profiles_dir.is_dir():
        print(f"Warning: '{directory}' is not a directory")
        return []
    
    profiles = []
    
    # Look for .complete.json files
    json_files = list(profiles_dir.glob("*.complete.json"))
    
    if json_files:
        # Load existing profiles
        for file in sorted(json_files):
            try:
                with open(file, 'r') as f:
                    profile = json.load(f)
                    profile['_id'] = file.stem.replace('.complete', '')
                    profile['_path'] = str(file)
                    profiles.append(profile)
            except Exception as e:
                print(f"Error loading {file}: {e}")
    else:
        # No profiles found, scan for TTF files to create basic entries
        ttf_files = list(profiles_dir.glob("*.ttf"))
        for ttf_file in sorted(ttf_files):
            # Create minimal profile from TTF
            profile = {
                '_id': ttf_file.stem,
                '_path': str(ttf_file),
                '_is_ttf_only': True,  # Flag to indicate no full profile
                'metadata': {
                    'font_name': ttf_file.stem.replace('_', ' ').title(),
                    'file_path': str(ttf_file)
                },
                'basic_metrics': {},
                'anatomy_metrics': {}
            }
            profiles.append(profile)
    
    return profiles


@router.get("/directories")
async def list_font_directories():
    """
    List available font directories in the project.
    
    Returns common font directories that may contain .ttf files.
    """
    # Base directory is project root (one level up from src/)
    base_dir = Path("..").resolve()
    
    potential_dirs = [
        "fonts/samples",
        "fonts",
        "fonts/system",
        "fonts/custom"
    ]
    
    available = []
    for dir_path in potential_dirs:
        full_path = base_dir / dir_path
        if full_path.exists() and full_path.is_dir():
            ttf_count = len(list(full_path.glob("*.ttf")))
            json_count = len(list(full_path.glob("*.complete.json")))
            
            # Store path relative to where server runs (src/)
            relative_from_src = f"../{dir_path}"
            
            available.append({
                "path": relative_from_src,
                "absolute_path": str(full_path),
                "ttf_files": ttf_count,
                "profile_files": json_count
            })
    
    return {"directories": available}


@router.get("/browse")
async def browse_filesystem(path: str = Query(".", description="Path to browse")):
    """
    Browse the filesystem to select a font directory.
    
    Returns list of subdirectories in the specified path.
    WARNING: No security restrictions - can browse entire filesystem.
    """
    import os
    
    # Normalize and resolve path
    try:
        base_path = Path(".").resolve()
        browse_path = Path(path).resolve()
        
        # No security checks - allow browsing anywhere
        if not browse_path.exists() or not browse_path.is_dir():
            raise HTTPException(status_code=404, detail="Directory not found")
        
        # Get subdirectories and count font files
        items = []
        for item in sorted(browse_path.iterdir()):
            if item.is_dir() and not item.name.startswith('.'):
                # Count font files in this directory
                try:
                    ttf_count = len(list(item.glob("*.ttf")))
                    otf_count = len(list(item.glob("*.otf")))
                    json_count = len(list(item.glob("*.complete.json")))
                except PermissionError:
                    # Skip directories we can't access
                    continue
                
                items.append({
                    "name": item.name,
                    "path": str(item),
                    "relative_path": str(item.relative_to(base_path)) if item.is_relative_to(base_path) else str(item),
                    "ttf_files": ttf_count,
                    "otf_files": otf_count,
                    "profile_files": json_count,
                    "has_fonts": (ttf_count + otf_count + json_count) > 0
                })
        
        # Get parent directory info
        parent = None
        parent_path = browse_path.parent
        # Always allow going to parent (unless at root)
        if parent_path != browse_path:
            parent = {
                "name": "..",
                "path": str(parent_path),
                "relative_path": str(parent_path.relative_to(base_path)) if parent_path.is_relative_to(base_path) else str(parent_path)
            }
        
        return {
            "current_path": str(browse_path),
            "current_relative_path": str(browse_path.relative_to(base_path)) if browse_path.is_relative_to(base_path) else str(browse_path),
            "parent": parent,
            "directories": items
        }
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error browsing directory: {str(e)}")


@router.get("/", response_model=FontListResponse)
async def list_fonts(directory: Optional[str] = Query(DEFAULT_FONT_DIR, description="Font directory path")):
    """
    List all available fonts with summary information.
    
    Returns basic metrics and anatomy data for each font.
    Can optionally specify a custom directory.
    Handles both .complete.json profiles and raw .ttf files.
    """
    profiles = load_font_profiles(directory)
    
    fonts = []
    for profile in profiles:
        # Extract summary data
        font_id = profile.get('_id', 'unknown')
        name = profile.get('metadata', {}).get('font_name', 'Unknown')
        
        # Check if this is a TTF-only entry (no full profile)
        is_ttf_only = profile.get('_is_ttf_only', False)
        
        if is_ttf_only:
            # Minimal info for TTF files without profiles
            fonts.append(FontSummary(
                id=font_id,
                name=name,
                category='Unknown',
                x_height=0.0,
                width_variance=0.0,
                stroke_contrast=0.0,
                aperture=0.0
            ))
        elif profile.get('basic_metrics') and profile.get('anatomy_metrics'):
            # Full profile data
            # Get category from classification (preferred) or Google Fonts (fallback)
            classification = profile.get('metadata_layer', {}).get('classification', {})
            auto_category = classification.get('auto_category')
            google_category = profile.get('metadata_layer', {}).get('google_fonts', {}).get('category', 'Unknown')
            
            # Prefer auto-classification, fallback to Google Fonts
            category = auto_category if auto_category else google_category
            
            basic_metrics = profile.get('basic_metrics', {})
            anatomy = profile.get('anatomy_metrics', {})
            
            # Safe metric extraction with defaults
            x_height = basic_metrics.get('x_height_ratio')
            if x_height is None or not isinstance(x_height, (int, float)):
                x_height = 0.0
            
            width_var = basic_metrics.get('width_variance')
            if width_var is None or not isinstance(width_var, (int, float)):
                width_var = 0.0
            
            stroke = anatomy.get('stroke_contrast', {}).get('mean_contrast')
            if stroke is None or not isinstance(stroke, (int, float)):
                stroke = 1.0
            
            aperture = anatomy.get('aperture_openness', {}).get('mean_aperture')
            if aperture is None or not isinstance(aperture, (int, float)):
                aperture = 0.5
            
            fonts.append(FontSummary(
                id=font_id,
                name=name,
                category=category,
                x_height=float(x_height),
                width_variance=float(width_var),
                stroke_contrast=float(stroke),
                aperture=float(aperture)
            ))
    
    return FontListResponse(fonts=fonts, count=len(fonts))


# Global scanner instance for progress tracking
active_scanners = {}

class ScanRequest(BaseModel):
    directory: str
    force: bool = False  # Force rescan existing profiles


@router.post("/scan")
async def scan_directory(request: ScanRequest):
    """
    Scan a directory for TTF files and generate complete profiles.
    
    Args:
        directory: Directory path
        force: If True, rescan existing profiles
    
    Returns a task ID for progress tracking.
    """
    from api.font_scanner import FontScanner
    import threading
    import uuid
    
    directory = request.directory
    force = request.force
    
    # Generate task ID
    task_id = str(uuid.uuid4())
    
    # Create scanner
    scanner = FontScanner(directory, force=force)
    active_scanners[task_id] = scanner
    
    # Run scan in background thread
    def run_scan():
        try:
            scanner.scan()
        except Exception as e:
            scanner.progress['status'] = 'error'
            scanner.progress['errors'].append(str(e))
    
    thread = threading.Thread(target=run_scan, daemon=True)
    thread.start()
    
    return {
        "task_id": task_id,
        "message": "Scan started",
        "directory": directory
    }


@router.get("/scan/{task_id}/progress")
async def get_scan_progress(task_id: str):
    """
    Get progress of a scanning task.
    """
    scanner = active_scanners.get(task_id)
    
    if not scanner:
        raise HTTPException(status_code=404, detail="Task not found")
    
    progress = scanner.get_progress()
    
    # Clean up completed tasks
    if progress['status'] in ['complete', 'error']:
        # Keep for 1 minute then cleanup
        pass
    
    return progress


# Indexing functionality
active_indexers = {}

@router.post("/index")
async def index_to_qdrant(directory: str):
    """
    Index all font profiles from a directory into Qdrant.
    
    Returns a task ID for progress tracking.
    """
    import threading
    import uuid
    from database.vector_db import FontVectorDB
    
    # Generate task ID
    task_id = str(uuid.uuid4())
    
    # Progress tracker
    progress = {
        'total': 0,
        'current': 0,
        'status': 'idle',
        'current_file': '',
        'errors': []
    }
    active_indexers[task_id] = progress
    
    def run_index():
        try:
            progress['status'] = 'indexing'
            
            # Find all profiles
            profile_dir = Path(directory)
            if not profile_dir.exists():
                progress['status'] = 'error'
                progress['errors'].append(f"Directory {directory} not found")
                return
            
            profiles = list(profile_dir.glob("*.complete.json"))
            progress['total'] = len(profiles)
            
            if len(profiles) == 0:
                progress['status'] = 'error'
                progress['errors'].append("No .complete.json profiles found")
                return
            
            # Initialize Qdrant
            vector_db = FontVectorDB()
            
            # Index each profile
            for i, profile_path in enumerate(profiles):
                progress['current'] = i + 1
                progress['current_file'] = profile_path.name
                
                try:
                    vector_db.upsert_font_profile(str(profile_path))
                except Exception as e:
                    progress['errors'].append(f"{profile_path.name}: {str(e)}")
            
            progress['status'] = 'complete'
            
        except Exception as e:
            progress['status'] = 'error'
            progress['errors'].append(str(e))
    
    thread = threading.Thread(target=run_index, daemon=True)
    thread.start()
    
    return {
        "task_id": task_id,
        "message": "Indexing started",
        "directory": directory
    }


@router.get("/index/{task_id}/progress")
async def get_index_progress(task_id: str):
    """
    Get progress of an indexing task.
    """
    progress = active_indexers.get(task_id)
    
    if not progress:
        raise HTTPException(status_code=404, detail="Task not found")
    
    return progress


@router.get("/font-file/{font_id}")
async def serve_font_file(font_id: str, directory: Optional[str] = Query(DEFAULT_FONT_DIR)):
    """
    Serve TTF font file for browser preview.
    
    Returns font file with proper headers for @font-face.
    """
    from fastapi.responses import FileResponse
    
    font_path = Path(directory) / f"{font_id}.ttf"
    
    if not font_path.exists():
        raise HTTPException(status_code=404, detail=f"Font file not found: {font_id}")
    
    return FileResponse(
        path=str(font_path),
        media_type="font/ttf",
        headers={
            "Cache-Control": "public, max-age=31536000",
            "Access-Control-Allow-Origin": "*"
        }
    )


@router.get("/{font_id}")
async def get_font_profile(font_id: str, directory: Optional[str] = Query(DEFAULT_FONT_DIR, description="Font directory path")):
    """
    Get complete profile for a specific font.
    
    Returns all layers: metrics, coverage, metadata, perceptual, anatomy.
    """
    # Try with .complete.json extension first
    profile_path = Path(directory) / f"{font_id}.complete.json"
    
    if not profile_path.exists():
        raise HTTPException(status_code=404, detail=f"Font '{font_id}' not found in {directory}")
    
    with open(profile_path, 'r') as f:
        profile = json.load(f)
    
    return profile
