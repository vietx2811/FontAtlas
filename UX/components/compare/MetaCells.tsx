import type { CompareFace } from '../../api/compare';
import { hasKey, t, type I18nKey } from '../../i18n';

const tr = (key: string, fallback: string): string => (hasKey(key) ? t(key) : fallback);
const range = (r: [number, number] | null | undefined): string | null => (r ? (r[0] === r[1] ? `${r[0]}` : `${r[0]}–${r[1]}`) : null);
const nf = (n: number) => new Intl.NumberFormat('vi-VN').format(n);
const Empty = (): React.JSX.Element => <span className="cmp-meta-na" title={t('compare.noExtras')}>—</span>;
/** Font chưa phân tích lại theo analyzer mới: backend báo `extrasAvailable: false` (thiếu trường = có dữ liệu). */
const stale = (face: CompareFace): boolean => (face.extras as { extrasAvailable?: boolean } | null)?.extrasAvailable === false;
const Rescan = (): React.JSX.Element => <span className="cmp-meta-na" title={t('compare.noExtras')}>— <small>{t('compare.rescan')}</small></span>;
const Chip = ({ children, title }: { children: React.ReactNode; title?: string }) => <span className="cmp-tag" title={title}>{children}</span>;

export const licenseLabel = (kind: string | null | undefined): string => (kind ? tr(`lic.${kind}`, kind) : '—');

export function LicenseCell({ face }: { face: CompareFace }): React.JSX.Element {
  const n = face.licenseNote;
  return (
    <div className="cmp-lic">
      <b title={n?.description ?? undefined}>{n ? licenseLabel(n.kind) : face.license ?? '—'}</b>
      <span>{t('compare.licenseNote')}</span>
    </div>
  );
}

export function VendorCell({ face }: { face: CompareFace }): React.JSX.Element {
  if (stale(face)) return <Rescan />;
  const v = face.extras?.vendorId;
  return v ? <b className="cmp-mono">{v}</b> : <Empty />;
}

export function YearsCell({ face }: { face: CompareFace }): React.JSX.Element {
  if (stale(face)) return <Rescan />;
  const y = face.extras?.years;
  if (!y || (!y.created && !y.modified && !y.copyright)) return <Empty />;
  return (
    <div className="cmp-meta-list">
      {y.created != null && <span>{t('meta.created', { y: y.created })}</span>}
      {y.modified != null && y.modified !== y.created && <span>{t('meta.modified', { y: y.modified })}</span>}
      {y.copyright != null && <span>{t('meta.copyright', { y: y.copyright })}</span>}
    </div>
  );
}

export function StyleCell({ face }: { face: CompareFace }): React.JSX.Element {
  if (stale(face)) return <Rescan />;
  const s = face.extras?.style;
  if (!s) return <Empty />;
  const kind = s.italicKind;
  const axes: Array<[string, string | null]> = [['wght', range(s.wghtRange)], ['wdth', range(s.wdthRange)], ['opsz', range(s.opszRange)]];
  return (
    <div className="cmp-meta-list">
      <b title={tr(`meta.${kind}.gloss`, '')}>{tr(`meta.${kind}`, kind)}</b>
      {s.italAxis && <Chip>{t('meta.axis.ital')}</Chip>}{s.slntAxis && <Chip>{t('meta.axis.slnt')}</Chip>}
      {axes.map(([k, r]) => r && <span key={k}>{tr(`meta.axis.${k}`, k)} {r}</span>)}
    </div>
  );
}

export function FamilyCell({ face }: { face: CompareFace }): React.JSX.Element {
  const f = face.family;
  if (!f) return <Empty />;
  const w = f.weights.length ? range([Math.min(...f.weights), Math.max(...f.weights)]) : null;
  return (
    <div className="cmp-meta-list">
      <span>{t('meta.faces', { n: f.faces })}{w && ` · ${t('meta.weights', { range: w })}`}</span>
      <span className="cmp-tags">
        {f.hasItalic && <Chip>{t('meta.hasItalic')}</Chip>}{f.hasOblique && <Chip>{t('meta.hasOblique')}</Chip>}
        {f.hasVariable && <Chip>{t('meta.hasVariable')}</Chip>}{f.hasOpsz && <Chip>{t('meta.hasOpsz')}</Chip>}
      </span>
    </div>
  );
}

export function OtCell({ face }: { face: CompareFace }): React.JSX.Element {
  if (stale(face)) return <Rescan />;
  const g = face.extras?.otGroups;
  if (!face.extras) return <Empty />;
  if (!g?.length) return <span className="cmp-meta-na">—</span>;
  return (
    <span className="cmp-tags">
      {g.map(x => {
        const names = x.sets.length ? x.sets.map(s => `${s.tag}${s.name ? ` ${s.name}` : ''}`).join(', ') : x.tags.join(', ');
        const gloss = tr(`ot.${x.group}.gloss`, '');
        return <Chip key={x.group} title={`${gloss ? gloss + ' — ' : ''}${names}`}>{tr(`ot.${x.group}`, x.group)}{x.group === 'stylistic_sets' ? ` ${x.tags.length}` : ''}</Chip>;
      })}
    </span>
  );
}

export function KerningCell({ face }: { face: CompareFace }): React.JSX.Element {
  if (stale(face)) return <Rescan />;
  const k = face.extras?.kerning;
  if (!k) return <Empty />;
  if (k.source === 'none' || (k.pairs === 0 && k.classPairs === 0)) return <span className="cmp-meta-na">{t('meta.kernNone')}</span>;
  return <span className="cmp-meta-list" title={k.source.toUpperCase()}>{t('meta.kernPairs', { pairs: nf(k.pairs), classes: nf(k.classPairs) })}</span>;
}

export function EmbeddingCell({ face }: { face: CompareFace }): React.JSX.Element {
  if (stale(face) && !face.licenseNote) return <Rescan />;
  const e = face.licenseNote?.embedding ?? face.extras?.embedding;
  if (!e) return <Empty />;
  return (
    <div className="cmp-meta-list">
      <b title={tr(`emb.${e.level}.gloss`, '')}>{tr(`emb.${e.level}`, e.level)}</b>
      {e.flags.map(f => <span key={f}>{tr(`emb.${f}`, f)}</span>)}
    </div>
  );
}

export const META_ROWS: Array<{ key: string; label: I18nKey; gloss: I18nKey; Cell(p: { face: CompareFace }): React.JSX.Element }> = [
  { key: 'vendor', label: 'meta.vendor', gloss: 'meta.vendor.gloss', Cell: VendorCell },
  { key: 'years', label: 'meta.years', gloss: 'meta.years.gloss', Cell: YearsCell },
  { key: 'style', label: 'meta.style', gloss: 'meta.style.gloss', Cell: StyleCell },
  { key: 'family', label: 'meta.family', gloss: 'meta.family.gloss', Cell: FamilyCell },
  { key: 'ot', label: 'meta.ot', gloss: 'meta.ot.gloss', Cell: OtCell },
  { key: 'kerning', label: 'meta.kerning', gloss: 'meta.kerning.gloss', Cell: KerningCell },
  { key: 'embedding', label: 'meta.embedding', gloss: 'meta.embedding.gloss', Cell: EmbeddingCell },
];
