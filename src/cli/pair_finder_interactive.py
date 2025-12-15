"""
Interactive Pair Finder
========================

Interactive interface for finding complementary font pairs.
"""

from pathlib import Path
import json
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich import box
import questionary
from questionary import Style
import sys

sys.path.insert(0, 'src')

from pairing.finder import ComplementaryFinder
from pairing.explainer import PairingExplainer

console = Console()

custom_style = Style([
    ('qmark', 'fg:#673ab7 bold'),
    ('question', 'bold'),
    ('answer', 'fg:#f44336 bold'),
    ('pointer', 'fg:#673ab7 bold'),
    ('highlighted', 'fg:#673ab7 bold'),
])


class InteractivePairFinder:
    """Interactive pair finding with prompts."""
    
    def __init__(self, fonts_dir: str = "fonts/samples"):
        """Initialize finder."""
        self.fonts_dir = Path(fonts_dir)
        self.finder = ComplementaryFinder()
        self.explainer = PairingExplainer()
    
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
                    'path': str(profile_path),
                    'category': profile.get('metadata_layer', {}).get('google_fonts', {}).get('category', 'Unknown')
                })
        
        return fonts
    
    def run(self):
        """Run interactive pair finder."""
        console.print("\n[cyan bold]🔍 Find Complementary Pairs[/cyan bold]\n")
        
        # Step 1: Select font
        fonts = self.get_available_fonts()
        
        if not fonts:
            console.print("[red]❌ No fonts found in library[/red]\n")
            console.print("[dim]Press Enter to continue...[/dim]")
            input()
            return
        
        font_choices = [f"{f['name']} ({f['category']})" for f in fonts]
        
        selected_font = questionary.select(
            "Select a font to find pairs for:",
            choices=font_choices,
            style=custom_style
        ).ask()
        
        if not selected_font:
            return
        
        # Find the selected font path
        selected_index = font_choices.index(selected_font)
        query_font = fonts[selected_index]
        
        # Step 2: Number of results
        limit = questionary.text(
            "How many pairs to show?",
            default="3",
            validate=lambda text: text.isdigit() and 1 <= int(text) <= 10,
            style=custom_style
        ).ask()
        
        if not limit:
            return
        
        limit = int(limit)
        
        # Step 3: Explanation level
        explain_choice = questionary.select(
            "Explanation detail:",
            choices=[
                "Compact (scores only)",
                "Detailed (full structural analysis)"
            ],
            style=custom_style
        ).ask()
        
        if not explain_choice:
            return
        
        detailed = "Detailed" in explain_choice
        
        # Find pairs
        console.print(f"\n[dim]Searching for complementary pairs...[/dim]\n")
        
        try:
            # Load query profile
            query_profile = self.finder.load_font_profile(query_font['path'])
            
            # Find pairs
            pairs = self.finder.find_complementary_pairs(
                query_font_path=query_font['path'],
                limit=limit,
                min_score=0.5
            )
            
            if not pairs:
                console.print("[yellow]⚠️  No complementary pairs found[/yellow]\n")
            else:
                # Display results
                if detailed:
                    for i, pair in enumerate(pairs, 1):
                        explanation = self.explainer.generate_explanation(
                            font_a=query_profile,
                            font_b=pair['profile'],
                            pairing_score=pair['pairing_score'],
                            components=pair['components']
                        )
                        console.print(f"[bold]{i}.[/bold]{explanation}")
                else:
                    # Compact table view
                    table = Table(box=box.ROUNDED, show_header=True, header_style="bold magenta")
                    table.add_column("#", style="dim", width=3)
                    table.add_column("Font Name", style="cyan")
                    table.add_column("Score", justify="right", style="green")
                    table.add_column("X-Height", justify="right", style="yellow")
                    table.add_column("Width", justify="right", style="blue")
                    table.add_column("Perceptual", justify="right", style="magenta")
                    
                    for i, pair in enumerate(pairs, 1):
                        score_color = "green" if pair['pairing_score'] > 0.8 else "yellow" if pair['pairing_score'] > 0.6 else "red"
                        
                        table.add_row(
                            str(i),
                            pair['font_name'],
                            f"[{score_color}]{pair['pairing_score']:.3f}[/{score_color}]",
                            f"{pair['components']['x_height_score']:.2f}",
                            f"{pair['components']['width_variance_score']:.2f}",
                            f"{pair['components']['perceptual_score']:.2f}"
                        )
                    
                    console.print(table)
        
        except Exception as e:
            console.print(f"[red]❌ Error: {e}[/red]")
        
        console.print("\n[dim]Press Enter to continue...[/dim]")
        input()


if __name__ == '__main__':
    finder = InteractivePairFinder()
    finder.run()
