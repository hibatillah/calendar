import { defineConfig } from 'vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
export default defineConfig({
  server: { host: '127.0.0.1', port: 3000, strictPort: true },
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  plugins: [
    { name: 'calendar-local-bridge', configureServer(server) {
      server.middlewares.use('/api/graphql', async (request, response) => {
        if (request.method !== 'POST' || request.headers.origin !== 'http://127.0.0.1:3000' || request.headers.host !== '127.0.0.1:3000') {
          response.writeHead(403).end(); return;
        }
        try {
          const chunks: Buffer[] = []; let length = 0;
          for await (const chunk of request) {
            length += chunk.length;
            if (length > 1_000_000) { response.writeHead(413).end(); return; }
            chunks.push(Buffer.from(chunk));
          }
          const token = (await readFile(new URL('../../.local/session-token', import.meta.url), 'utf8')).trim();
          const upstream = await fetch('http://127.0.0.1:47821/graphql', { method: 'POST', headers: { 'Content-Type': 'application/json', 'x-calendar-token': token }, body: Buffer.concat(chunks).toString(), signal: AbortSignal.timeout(30_000) });
          response.writeHead(upstream.status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
          response.end(await upstream.text());
        } catch { response.writeHead(503, { 'Content-Type': 'application/json' }).end(JSON.stringify({ errors: [{ message: 'Start the local Calendar backend, then retry.' }] })); }
      });
    } },
    tailwindcss(), tanstackStart(), react(),
  ],
});
