#!/usr/bin/env python3
"""
FontAtlas CLI
=============

Interactive menu-driven interface for font pairing.

Usage:
    python fontatlas.py              # Interactive mode (default)
    python fontatlas.py --help       # Show all commands
"""

import typer
from rich.console import Console
import sys
import os

# Add src to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from cli.menu import MainMenu

app = typer.Typer(
    name="fontatlas",
    help="FontAtlas - Anatomy-based font pairing system",
    add_completion=False
)

console = Console()


@app.command()
def interactive():
    """Launch interactive menu (default)."""
    menu = MainMenu()
    menu.run()


@app.callback(invoke_without_command=True)
def main(ctx: typer.Context):
    """Launch FontAtlas interactive menu."""
    if ctx.invoked_subcommand is None:
        interactive()


if __name__ == "__main__":
    app()
