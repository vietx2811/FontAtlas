// Vite riêng cho harness S2: giả lập backend preview bằng ảnh do dump_previews dựng sẵn.
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import fs from 'node:fs';
import path from 'node:path';
import { execFile } from 'node:child_process';

const HERE = __dirname;
const UX = path.resolve(HERE, '../../..');
const PREVIEWS = path.join(HERE, '.data/previews');
const BIN = path.join(UX, 'src-tauri/target/release/examples/dump_previews');
const DEFAULT_SAMPLE = 'Hamburgefonts Ăn phở';

type Entry = { id: number; path: string; faceIndex: number; family: string; hashHex: string };

export default defineConfig({
  root: HERE,
  resolve: { alias: { '@': UX } },
  plugins: [react(), {
    name: 's2-backend',
    configureServer(server) {
      const manifest: Entry[] = JSON.parse(fs.readFileSync(path.join(PREVIEWS, 'manifest.json'), 'utf8'));
      server.middlewares.use((req, res, next) => {
        const u = new URL(req.url ?? '/', 'http://x');
        if (u.pathname === '/api/manifest') {
          res.setHeader('content-type', 'application/json');
          return res.end(JSON.stringify(manifest.map(e => ({ family: e.family }))));
        }
        const m = u.pathname.match(/^\/preview\/([0-9a-f]{64})_(\d+)_(\d+)\.png$/);
        if (m) {
          const e = manifest[parseInt(m[1], 16) % manifest.length];
          const t = u.searchParams.get('t');
          res.setHeader('content-type', 'image/png');
          res.setHeader('cache-control', 'private, max-age=86400');
          if (!t || t === DEFAULT_SAMPLE) return fs.createReadStream(path.join(PREVIEWS, `${e.id}.png`)).pipe(res);
          return execFile(BIN, ['one', e.path, String(e.faceIndex), m[3], t], { encoding: 'buffer', maxBuffer: 1 << 24 }, (err, out) => {
            if (err) { res.statusCode = 422; res.setHeader('content-type', 'text/plain'); return res.end('no_outlines'); }
            res.end(out);
          });
        }
        if (u.pathname === '/api/result' && req.method === 'POST') {
          let body = '';
          req.on('data', c => (body += c));
          req.on('end', () => { fs.appendFileSync(path.join(HERE, '.data/results.jsonl'), body.trim() + '\n'); res.end('ok'); });
          return;
        }
        next();
      });
    },
  }],
});
