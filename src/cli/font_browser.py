"""
Font Library Browser
=====================

Browse available font profiles with rich table display.
"""

from pathlib import Path
from typing import List, Dict
import json
from rich.console import Console
from rich.table import Table
from rich import box

console = Console()


class FontBrowser:
    """Browse font library with rich tables."""
    
    def __init__(self, fonts_dir: str = "fonts/samples"):
        """Initialize browser with fonts directory."""
        self.fonts_dir = Path(fonts_dir)
    
    def load_profiles(self) -> List[Dict]:
        """Load all complete font profiles."""
        profiles = []
        
        for profile_file in sorted(self.fonts_dir.glob("*.complete.json")):
            try:
                with open(profile_file, 'r') as f:
                    profile = json.load(f)
                    profiles.append(profile)
            except Exception as e:
                console.print(f"[yellow]⚠️  Skipped {profile_file.name}: {e}[/yellow]")
        
        return profiles
    
    def display_fonts_table(self):
        """Display fonts in a beautiful table."""
        profiles = self.load_profiles()
        
        if not profiles:
            console.print("[red]❌ No fonts found in library[/red]")
            return
        
        console.print(f"\n[cyan bold]Available Fonts ({len(profiles)} total)[/cyan bold]\n")
        
        # Create table
        table = Table(box=box.ROUNDED, show_header=True, header_style="bold magenta")
        
        table.add_column("Font Name", style="cyan", no_wrap=True)
        table.add_column("Category", style="green")
        table.add_column("X-Height", justify="right", style="yellow")
        table.add_column("Width Var", justify="right", style="blue")
        table.add_column("Glyphs", justify="right", style="white")
        table.add_column("Vietnamese", justify="center")
        
        # Add rows
        for profile in profiles:
            name = profile['metadata']['font_name']
            category = profile.get('metadata_layer', {}).get('google_fonts', {}).get('category', 'Unknown')
            x_height = profile['basic_metrics']['x_height_ratio']
            width_var = profile['basic_metrics']['width_variance']
            glyphs = profile['unicode_coverage']['total_glyphs']
            vietnamese = profile['unicode_coverage']['supports_vietnamese']
            
            vietnamese_icon = "✅" if vietnamese else "❌"
            
            table.add_row(
                name,
                category,
                f"{x_height:.3f}",
                f"{width_var:.3f}",
                str(glyphs),
                vietnamese_icon
            )
        
        console.print(table)
        console.print()
    
    def browse(self):
        """Main browse entry point."""
        self.display_fonts_table()
        
        console.print("\n[dim]Press Enter to return to main menu...[/dim]")
        input()


if __name__ == '__main__':
    browser = FontBrowser()
    browser.browse()
