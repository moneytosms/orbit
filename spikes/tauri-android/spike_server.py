# PROTOTYPE stand-in sync server: logs every POSTed outbox command. Emulator reaches it at 10.0.2.2:8787.
import http.server, json, datetime
class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"]))
        print(datetime.datetime.now().isoformat(timespec="seconds"), "RECEIVED", body.decode(), flush=True)
        self.send_response(200); self.end_headers(); self.wfile.write(b'{"ok":true}')
http.server.HTTPServer(("0.0.0.0", 8787), H).serve_forever()
