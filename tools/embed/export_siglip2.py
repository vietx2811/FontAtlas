"""Export SigLIP 2 base (google/siglip2-base-patch16-224, Apache-2.0) → ONNX lô tĩnh cho fontatlas-embed.

Chạy offline (cần mạng để tải trọng số từ Hugging Face):
  uv venv /tmp/fa-embed && VIRTUAL_ENV=/tmp/fa-embed uv pip install torch transformers onnx onnxruntime huggingface_hub
  /tmp/fa-embed/bin/python tools/embed/export_siglip2.py --revision 75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2 \
      --batch 4 --out ~/fontatlas-ref/siglip2_base_p16_224_75de2d5_b4.onnx
Đầu vào pixel_values [B,3,224,224] (chuẩn hoá (v/255-0.5)/0.5), đầu ra image_embeds [B,768] = get_image_features (pooler MAP).
Không đổi trọng số; chỉ chuyển định dạng. Tự kiểm ONNX ↔ torch (cos ≥ 0.9999) trên 8 ảnh ngẫu nhiên.
"""
import argparse, hashlib, os
import numpy as np
import torch
import transformers
from transformers import AutoModel

MODEL_ID = 'google/siglip2-base-patch16-224'

ap = argparse.ArgumentParser()
ap.add_argument('--revision', required=True)
ap.add_argument('--batch', type=int, default=4)
ap.add_argument('--out', required=True)
a = ap.parse_args()
out = os.path.expanduser(a.out)

m = AutoModel.from_pretrained(MODEL_ID, revision=a.revision, attn_implementation='eager').eval()


class Vision(torch.nn.Module):
    def __init__(self, m):
        super().__init__()
        self.m = m

    def forward(self, x):
        o = self.m.get_image_features(pixel_values=x)
        return o.pooler_output if hasattr(o, 'pooler_output') else o


v = Vision(m)
x = torch.randn(a.batch, 3, 224, 224)
torch.onnx.export(v, (x,), out, input_names=['pixel_values'], output_names=['image_embeds'], opset_version=17, dynamo=False)
print('torch', torch.__version__, 'transformers', transformers.__version__, 'revision', a.revision)

import onnxruntime as ort
s = ort.InferenceSession(out, providers=['CPUExecutionProvider'])
rng = np.random.default_rng(0)
cs = []
for _ in range(2):
    xb = rng.uniform(-1, 1, (a.batch, 3, 224, 224)).astype(np.float32)
    o = s.run(None, {'pixel_values': xb})[0]
    with torch.no_grad():
        t = v(torch.from_numpy(xb)).numpy()
    cs += list((o * t).sum(1) / (np.linalg.norm(o, axis=1) * np.linalg.norm(t, axis=1)))
print('onnx↔torch cos min', min(cs))
assert min(cs) >= 0.9999, cs
h = hashlib.sha256(open(out, 'rb').read()).hexdigest()
print('file', out, 'size', os.path.getsize(out), 'sha256', h)
