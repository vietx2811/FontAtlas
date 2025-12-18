import { FontCategory, FontData, FontPairing } from './types';

export const MOCK_FONTS: FontData[] = [
  {
    id: '1',
    name: 'Inter Regular',
    family: 'Inter',
    category: FontCategory.SansSerif,
    metrics: {
      xHeightRatio: 0.72,
      widthVariance: 0.12,
      strokeContrast: 1.1,
      aperture: 0.65,
      capHeight: 720,
      unitsPerEm: 2048
    },
    description: "A highly legible sans-serif designed for computer screens with a tall x-height.",
    tags: ['Ui', 'Modern', 'Neutral'],
    fileSize: "245 KB",
    dateAnalyzed: "2023-10-15"
  },
  {
    id: '2',
    name: 'Playfair Display',
    family: 'Playfair Display',
    category: FontCategory.Serif,
    metrics: {
      xHeightRatio: 0.68,
      widthVariance: 0.35,
      strokeContrast: 4.5,
      aperture: 0.45,
      capHeight: 700,
      unitsPerEm: 1000
    },
    description: "A transitional design. From the time of enlightenment in the late 18th century.",
    tags: ['Editorial', 'High Contrast', 'Classic'],
    fileSize: "180 KB",
    dateAnalyzed: "2023-10-12"
  },
  {
    id: '3',
    name: 'Roboto Mono',
    family: 'Roboto Mono',
    category: FontCategory.Monospace,
    metrics: {
      xHeightRatio: 0.69,
      widthVariance: 0.0,
      strokeContrast: 1.0,
      aperture: 0.60,
      capHeight: 710,
      unitsPerEm: 2048
    },
    description: "A monospaced addition to the Roboto type family. Optimized for reading code.",
    tags: ['Code', 'Technical', 'Google'],
    fileSize: "150 KB",
    dateAnalyzed: "2023-09-28"
  },
  {
    id: '4',
    name: 'Abril Fatface',
    family: 'Abril Fatface',
    category: FontCategory.Display,
    metrics: {
      xHeightRatio: 0.55,
      widthVariance: 0.40,
      strokeContrast: 8.2,
      aperture: 0.30,
      capHeight: 680,
      unitsPerEm: 1000
    },
    description: "Inspired by the heavy titling posters of 19th century Britain and France.",
    tags: ['Bold', 'Poster', 'Heavy'],
    fileSize: "210 KB",
    dateAnalyzed: "2023-11-01"
  },
  {
    id: '5',
    name: 'Lato Light',
    family: 'Lato',
    category: FontCategory.SansSerif,
    metrics: {
      xHeightRatio: 0.65,
      widthVariance: 0.18,
      strokeContrast: 1.2,
      aperture: 0.55,
      capHeight: 700,
      unitsPerEm: 2000
    },
    description: "Semi-rounded details give Lato a feeling of warmth, while the strong structure provides stability.",
    tags: ['Humanist', 'Friendly', 'Web'],
    fileSize: "195 KB",
    dateAnalyzed: "2023-10-20"
  },
  {
    id: '6',
    name: 'Merriweather',
    family: 'Merriweather',
    category: FontCategory.Serif,
    metrics: {
      xHeightRatio: 0.74,
      widthVariance: 0.22,
      strokeContrast: 2.1,
      aperture: 0.50,
      capHeight: 730,
      unitsPerEm: 2048
    },
    description: "Designed to be a text face that is pleasant to read on screens.",
    tags: ['Reading', 'Screen', 'Sturdy'],
    fileSize: "220 KB",
    dateAnalyzed: "2023-10-22"
  },
];

export const MOCK_PAIRINGS: Record<string, FontPairing[]> = {
  '1': [ // Inter pairings
    {
      targetFontId: '2', // Playfair
      score: 0.92,
      rationale: "Excellent structural contrast. The high stroke contrast of Playfair balances the neutral uniformity of Inter.",
      metrics_harmony: { xHeight: 0.85, contrast: 0.95, width: 0.80 }
    },
    {
      targetFontId: '4', // Abril
      score: 0.85,
      rationale: "Strong display hierarchy. Use Abril for impact headlines and Inter for dense body copy.",
      metrics_harmony: { xHeight: 0.70, contrast: 0.98, width: 0.75 }
    }
  ],
  '2': [ // Playfair pairings
    {
      targetFontId: '1',
      score: 0.92,
      rationale: "Classic Editorial look. The neutral sans supports the expressive serif header perfectly.",
      metrics_harmony: { xHeight: 0.85, contrast: 0.95, width: 0.80 }
    },
    {
      targetFontId: '5',
      score: 0.88,
      rationale: "Humanist balance. Lato's warmth complements the sharp serifs of Playfair.",
      metrics_harmony: { xHeight: 0.90, contrast: 0.85, width: 0.82 }
    }
  ]
};

export const SAMPLE_TEXT = "The quick brown fox jumps over the lazy dog.";
export const ALPHABET = "Aa Bb Cc Dd Ee Ff Gg Hh Ii Jj Kk Ll Mm Nn Oo Pp Qq Rr Ss Tt Uu Vv Ww Xx Yy Zz 0123456789";
