"""Sinh vector tham chiếu PyTorch (768 chiều, thô) cho ảnh vàng của fontatlas-embed.

Chạy offline:  python tools/embed/torch_ref.py --revision 75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2 \
                 UX/src-tauri/crates/fontatlas-embed/tests/data/render > .../tests/data/torch_ref.json
Ảnh: PNG xám 224×224 → (v/255−0.5)/0.5, R=G=B (đúng công thức to_tensor của Rust; không dùng processor của HF).
Model: google/siglip2-base-patch16-224 cùng revision với bản ONNX → get_image_features.
"""
import argparse, glob, json, os, sys
import numpy as np
import torch
import transformers
from PIL import Image
from transformers import AutoModel

ap = argparse.ArgumentParser()
ap.add_argument('--revision', required=True)
ap.add_argument('dir')
a = ap.parse_args()
m = AutoModel.from_pretrained('google/siglip2-base-patch16-224', revision=a.revision, attn_implementation='eager').eval()
out = {'_revision': a.revision, '_transformers': transformers.__version__, '_torch': torch.__version__}
for p in sorted(glob.glob(os.path.join(a.dir, '*.png'))):
    g = np.asarray(Image.open(p).convert('L'), np.float32)
    assert g.shape == (224, 224), p
    t = (g / 255.0 - 0.5) / 0.5
    x = torch.from_numpy(np.stack([t, t, t])[None].astype(np.float32))
    with torch.no_grad():
        o = m.get_image_features(pixel_values=x)
        o = o.pooler_output if hasattr(o, 'pooler_output') else o
    out[os.path.basename(p)[:-4]] = [float(v) for v in o[0].numpy()]
json.dump(out, sys.stdout)
