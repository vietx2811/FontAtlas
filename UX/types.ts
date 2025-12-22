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
  // Additional metrics for classification
  curvature: number;
  terminalSharpness: number;
  stressAngle: number;
  counterDensity: number;
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
  // Enhanced taxonomy (Phase 4)
  primary_class?: string | null;
  secondary_tags?: string | null;  // JSON string array
  serif_subtype?: string | null;
  sans_subtype?: string | null;
  script_subtype?: string | null;
  // Metadata
  foundry?: string | null;
  isFavorite: boolean;
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

// Font Family for grouping
export interface FontFamily {
  family_name: string;
  variants: Array<{
    id: string;
    name: string;
    subfamily: string;
    weight_class: number;
    width_class: number;
    file_path: string;
    is_italic: boolean;
  }>;
  representative_id: string; // Default variant to show
}
