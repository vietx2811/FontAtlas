"""Sinh model SigLIP vision tí hon (ngẫu nhiên, seed 0) + vector tham chiếu torch cho test candle.

python tools/embed/make_tiny_siglip.py UX/src-tauri/crates/fontatlas-embed/tests/data
→ tiny_siglip.safetensors (khoá `vision_model.*`, có head MAP) và tiny_siglip_ref.json.
Đầu vào xác định, Rust tạo lại được: x0[i] = (i mod 251)/125 − 1, x1[i] = ((31·i) mod 241)/120 − 1, i ∈ [0, 3·224·224).
Config phải khớp `tiny_vision_config()` trong tests/encoder.rs.
"""
import json, os, sys
import numpy as np
import torch, transformers
from safetensors.torch import save_file
from transformers import SiglipVisionConfig, SiglipVisionModel

out = sys.argv[1]
torch.manual_seed(0)
cfg = SiglipVisionConfig(hidden_size=32, intermediate_size=64, num_hidden_layers=2, num_attention_heads=4,
                         image_size=224, patch_size=16, num_channels=3, hidden_act='gelu_pytorch_tanh',
                         layer_norm_eps=1e-6, vision_use_head=True)
m = SiglipVisionModel(cfg).eval()
for p in m.parameters():  # init mặc định quá nhỏ ở vài tensor → kéo về N(0, 0.2) cho tín hiệu rõ
    torch.nn.init.normal_(p, 0.0, 0.2)
sd = {k if k.startswith('vision_model.') else 'vision_model.' + k: v.contiguous() for k, v in m.state_dict().items()}
assert all(k.startswith('vision_model.') for k in sd), list(sd)[:3]
save_file(sd, os.path.join(out, 'tiny_siglip.safetensors'), metadata={'format': 'pt'})
i = np.arange(3 * 224 * 224, dtype=np.int64)
xs = [(i % 251) / 125.0 - 1.0, ((31 * i) % 241) / 120.0 - 1.0]
x = torch.from_numpy(np.stack(xs).astype(np.float32).reshape(2, 3, 224, 224))
with torch.no_grad():
    y = m(pixel_values=x).pooler_output
ref = {'_torch': torch.__version__, '_transformers': transformers.__version__, 'x0': y[0].tolist(), 'x1': y[1].tolist()}
json.dump(ref, open(os.path.join(out, 'tiny_siglip_ref.json'), 'w'))
print(y.shape, os.path.getsize(os.path.join(out, 'tiny_siglip.safetensors')))
