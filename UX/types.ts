export enum FontCategory {
  Serif = 'Serif',
  SansSerif = 'Sans Serif',
  Display = 'Display',
  Monospace = 'Monospace',
  Script = 'Script',
  Decorative = 'Decorative'
}

export interface FontMetrics {
  xHeightRatio: number;
  widthVariance: number;
  strokeContrast: number;
  aperture: number;
  capHeight: number;
  unitsPerEm: number;
}

export interface FontData {
  id: string;
  name: string;
  family: string;
  category: FontCategory;
  metrics: FontMetrics;
  filePath?: string;  // Path to font file for preview
  pairingScore?: number;
  previewText?: string;
  description: string;
  tags: string[];
  fileSize: string;
  dateAnalyzed: string;
}

export interface FontPairing {
  targetFontId: string;
  score: number;
  rationale: string;
  font?: FontData;
  metrics_harmony: {
    xHeight: number;
    contrast: number;
    width: number;
  };
}
