#!/usr/bin/env python3
"""Serves the fling recorder (index.html beside this file) and writes each
recording it POSTs to save into the directory given, one JSON file each,
numbered in arrival order. Binds 127.0.0.1 only; put it on the tailnet with
`tailscale serve --bg --set-path /fling http://127.0.0.1:<port>`.

Usage: scripts/fling-recorder/serve.py <recordings dir> [port]
"""
import http.server
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
OUT = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else None
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 8796
LIMIT = 8 * 1024 * 1024  # a fling of a few seconds is well under 1 MB


class Recorder(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(HERE), **kwargs)

    def do_POST(self):
        if self.path.rstrip('/').split('/')[-1] != 'save':
            return self.send_error(404, 'POST only to save')
        size = int(self.headers.get('content-length', 0))
        if not 0 < size <= LIMIT:
            return self.send_error(413, f'a recording is 1 to {LIMIT} bytes, not {size}')
        body = self.rfile.read(size)
        try:
            json.loads(body)
        except ValueError as error:
            return self.send_error(400, f'not JSON: {error}')
        number = len(list(OUT.glob('fling-*.json'))) + 1
        path = OUT / f'fling-{number:03}.json'
        path.write_bytes(body)
        print(f'saved {path} ({size} bytes)', flush=True)
        self.send_response(204)
        self.end_headers()


if __name__ == '__main__':
    if OUT is None:
        sys.exit('usage: serve.py <recordings dir> [port]')
    OUT.mkdir(parents=True, exist_ok=True)
    http.server.ThreadingHTTPServer(('127.0.0.1', PORT), Recorder).serve_forever()
