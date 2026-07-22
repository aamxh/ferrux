import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

port = int(sys.argv[1])

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(f"Backend on port {port}".encode())

    def log_message(self, format, *args):
        print(f"[{port}] hit from {self.client_address}")

HTTPServer(('127.0.0.1', port), Handler).serve_forever()