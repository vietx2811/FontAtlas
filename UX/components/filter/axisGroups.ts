/** 16 trục đo + 2 trục class, chia nhóm cho Advanced. Thứ tự cố định; mỗi trục đúng một nhóm. */
export type AxisGroupDef = { id: string; title: string; axes: string[] };
export const AXIS_GROUPS: AxisGroupDef[] = [
  { id: 'weightwidth', title: 'Weight & Width', axes: ['weight_class', 'width_class'] },
  { id: 'proportions', title: 'Proportions', axes: ['x_height_em', 'x_height_cap', 'ascender_em', 'width', 'mean_advance', 'width_cv'] },
  { id: 'contrast', title: 'Contrast & Stress', axes: ['contrast', 'stress_deg', 'stem', 'darkness'] },
  { id: 'aperture', title: 'Aperture', axes: ['aperture', 'aperture_e', 'counter'] },
  { id: 'spacing', title: 'Spacing', axes: ['spacing'] },
  { id: 'serifslant', title: 'Serif & Slant', axes: ['serif_foot', 'slant_deg'] },
];
export const ALL_AXES = AXIS_GROUPS.flatMap(g => g.axes);
