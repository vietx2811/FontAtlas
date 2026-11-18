(async () => {
  try {
    await scanSub(); await wait(() => window.__faLocale, 20000); window.__faLocale?.('en');
    await wait(() => { const si = sideInfo(); return si.all != null && si.srcs.includes('cloud') ? si : null; }, 60000);
    await wait(() => names() > 0, 30000); await scanIdle(120000); await sleep(8000);
    const R = n => Math.round(n);
    const cloudFaces = async () => { try { const s = (await raw('list_sources', {})).find(x => x.name === 'cloud'); return s ? s.faceCount : null; } catch (e) { return 'err'; } };
    const phase = async (name, obsMs, marker) => {
      const a = now(); const f0 = await cloudFaces(); if (marker) await go(marker); await sleep(obsMs); await scanIdle(60000); await sleep(3000); const b = now();
      const sc = scanEv.filter(x => x[0] >= a && x[0] < b); const si = sideInfo();
      await log('phase-' + name, true, { windowMs: [a - T0, b - T0], cloudFacesBefore: f0, cloudFacesAfter: await cloudFaces(), scanEvents: evCount(a, b), ipc: ipcCount(a, b), latencyMs: lat(a, b),
        scansQueued: sc.filter(x => x[1] === 'queued').length, scansFinished: sc.filter(x => x[1] === 'finished').length,
        emptyEpisodes: emptyEpisodes(a, b).map(x => [x[0] - T0, x[1]]), sideBlankSamples: sideBlank(a, b), remountsTotal: remounts, sidebarNow: { all: si.all, dups: si.dups, srcs: si.srcs },
        scanTimeline: sc.filter(x => x[1] !== 'progress').slice(0, 70).map(x => [R(x[0] - T0), x[1], x[2], x[3]]),
        queryFacesTimes: ipc.filter(x => x[0] >= a && x[0] < b && x[1] === 'query_faces').map(x => R(x[0] - T0)).slice(0, 80) });
    };
    const ONLY = window.__Q8_ONLY || '';
    if (!ONLY || ONLY.includes('base')) await phase('0-baseline-quiet', 45000, null);
    if (!ONLY || ONLY.includes('A')) await phase('A-1file', 45000, 'sc-A');
    if (!ONLY || ONLY.includes('B')) await phase('B-30files-1s', 31000 + 30000, 'sc-B');
    if (!ONLY || ONLY.includes('C')) await phase('C-30files-4s', 121000 + 30000, 'sc-C');
    if (!ONLY || ONLY.includes('D')) await phase('D-10rewrites-4s-no-new-font', 41000 + 20000, 'sc-D');
    await log('blank-samples', true, blank);
    await logBig('timeline', tl.map(x => [x[0] - T0, x[1], x[2], x[3], x[4], x[5], x[6], x[7]]).slice(-150));
  } catch (e) { await log('scen', false, E(e).slice(0, 500)); }
  await log('q8-done', true, null);
})();
