/** English for the library view options (sort, group, tags, remove source, activate all). Keys mirror i18n/library.ts. */
import type { LIBRARY_I18N } from '../library';

export const LIBRARY_EN: Record<keyof typeof LIBRARY_I18N, string> = {
  'view.sort.released': 'Release year (estimated)',
  'view.sort.file_created': 'File created',
  'view.sort.released.asc': 'Old → New',
  'view.sort.released.desc': 'New → Old',
  'view.sort.file_created.asc': 'Old → New',
  'view.sort.file_created.desc': 'New → Old',
  'view.sort.src.released': 'Estimated from the copyright line; if missing, from the created year (head.created), then the modified year (head.modified). Fonts with no information go last.',
  'view.sort.src.file_created': 'When the file appeared on disk (birth time). Fonts whose date cannot be read go last.',
  'view.sort.note.released': 'Medium reliability: the copyright line is usually the first publication year; some foundries set the created year at build time, or leave 0/1904.',
  'view.sort.note.file_created': 'Low reliability as a “font release date”: copying, re-downloading or cloud sync reset the date. For files on a cloud drive this may be the sync time, not when the font was made.',
};
