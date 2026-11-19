# PROTOTYPE stand-in sync server. Emulator reaches it at 10.0.2.2:8787.
# POST /sync    apply one outbox command (logs it)
# GET  /tasks   full task list (stand-in for sync-token delta)
# POST /admin/add {"title","due","project","recurrence"}  simulate an edit from another device
import http.server, json, datetime, uuid

d = lambda n: (datetime.date.today() + datetime.timedelta(days=n)).isoformat()
TASKS = {t["id"]: t for t in [
    {"id": "t1", "title": "Pay rent", "due": d(-1), "project": "Home", "recurrence": None, "done": False},
    {"id": "t2", "title": "Lab report", "due": d(0), "project": "College", "recurrence": None, "done": False},
    {"id": "t3", "title": "Water plants", "due": d(0), "project": "Home", "recurrence": "daily", "done": False},
    {"id": "t4", "title": "Read paper", "due": d(2), "project": "Career", "recurrence": None, "done": False},
    {"id": "t5", "title": "Undated idea", "due": None, "project": "Inbox", "recurrence": None, "done": False},
]}

class H(http.server.BaseHTTPRequestHandler):
    def reply(self, obj):
        b = json.dumps(obj).encode()
        self.send_response(200); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)

    def do_GET(self):
        print(datetime.datetime.now().isoformat(timespec="seconds"), "PULL", flush=True)
        self.reply(list(TASKS.values()))

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])) or b"{}")
        now = datetime.datetime.now().isoformat(timespec="seconds")
        if self.path == "/admin/add":
            t = {"id": str(uuid.uuid4())[:8], "done": False, **body}; TASKS[t["id"]] = t
            print(now, "ADMIN ADD", t, flush=True)
        else:
            print(now, "RECEIVED", json.dumps(body), flush=True)
            if body.get("type") == "task_complete" and (t := TASKS.get(body["args"]["id"])):
                if t["recurrence"] == "daily":
                    t["due"] = (datetime.date.fromisoformat(t["due"]) + datetime.timedelta(days=1)).isoformat()
                else:
                    t["done"] = True
        self.reply({"ok": True})

http.server.HTTPServer(("0.0.0.0", 8787), H).serve_forever()
