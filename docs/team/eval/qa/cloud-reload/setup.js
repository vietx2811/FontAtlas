(async () => {
  try {
    await wait(() => window.__faLocale, 20000); window.__faLocale?.('en'); await skipOnboarding();
    await log('onb', !document.querySelector('.fa-onb'), null);
    if (window.__Q8_ONLY === 'cloudonly') { await raw('add_source', { path: '__Q__/small100' }); await sleep(1000); await scanIdle(120000); } else { await raw('add_source', { path: '__Q__/base' }); await sleep(1000); await scanIdle(240000); }
    await raw('add_source', { path: '__Q__/cloud' }); await sleep(1000); await scanIdle(240000); await sleep(4000);
    const srcs = (await raw('list_sources', {})).map(s => ({ n: s.name, faces: s.faceCount, kind: s.kind }));
    const si = sideInfo(); await log('setup', true, { srcs, all: si.all, dups: si.dups, side: si.txt.slice(0, 500) });
  } catch (e) { await log('setup', false, E(e).slice(0, 500)); }
  await log('q8-done', true, null);
})();
