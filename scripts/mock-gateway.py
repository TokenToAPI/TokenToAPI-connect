"""Local UI smoke-test gateway. Only accepts a fake fixture key; never logs headers."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import secrets
import time
from urllib.parse import urlencode

handoffs = {}

class Handler(BaseHTTPRequestHandler):
    def reply(self, status, payload):
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        if self.path != '/api/v1/connect/handoff/redeem':
            self.reply(404, {'code': 404})
            return
        try:
            length = int(self.headers.get('Content-Length', 0))
            if length > 1024:
                raise ValueError()
            code = json.loads(self.rfile.read(length)).get('code')
            expiry = handoffs.pop(code, 0)
            if expiry <= time.time():
                raise ValueError()
        except (ValueError, TypeError):
            self.reply(400, {'code': 400, 'message': 'Prepare a new fixture handoff.'})
            return
        self.reply(200, {'code': 0, 'data': {
            'version': 1, 'providerName': 'Isolated fixture gateway',
            'baseUrl': 'http://127.0.0.1:18473',
            'apiKey': 'sk-local-fixture-only', 'suggestedClient': 'codex'}})

    def do_GET(self):
        if self.path in ('/fixture/issue', '/fixture/launch'):
            handoffs.clear()
            code = secrets.token_urlsafe(32)
            handoffs[code] = time.time() + 120
            url = 'tokentoapi-connect://v1/import?' + urlencode({
                'issuer': 'http://127.0.0.1:18473', 'code': code})
            if self.path == '/fixture/launch':
                body = ('<!doctype html><html><body><h1>Isolated Connect test</h1>'
                        '<p>Uses a fake key and a temporary client home.</p>'
                        '<a href="' + url.replace('&', '&amp;') + '">Open Connect test handoff</a>'
                        '</body></html>').encode()
                self.send_response(200)
                self.send_header('Content-Type', 'text/html')
                self.send_header('Content-Length', str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            else:
                self.reply(200, {'url': url})
            return
        if self.path != '/v1/models':
            self.send_error(404)
            return
        allowed = self.headers.get('Authorization') == 'Bearer sk-local-fixture-only'
        payload = {'data': [{'id': 'gpt-test-codex', 'name': 'GPT · Codex test model'}, {'id': 'gpt-test-mini', 'name': 'GPT · Mini test model'}, {'id': 'claude-sonnet-test', 'name': 'Claude · Sonnet test model'}, {'id': 'claude-opus-test', 'name': 'Claude · Opus test model'}, {'id': 'custom-model-test'}, {'id': 'excluded-model-test'}]} if allowed else {'error': 'unauthorized'}
        body = json.dumps(payload).encode()
        self.send_response(200 if allowed else 401)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass

if __name__ == '__main__':
    print('Local fixture gateway: http://127.0.0.1:18473', flush=True)
    HTTPServer(('127.0.0.1', 18473), Handler).serve_forever()
