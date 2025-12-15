"""
Font Scanner
=============

Background task for scanning and analyzing fonts in a directory.
"""

from pathlib import Path
from typing import Dict, List
import json
from parsers.font_parser import FontParser
from parsers.profile_integrator import FontProfileIntegrator

class FontScanner:
    """Scan directory and generate font profiles."""
    
    def __init__(self, directory: str, force: bool = False):
        """
        Initialize scanner.
        
        Args:
            directory: Directory to scan
            force: If True, rescan even if .complete.json exists
        """
        self.directory = Path(directory)
        self.force = force
        self.progress = {
            'total': 0,
            'current': 0,
            'status': 'idle',
            'current_file': '',
            'errors': []
        }
    
    def scan(self):
        """Scan directory for TTF files and generate profiles."""
        if not self.directory.exists():
            raise ValueError(f"Directory {self.directory} does not exist")
        
        print(f"Starting scan with force={self.force}")
        
        # Find all TTF files
        ttf_files = list(self.directory.glob("*.ttf"))
        self.progress['total'] = len(ttf_files)
        self.progress['status'] = 'scanning'
        
        if len(ttf_files) == 0:
            self.progress['status'] = 'complete'
            return
        
        # Process each font
        for i, ttf_file in enumerate(ttf_files):
            self.progress['current'] = i + 1
            self.progress['current_file'] = ttf_file.name
            
            try:
                # Check if profile already exists
                profile_path = ttf_file.with_suffix('.complete.json')
                if profile_path.exists() and not self.force:
                    print(f"Skipping {ttf_file.name} (profile exists)")
                    continue
                
                # Generate profile
                action = "Re-processing" if profile_path.exists() else "Processing"
                print(f"{action} {ttf_file.name}...")
                integrator = FontProfileIntegrator()
                profile = integrator.create_complete_profile(str(ttf_file))
                
                # Save profile
                with open(profile_path, 'w') as f:
                    json.dump(profile, f, indent=2)
                
                print(f"✓ Created profile for {ttf_file.name}")
                
            except Exception as e:
                error_msg = f"Error processing {ttf_file.name}: {str(e)}"
                print(error_msg)
                self.progress['errors'].append(error_msg)
        
        self.progress['status'] = 'complete'
    
    def get_progress(self) -> Dict:
        """Get current progress."""
        return self.progress.copy()
