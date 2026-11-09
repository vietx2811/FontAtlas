// Mỗi trục một icon riêng (24×24, nét 1.6). Không dùng ●◐◆○: bộ ký hiệu đó dành cho quan hệ ghép cặp.
export const AXIS_PATHS: Record<string, React.ReactNode> = {
  x_height_em: <><path d="M4 20h16" /><path d="M4 12h16" strokeDasharray="2 2" /><path d="M12 12v8M9 15l3-3 3 3" /></>,
  cap_height_em: <><path d="M4 20h16M4 5h16" strokeDasharray="2 2" /><path d="M9 20V6l3 6 3-6v14" /></>,
  ascender_em: <><path d="M4 20h16M4 4h16" strokeDasharray="2 2" /><path d="M10 20V4M10 12c4 0 6 2 6 4s-2 4-6 4" /></>,
  descender_em: <><path d="M4 8h16M4 21h16" strokeDasharray="2 2" /><path d="M14 4v17M14 9c-4 0-6 2-6 4s2 4 6 4" /></>,
  stem: <><rect x="8" y="4" width="8" height="16" rx="1" /><path d="M8 12h8" /></>,
  contrast: <><path d="M6 20c2-3 3-9 3-16M16 20c0-4 1-8 3-12" strokeWidth="2.6" /><path d="M6 20c2-3 3-9 3-16M16 20c0-4 1-8 3-12" /></>,
  contrast_h: <><path d="M6 4v16M18 4v16" strokeWidth="2.6" /><path d="M6 12h12" /></>,
  stress_deg: <><ellipse cx="12" cy="12" rx="5" ry="8" transform="rotate(25 12 12)" /><path d="M5 18L19 6" strokeDasharray="2 2" /></>,
  slant_deg: <><path d="M9 20L15 4" strokeWidth="2.4" /><path d="M5 20h14" /></>,
  width: <><path d="M4 12h16M4 9v6M20 9v6" /><path d="M8 12l2-2m-2 2l2 2M16 12l-2-2m2 2l-2 2" /></>,
  aperture: <><path d="M17 8a6 6 0 1 0 0 8" /><path d="M17 8v0M17 16v0" strokeWidth="2.4" /></>,
  counter: <><rect x="5" y="4" width="14" height="16" rx="6" /><rect x="9" y="9" width="6" height="6" rx="3" /></>,
  spacing: <><path d="M4 6v12M20 6v12" /><path d="M8 12h8M8 12l2-2M8 12l2 2M16 12l-2-2M16 12l-2 2" /></>,
  serif_foot: <><path d="M12 4v13M7 20h10M9 17h6" /></>,
  mean_advance: <><path d="M6 4v16M6 11h11" /><path d="M17 11v-3" /></>,
  width_cv: <><path d="M7 5v7M7 12c3 0 5 1 5 3s-2 5-6 5M12 15h6" /></>,
  darkness: <><rect x="4" y="4" width="16" height="16" rx="8" /><path d="M4 12h16M12 4v16" strokeDasharray="2 2" /></>,
  x_height_cap: <><path d="M4 20h16M4 5h16M4 12h16" strokeDasharray="2 2" /><path d="M10 5v15m5-8v8" /></>,
  aperture_e: <><path d="M18 8a6 6 0 1 0 0 8M6 12h12m0 0v-3" /></>,
  terminal: <><path d="M5 18c4-1 7-5 8-12" /><path d="M13 6l4 0" strokeWidth="2.6" /></>,
  a_storeys: <><path d="M8 9c1-2 7-2 7 2v8M15 14c-4-1-7 0-7 3s5 3 7 0" /></>,
  g_storeys: <><circle cx="11" cy="9" r="4" /><path d="M15 9v7c0 3-3 4-6 3" /></>,
};

export function AxisIcon({ name }: { name: string }): React.JSX.Element {
  return (
    <svg data-icon={name} viewBox="0 0 24 24" width="16" height="16" aria-hidden fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      {AXIS_PATHS[name] ?? <path d="M5 12h14" />}
    </svg>
  );
}
