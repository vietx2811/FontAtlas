# -*- mode: python ; coding: utf-8 -*-
"""
PyInstaller spec file for FontAtlas Python Sidecar
Creates standalone executable with all dependencies
"""

block_cipher = None

# Hidden imports that PyInstaller might miss
hidden_imports = [
    'fontTools.ttLib',
    'fontTools.pens',
    'numpy',
    'sklearn',
    'umap',
    'qdrant_client',
]

# Data files to include
datas = [
    # Add any model files or data files here
    # ('path/to/models', 'models'),
]

a = Analysis(
    ['sidecar.py'],
    pathex=[],
    binaries=[],
    datas=datas,
    hiddenimports=hidden_imports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'matplotlib',
        'pandas',
        'scipy',
        'PIL',
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='fontatlas-parser',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,  # Keep console for stdin/stdout
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
