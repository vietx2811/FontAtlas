"""
Profile Viewer
===============

View font profiles with rich formatting.
"""

from pathlib import Path
import json
from rich.console import Console
from rich.panel import Panel
from rich.syntax import Syntax
import questionary
from questionary import Style

console = Console()

custom_style = Style([
    ('qmark', 'fg:#673ab7 bold'),
    ('question', 'bold'),
    ('answer', 'fg:#f44336 bold'),
    ('pointer', 'fg:#673ab7 bold'),
    ('highlighted', 'fg:#673ab7 bold'),
])


class ProfileViewer:
    """View font profiles with formatting."""
    
    def __init__(self, fonts_dir: str = "fonts/samples"):
        """Initialize viewer."""
        self.fonts_dir = Path(fonts_dir)
    
    def get_available_fonts(self):
        """Get list of available fonts."""
        profiles = list(self.fonts_dir.glob("*.complete.json"))
        
        if not profiles:
            return []
        
        fonts = []
        for profile_path in sorted(profiles):
            with open(profile_path, 'r') as f:
                profile = json.load(f)
                fonts.append({
                    'name': profile['metadata']['font_name'],
                    'path': str(profile_path)
                })
        
        return fonts
    
    def display_profile(self, profile):
        """Display profile in formatted view."""
        name = profile['metadata']['font_name']
        
        # Build formatted view
        content = []
        content.append(f"[bold cyan]Font Profile: {name}[/bold cyan]\n")
        
        # Basic Metrics
        content.append("[bold yellow]Basic Metrics[/bold yellow]")
        metrics = profile['basic_metrics']
        content.append(f"  X-height ratio:    {metrics['x_height_ratio']:.4f}")
        content.append(f"  Width variance:    {metrics['width_variance']:.4f}")
        content.append(f"  Width mean:        {metrics['width_mean']:.4f}")
        content.append("")
        
        # Unicode Coverage
        content.append("[bold green]Unicode Coverage[/bold green]")
        coverage = profile['unicode_coverage']
        content.append(f"  Total glyphs:      {coverage['total_glyphs']}")
        content.append(f"  Vietnamese:        {'✅ Yes' if coverage['supports_vietnamese'] else '❌ No'}")
        content.append(f"  Latin Extended:    {'✅ Yes' if coverage['supports_latin_extended'] else '❌ No'}")
        content.append("")
        
        # Metadata
        if 'metadata_layer' in profile and 'google_fonts' in profile['metadata_layer']:
            content.append("[bold magenta]Metadata[/bold magenta]")
            gf = profile['metadata_layer']['google_fonts']
            content.append(f"  Category:          {gf.get('category', 'Unknown')}")
            designers = gf.get('designers', [])
            if designers:
                content.append(f"  Designers:         {', '.join(designers[:2])}")
            if gf.get('popularity_rank'):
                content.append(f"  Popularity:        #{gf['popularity_rank']}")
            content.append("")
        
        # Perceptual Layer
        content.append("[bold blue]Perceptual Layer[/bold blue]")
        perceptual = profile.get('perceptual_layer', {})
        content.append(f"  Embedding dims:    {perceptual.get('embedding_dim', 512)}")
        content.append(f"  Model:             {perceptual.get('model', 'ViT-B-32')}")
        content.append(f"  Glyphs analyzed:   {perceptual.get('num_glyphs', 6)}")
        
        # Display in panel
        panel = Panel(
            "\n".join(content),
            border_style="cyan",
            padding=(1, 2)
        )
        
        console.print()
        console.print(panel)
        console.print()
    
    def run(self):
        """Run interactive profile viewer."""
        console.print("\n[cyan bold]📊 View Font Profile[/cyan bold]\n")
        
        # Select font
        fonts = self.get_available_fonts()
        
        if not fonts:
            console.print("[red]❌ No fonts found in library[/red]\n")
            console.print("[dim]Press Enter to continue...[/dim]")
            input()
            return
        
        font_choices = [f['name'] for f in fonts]
        
        selected_font = questionary.select(
            "Select a font to view:",
            choices=font_choices,
            style=custom_style
        ).ask()
        
        if not selected_font:
            return
        
        # Find the selected font path
        selected_index = font_choices.index(selected_font)
        font_path = fonts[selected_index]['path']
        
        # Load and display profile
        with open(font_path, 'r') as f:
            profile = json.load(f)
        
        self.display_profile(profile)
        
        console.print("[dim]Press Enter to continue...[/dim]")
        input()


if __name__ == '__main__':
    viewer = ProfileViewer()
    viewer.run()
