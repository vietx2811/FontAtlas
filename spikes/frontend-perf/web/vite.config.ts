// SPIKE — giả lập backend: /font/:id ~ readFile qua IPC, /png|svg/:id ~ cache preview, /render ~ render theo yêu cầu.
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import fs from 'node:fs';
import path from 'node:path';
import { execFile } from 'node:child_process';

const OUT = path.resolve(__dirname, '../out');
const GEN = path.resolve(__dirname, '../gen/target/release/gen');
const MAX_BYTES = 40 << 20; // loại 6 file >40MB (Emoji, CJK ttc) khỏi pool thẻ
const all = JSON.parse(fs.readFileSync(path.join(OUT, 'manifest.json'), 'utf8')) as any[];
const pool = all.filter(e => e.bytes <= MAX_BYTES).sort((a, b) => a.id - b.id);
const byId = new Map(all.map(e => [e.id, e]));

export default defineConfig({
  plugins: [react(), {
    name: 'spike-backend',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const u = new URL(req.url!, 'http://x');
        let m;
        if (u.pathname === '/api/manifest') {
          res.setHeader('content-type', 'application/json');
          return res.end(JSON.stringify(pool.map(e => ({ id: e.id, family: e.family, style: e.style }))));
        }
        if ((m = u.pathname.match(/^\/font\/(\d+)$/))) {
          const e = byId.get(+m[1]);
          res.setHeader('content-type', 'application/octet-stream');
          res.setHeader('cache-control', 'no-store'); // readFile không có HTTP cache
          return fs.createReadStream(e.path).pipe(res);
        }
        if ((m = u.pathname.match(/^\/(png|svg)\/(\d+)\.(png|svg)$/))) {
          res.setHeader('content-type', m[1] === 'png' ? 'image/png' : 'image/svg+xml');
          res.setHeader('cache-control', 'max-age=3600');
          return fs.createReadStream(path.join(OUT, m[1], `${m[2]}.${m[3]}`)).pipe(res);
        }
        if ((m = u.pathname.match(/^\/render\/(\d+)\.png$/))) {
          const e = byId.get(+m[1]);
          return execFile(GEN, ['one', e.path, String(e.index), u.searchParams.get('text') ?? 'Aa', u.searchParams.get('px') ?? '48'],
            { encoding: 'buffer', maxBuffer: 1 << 24 }, (err, stdout) => {
              if (err) { res.statusCode = 500; return res.end(String(err)); }
              res.setHeader('content-type', 'image/png');
              res.end(stdout);
            });
        }
        if (u.pathname === '/api/result' && req.method === 'POST') {
          let body = '';
          req.on('data', c => (body += c));
          req.on('end', () => { fs.appendFileSync(path.resolve(__dirname, '../results.jsonl'), body.trim() + '\n'); res.end('ok'); });
          return;
        }
        next();
      });
    },
  }],
});
