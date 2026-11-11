import type { CLOUD_I18N } from '../cloud';

export const EN_CLOUD: Record<keyof typeof CLOUD_I18N, string> = {
  'cloud.loading': 'In the cloud — downloading for preview…',
  'cloud.errTimeout': 'Waited 30 seconds for the font to download from the cloud, so there’s no preview. Check your connection or the cloud drive app, then scroll this card back into view to retry.',
  'cloud.errFailed': 'Couldn’t download the font from the cloud, so there’s no preview. Check your connection or the cloud drive app.',
  'cloud.errMissing': 'The font file wasn’t found (the drive isn’t connected, or the file was deleted or moved), so there’s no preview.',
  'cloud.errBusy': 'The cloud drive is busy downloading other fonts and hasn’t reached this one yet. Scroll this card back into view to retry.',
  'cloud.errOff': 'The font isn’t downloaded, so there’s no preview. Turn on “Download cloud fonts for preview” in Settings → Library.',
  'cloud.sourceHint': '{pct}% of this source’s fonts aren’t downloaded — choose “Always keep on this device” in Finder for faster previews.',
  'cloud.switch': 'Download cloud fonts for preview',
  'cloud.switch.note': 'On: for cards on screen whose font is still in the cloud, the font is downloaded to draw the preview (up to 4 at once, 30 seconds each). Off: nothing is downloaded, and those cards have no preview until you download the font.',
  'cloud.note': 'In the cloud — not downloaded yet. The font downloads when you activate it, up to 30 seconds.',
  'act.msg.cloudTimeout': 'Waited 30 seconds for the font to download from the cloud drive. Check your connection or the cloud drive app and try again.',
  'act.msg.cloudFailed': 'Couldn’t download the font from the cloud drive: {message}',
  'act.msg.cloudStub': 'The font is only an iCloud placeholder with no file to download yet. Open the folder in Finder so iCloud downloads it, then try again.',
  'term.dataless.gloss': 'Only a placeholder is on this Mac: the content lives in the cloud (OneDrive, iCloud…) and downloads when an app opens it.',
  'term.always_keep.gloss': 'A Finder option (right-click the folder → Always keep on this device) that keeps the files on this Mac so their space is never freed.',
};
