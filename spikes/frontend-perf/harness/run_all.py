# SPIKE — chạy ma trận bench: engine × mode × n × lần lặp; ghi harness/runs.jsonl
import json, os, subprocess, sys, time, tempfile, shutil, glob
BASE = 'http://localhost:5199/'
HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, '..', 'results.jsonl')
OUT = os.path.join(HERE, 'runs.jsonl')
CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
modes = sys.argv[1].split(',') if len(sys.argv) > 1 else ['a', 'b', 'b2', 'c', 'csvg']
ns = [int(x) for x in (sys.argv[2].split(',') if len(sys.argv) > 2 else ['2000', '5000'])]
engines = sys.argv[3].split(',') if len(sys.argv) > 3 else ['wkwebview', 'chromium']
reps = int(sys.argv[4]) if len(sys.argv) > 4 else 3

def lines():
    return open(RES).read().splitlines() if os.path.exists(RES) else []

def run_wk(url):
    p = subprocess.run([os.path.join(HERE, 'wk'), url], capture_output=True, text=True, timeout=300)
    out = p.stdout.strip().splitlines()
    r = json.loads(out[0]); mb = None
    for l in out:
        if 'phys_footprint:' in l: mb = float(l.split('phys_footprint:')[1].split()[0]) * (1024 if 'GB' in l else 1)
    r['proc_mb'] = mb
    return r

def run_chrome(url):
    d = tempfile.mkdtemp(prefix='crperf')
    n0 = len(lines())
    p = subprocess.Popen([CHROME, '--headless=new', f'--user-data-dir={d}', '--window-size=1280,800', '--enable-precise-memory-info',
                          '--no-first-run', '--disable-background-timer-throttling', url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    t0 = time.time(); r = None
    while time.time() - t0 < 300:
        ls = lines()
        if len(ls) > n0: r = json.loads(ls[-1]); break
        time.sleep(0.5)
    ps = subprocess.run(['ps', '-axo', 'rss=,command='], capture_output=True, text=True).stdout.splitlines()
    rss = [int(l.split()[0]) for l in ps if d in l and '--type=renderer' in l]
    p.kill(); p.wait(); shutil.rmtree(d, ignore_errors=True)
    if r: r['proc_mb'] = round(max(rss) / 1024, 1) if rss else None
    return r

for eng in engines:
    for n in ns:
        for m in modes:
            for k in range(reps):
                url = f'{BASE}?mode={m}&n={n}&auto=1&tag={eng}'
                try:
                    r = run_wk(url) if eng == 'wkwebview' else run_chrome(url)
                except Exception as e:
                    r = {'tag': eng, 'mode': m, 'n': n, 'error': repr(e)[:200]}
                r['rep'] = k
                print(json.dumps({x: r.get(x) for x in ['tag', 'mode', 'n', 'rep', 'first_ms', 'fps', 'frame_p95', 'blank_pct', 'slow_blank_pct', 'text_change_ms', 'proc_mb', 'error']}), flush=True)
                open(OUT, 'a').write(json.dumps(r) + '\n')
