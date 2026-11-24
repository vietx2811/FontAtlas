(async () => {
  try {
    const t0 = now(); await scanSub(); await wait(() => window.__faLocale, 20000); window.__faLocale?.('en');
    const sideReady = await wait(() => { const si = sideInfo(); return si.all != null && si.srcs.includes('base') ? si : null; }, 60000);
    const tSide = now(); const gridReady = await wait(() => names() > 0, 60000); const tGrid = now();
    const dupsAt = await wait(() => sideInfo().dups != null, 30000); const tDup = now();
    await sleep(25000);
    const first = () => { const f = p => { const x = tl.find(p); return x ? x[0] - T0 : null; }; return { sidebarShell: f(x => true), sidebarAllCount: f(x => x[4] != null), sidebarSources: f(x => x[6] !== ''), duplicatesItem: f(x => x[5] != null), firstCards: f(x => x[1] > 0), scanningIndicator: f(x => x[7]), scanningGone: (() => { const i = tl.findIndex(x => x[7]); if (i < 0) return null; const j = tl.findIndex((x, k) => k > i && !x[7]); return j < 0 ? null : tl[j][0] - T0; })() }; };
    const si = sideInfo(); let ds = null; try { ds = JSON.stringify(await raw('duplicate_summary', {})).slice(0, 400); } catch (e) { ds = 'err ' + E(e).slice(0, 200); }
    await log('startup', true, { sinceLaunchMs: first(),  dupSummary: ds, final: { all: si.all, dups: si.dups, srcs: si.srcs }, scanEvents: evCount(0), ipc: ipcCount(0), emptyEpisodes: emptyEpisodes(0).map(x => [x[0] - T0, x[1]]), remounts, sideBlankSamples: sideBlank(0) });
    await logBig('startup-ipc', ipc.filter(x => REL.includes(x[1]) || /subscribe|boot|get_ui_state/.test(x[1])).map(x => [x[0] - T0, x[1], x[2]]));
    await logBig('startup-tl', tl.map(x => [x[0] - T0, x[1], x[2], x[3], x[4], x[5], x[6], x[7]]));
    await logBig('startup-scan', scanEv.map(x => [x[0] - T0, x[1], x[2], x[3]]));
  } catch (e) { await log('startup', false, E(e).slice(0, 500)); }
  await log('q8-done', true, null);
})();
