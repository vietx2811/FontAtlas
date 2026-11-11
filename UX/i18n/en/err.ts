import type { ERR_I18N } from '../err';

export const EN_ERR: Record<keyof typeof ERR_I18N, string> = {
  'err.internal': 'Something went wrong inside Font Atlas.',
  'err.invalid_arg': 'That request wasn’t valid.',
  'err.not_found': 'Couldn’t find that item.',
  'err.busy': 'Font Atlas is busy. Try again in a moment.',
  'err.io': 'Couldn’t read or write a file.',
  'err.exists': 'A file with that name already exists; choose another name.',
  'err.sqlite': 'The library database reported an error.',
  'err.newer_schema': 'This data was created by a newer version of Font Atlas.',
  'err.migration': 'Couldn’t upgrade the library data.',
  'err.bad_hash': 'The font identifier isn’t valid.',
  'err.legacy': 'Couldn’t read the old data.',
  'err.nested_source': 'This folder overlaps an existing source.',
  'err.permission_denied': 'macOS didn’t allow access to this folder.',
  'err.timeout': 'The folder didn’t open within a few seconds (it may be waiting for permission).',
  'err.model_missing': 'The model isn’t available yet.',
  'err.insufficient_disk': 'Not enough free space to download the model.',
  'err.cloud_busy': 'The cloud drive is busy downloading other fonts. Try again later.',
  'err.generic': 'Something went wrong ({code}).',
};
