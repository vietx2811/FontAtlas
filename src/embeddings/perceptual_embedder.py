"""
OpenCLIP Perceptual Embedder
=============================

Extract perceptual embeddings from glyph images using OpenCLIP.

Per spec:
- Use OpenCLIP (not FontCLIP - avoid pre-trained biases)
- Extract 512-dim vectors
- Weight: 30% in final vector composition
- Captures: soft vs rigid, visual texture, humanist vs mechanical
"""

import torch
import open_clip
from PIL import Image
import numpy as np
from typing import List, Dict
from pathlib import Path


class PerceptualEmbedder:
    """Extract perceptual embeddings using OpenCLIP."""
    
    def __init__(self, model_name: str = 'ViT-B-32', pretrained: str = 'openai'):
        """
        Initialize embedder with OpenCLIP model.
        
        Args:
            model_name: CLIP model architecture
                - 'ViT-B-32': Good balance (512-dim embeddings)
                - 'ViT-L-14': Larger, more accurate (768-dim)
            pretrained: Pretrained weights ('openai', 'laion400m', etc.)
        """
        self.model_name = model_name
        self.pretrained = pretrained
        
        print(f"Loading OpenCLIP model: {model_name} ({pretrained})...")
        
        # Load model and preprocessing
        self.model, _, self.preprocess = open_clip.create_model_and_transforms(
            model_name,
            pretrained=pretrained
        )
        
        # Move to GPU if available
        self.device = 'cuda' if torch.cuda.is_available() else 'cpu'
        self.model = self.model.to(self.device)
        self.model.eval()  # Inference mode
        
        print(f"✓ Model loaded on {self.device}")
    
    def embed_image(self, image: Image.Image) -> np.ndarray:
        """
        Extract embedding from a single image.
        
        Args:
            image: PIL Image (glyph render)
        
        Returns:
            Embedding vector (normalized)
        """
        # Preprocess image
        # OpenCLIP expects RGB, so convert grayscale to RGB
        if image.mode != 'RGB':
            image = image.convert('RGB')
        
        img_tensor = self.preprocess(image).unsqueeze(0).to(self.device)
        
        # Extract features
        with torch.no_grad():
            features = self.model.encode_image(img_tensor)
            # Normalize
            features = features / features.norm(dim=-1, keepdim=True)
        
        # Return as numpy array
        return features.cpu().numpy().flatten()
    
    def embed_glyph_set(self, images: List[Image.Image]) -> Dict:
        """
        Embed a set of glyph images and aggregate.
        
        Per spec: Standard glyphs are H O n a e g
        
        Args:
            images: List of glyph images
        
        Returns:
            Dict with:
                - individual_embeddings: List of per-glyph vectors
                - aggregated_embedding: Mean pooled vector
                - embedding_dim: Dimensionality
        """
        embeddings = []
        
        for img in images:
            emb = self.embed_image(img)
            embeddings.append(emb)
        
        embeddings = np.array(embeddings)
        
        # Aggregate via mean pooling
        aggregated = np.mean(embeddings, axis=0)
        
        # Normalize again
        aggregated = aggregated / np.linalg.norm(aggregated)
        
        return {
            'individual_embeddings': embeddings.tolist(),
            'aggregated_embedding': aggregated.tolist(),
            'embedding_dim': len(aggregated),
            'num_glyphs': len(images)
        }
    
    def embed_font(self, font_path: str, glyph_size: int = 256) -> Dict:
        """
        Complete pipeline: render glyphs and extract embeddings.
        
        Args:
            font_path: Path to font file
            glyph_size: Size for rendering glyphs
        
        Returns:
            Perceptual embedding dict
        """
        import sys
        sys.path.insert(0, 'src')
        from embeddings.glyph_renderer import GlyphRenderer
        
        # Render glyphs
        renderer = GlyphRenderer(font_path, size=128)
        glyph_images = renderer.render_standard_set(image_size=(glyph_size, glyph_size))
        
        # Extract embeddings
        embedding_data = self.embed_glyph_set(glyph_images)
        
        # Add metadata
        embedding_data['font_path'] = str(font_path)
        embedding_data['glyph_size'] = glyph_size
        embedding_data['model'] = self.model_name
        embedding_data['pretrained'] = self.pretrained
        
        return embedding_data


def test_embedder():
    """Test perceptual embedder."""
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python perceptual_embedder.py <font_file.ttf>")
        sys.exit(1)
    
    font_path = sys.argv[1]
    
    print(f"\n{'='*70}")
    print(f"Extracting perceptual embeddings from {Path(font_path).name}")
    print(f"{'='*70}\n")
    
    # Initialize embedder
    embedder = PerceptualEmbedder(model_name='ViT-B-32', pretrained='openai')
    
    # Extract embeddings
    print(f"\nRendering glyphs and extracting embeddings...")
    embedding_data = embedder.embed_font(font_path)
    
    print(f"\n✓ Embedding extraction complete!")
    print(f"  Embedding dimension: {embedding_data['embedding_dim']}")
    print(f"  Number of glyphs: {embedding_data['num_glyphs']}")
    print(f"  Model: {embedding_data['model']}")
    
    # Show sample values
    agg_emb = embedding_data['aggregated_embedding']
    print(f"\n  Aggregated embedding (first 10 dims):")
    print(f"  {agg_emb[:10]}")
    
    # Save
    import json
    output_path = f"data/embeddings/{Path(font_path).stem}_perceptual.json"
    Path(output_path).parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w') as f:
        json.dump(embedding_data, f, indent=2)
    
    print(f"\n✓ Saved to {output_path}")


if __name__ == '__main__':
    test_embedder()
