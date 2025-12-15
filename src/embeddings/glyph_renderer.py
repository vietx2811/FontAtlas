"""
Glyph Renderer
==============

Render font glyphs as images for perceptual analysis.

Uses Pillow (PIL) for rendering since it's lightweight and sufficient
for our needs. Cairo could be used for higher quality, but adds complexity.

Per spec: Render standard glyph set `H O n a e g` as black-on-white images.
"""

from PIL import Image, ImageDraw, ImageFont
from pathlib import Path
from typing import List, Tuple
import numpy as np


class GlyphRenderer:
    """Render font glyphs as images."""
    
    # Standard glyph set per spec
    STANDARD_GLYPHS = ['H', 'O', 'n', 'a', 'e', 'g']
    
    def __init__(self, font_path: str, size: int = 128):
        """
        Initialize renderer.
        
        Args:
            font_path: Path to font file
            size: Font size in pixels for rendering
        """
        self.font_path = Path(font_path)
        self.size = size
        
        try:
            # Load font using Pillow
            self.font = ImageFont.truetype(str(self.font_path), size=size)
        except Exception as e:
            raise ValueError(f"Could not load font {font_path}: {e}")
    
    def render_glyph(self, char: str, image_size: Tuple[int, int] = (256, 256)) -> Image.Image:
        """
        Render a single glyph.
        
        Args:
            char: Character to render
            image_size: Output image size (width, height)
        
        Returns:
            PIL Image with glyph (black on white)
        """
        # Create white background
        img = Image.new('L', image_size, color=255)  # 'L' = grayscale
        draw = ImageDraw.Draw(img)
        
        # Get text bounding box to center glyph
        bbox = draw.textbbox((0, 0), char, font=self.font)
        text_width = bbox[2] - bbox[0]
        text_height = bbox[3] - bbox[1]
        
        # Center position
        x = (image_size[0] - text_width) // 2 - bbox[0]
        y = (image_size[1] - text_height) // 2 - bbox[1]
        
        # Draw glyph (black on white)
        draw.text((x, y), char, fill=0, font=self.font)  # 0 = black
        
        return img
    
    def render_standard_set(self, image_size: Tuple[int, int] = (256, 256)) -> List[Image.Image]:
        """
        Render all standard glyphs.
        
        Args:
            image_size: Output image size for each glyph
        
        Returns:
            List of rendered glyph images
        """
        images = []
        for char in self.STANDARD_GLYPHS:
            try:
                img = self.render_glyph(char, image_size)
                images.append(img)
            except Exception as e:
                print(f"⚠️  Could not render '{char}': {e}")
        
        return images
    
    def create_composite_image(self, image_size: Tuple[int, int] = (256, 256)) -> Image.Image:
        """
        Create composite image with all standard glyphs in a row.
        
        Useful for visualization and quick inspection.
        
        Args:
            image_size: Size of each individual glyph
        
        Returns:
            Composite image with all glyphs
        """
        glyph_images = self.render_standard_set(image_size)
        
        # Calculate composite dimensions
        num_glyphs = len(glyph_images)
        composite_width = image_size[0] * num_glyphs
        composite_height = image_size[1]
        
        # Create composite
        composite = Image.new('L', (composite_width, composite_height), color=255)
        
        # Paste each glyph
        for i, glyph_img in enumerate(glyph_images):
            x_offset = i * image_size[0]
            composite.paste(glyph_img, (x_offset, 0))
        
        return composite
    
    def save_renders(self, output_dir: str, prefix: str = 'glyph'):
        """
        Save rendered glyphs to files.
        
        Args:
            output_dir: Directory to save images
            prefix: Filename prefix
        """
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)
        
        images = self.render_standard_set()
        
        for char, img in zip(self.STANDARD_GLYPHS, images):
            filename = f"{prefix}_{char}.png"
            img.save(output_path / filename)
        
        # Also save composite
        composite = self.create_composite_image()
        composite.save(output_path / f"{prefix}_composite.png")
        
        print(f"✓ Saved {len(images)} glyph renders + composite to {output_dir}")


def test_renderer():
    """Test glyph renderer."""
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python glyph_renderer.py <font_file.ttf>")
        sys.exit(1)
    
    font_path = sys.argv[1]
    
    print(f"\nRendering glyphs from {Path(font_path).name}...")
    
    renderer = GlyphRenderer(font_path, size=128)
    
    # Test render
    images = renderer.render_standard_set()
    
    print(f"✓ Rendered {len(images)} glyphs: {' '.join(GlyphRenderer.STANDARD_GLYPHS)}")
    
    # Save to test output
    font_name = Path(font_path).stem
    output_dir = f"data/renders/{font_name}"
    renderer.save_renders(output_dir, prefix=font_name)
    
    # Show composite
    composite = renderer.create_composite_image()
    print(f"\n✓ Composite image size: {composite.size}")
    print(f"✓ Files saved to {output_dir}")


if __name__ == '__main__':
    test_renderer()
