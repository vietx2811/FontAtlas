"""
Main Interactive Menu
======================

Menu-driven CLI for FontAtlas.
"""

from rich.console import Console
from rich.panel import Panel
from rich.text import Text
import questionary
from questionary import Style

console = Console()

# Custom style for questionary
custom_style = Style([
    ('qmark', 'fg:#673ab7 bold'),
    ('question', 'bold'),
    ('answer', 'fg:#f44336 bold'),
    ('pointer', 'fg:#673ab7 bold'),
    ('highlighted', 'fg:#673ab7 bold'),
    ('selected', 'fg:#cc5454'),
    ('separator', 'fg:#cc5454'),
    ('instruction', ''),
    ('text', ''),
    ('disabled', 'fg:#858585 italic')
])


class MainMenu:
    """Main interactive menu for FontAtlas."""
    
    def __init__(self):
        """Initialize menu."""
        self.running = True
    
    def display_header(self):
        """Display application header."""
        console.clear()
        
        title = Text()
        title.append("FontAtlas", style="bold cyan")
        title.append(" - Font Pairing System\n", style="bold white")
        title.append("Anatomy-Based Analysis", style="dim italic")
        
        panel = Panel(
            title,
            border_style="cyan",
            padding=(1, 2)
        )
        
        console.print()
        console.print(panel)
        console.print()
    
    def show_menu(self):
        """Display menu options."""
        choices = [
            "📚 Browse Font Library",
            "🔍 Find Complementary Pairs",
            "📊 View Font Profile",
            "⚙️  Batch Analyze New Fonts",
            "💾 Export Pairing Report",
            "ℹ️  System Status",
            "❌ Exit"
        ]
        
        selected = questionary.select(
            "Select an option:",
            choices=choices,
            style=custom_style
        ).ask()
        
        return selected
    
    def handle_browse(self):
        """Handle font browsing."""
        from cli.font_browser import FontBrowser
        
        browser = FontBrowser()
        browser.browse()
    
    def handle_find_pairs(self):
        """Handle pairing finder."""
        from cli.pair_finder_interactive import InteractivePairFinder
        
        finder = InteractivePairFinder()
        finder.run()
    
    def handle_view_profile(self):
        """Handle profile viewing."""
        from cli.profile_viewer import ProfileViewer
        
        viewer = ProfileViewer()
        viewer.run()
    
    def handle_batch_analyze(self):
        """Handle batch analysis."""
        console.print("\n[yellow]⚙️  Batch analysis coming soon![/yellow]\n")
        console.print("[dim]Press Enter to continue...[/dim]")
        input()
    
    def handle_export(self):
        """Handle export."""
        console.print("\n[yellow]💾 Export functionality coming soon![/yellow]\n")
        console.print("[dim]Press Enter to continue...[/dim]")
        input()
    
    def handle_system_status(self):
        """Handle system status."""
        import sys
        sys.path.insert(0, 'src')
        
        from database.vector_db import FontAtlasVectorDB
        
        console.print("\n[cyan bold]System Status[/cyan bold]\n")
        
        # Check Qdrant
        try:
            db = FontAtlasVectorDB()
            info = db.get_collection_info()
            
            console.print(f"✅ Qdrant Connection: [green]Healthy[/green]")
            console.print(f"📊 Collection: [cyan]{info['name']}[/cyan]")
            console.print(f"🔢 Indexed Fonts: [yellow]{info['points_count']}[/yellow]")
            console.print(f"📏 Vector Dimensions: [blue]{info['vector_size']}[/blue]")
            console.print(f"💚 Status: [green]{info['status']}[/green]")
        except Exception as e:
            console.print(f"❌ Qdrant Connection: [red]Failed[/red]")
            console.print(f"   Error: [dim]{e}[/dim]")
        
        console.print("\n[dim]Press Enter to continue...[/dim]")
        input()
    
    def run(self):
        """Main menu loop."""
        try:
            while self.running:
                self.display_header()
                
                choice = self.show_menu()
                
                if choice is None:  # User cancelled
                    break
                
                if "Browse" in choice:
                    self.handle_browse()
                elif "Find Complementary" in choice:
                    self.handle_find_pairs()
                elif "View Font Profile" in choice:
                    self.handle_view_profile()
                elif "Batch Analyze" in choice:
                    self.handle_batch_analyze()
                elif "Export" in choice:
                    self.handle_export()
                elif "System Status" in choice:
                    self.handle_system_status()
                elif "Exit" in choice:
                    console.print("\n[green]👋 Goodbye![/green]\n")
                    break
        
        except KeyboardInterrupt:
            console.print("\n\n[yellow]⚠️  Interrupted by user[/yellow]\n")
        except Exception as e:
            console.print(f"\n[red]❌ Error: {e}[/red]\n")


if __name__ == '__main__':
    menu = MainMenu()
    menu.run()
