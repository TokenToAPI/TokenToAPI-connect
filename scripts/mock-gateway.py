"""Local UI smoke-test gateway. Only accepts a fake fixture key; never logs headers."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
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
