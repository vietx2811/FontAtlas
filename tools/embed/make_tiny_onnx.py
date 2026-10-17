"""Tạo model ONNX giả cho test của fontatlas-embed (chạy offline một lần, kết quả đã commit).

Chạy: uv run --with onnx==1.17.0 tools/embed/make_tiny_onnx.py
  tiny_encoder.onnx   : pixel_values[4,3,224,224] → ReduceMean(H,W) → [4,3] → MatMul W[3,768] → image_embeds[4,768]
                        W[c][j] = (j%3==c) ? (j+1)/768 : 0
  tiny_bad_shape.onnx : giống trên nhưng output [4,512] (để test kiểm shape)
Ghim opset 17, ir_version 8.
"""
import os
import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

OUT = os.path.join(os.path.dirname(__file__), '..', '..', 'UX', 'src-tauri', 'crates', 'fontatlas-embed', 'tests', 'data')


def build(dim: int, name: str) -> None:
    w = np.zeros((3, dim), np.float32)
    for j in range(dim):
        w[j % 3, j] = (j + 1) / dim
    x = helper.make_tensor_value_info('pixel_values', TensorProto.FLOAT, [4, 3, 224, 224])
    y = helper.make_tensor_value_info('image_embeds', TensorProto.FLOAT, [4, dim])
    wt = numpy_helper.from_array(w, 'W')
    nodes = [
        helper.make_node('ReduceMean', ['pixel_values'], ['m'], axes=[2, 3], keepdims=0),  # opset 17: axes là attribute
        helper.make_node('MatMul', ['m', 'W'], ['image_embeds']),
    ]
    g = helper.make_graph(nodes, 'tiny', [x], [y], initializer=[wt])
    m = helper.make_model(g, opset_imports=[helper.make_opsetid('', 17)], producer_name='fontatlas-tiny')
    m.ir_version = 8
    onnx.checker.check_model(m)
    os.makedirs(OUT, exist_ok=True)
    onnx.save(m, os.path.join(OUT, name))
    print(name, os.path.getsize(os.path.join(OUT, name)), 'bytes')


build(768, 'tiny_encoder.onnx')
build(512, 'tiny_bad_shape.onnx')
