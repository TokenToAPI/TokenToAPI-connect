"""Exercise generated Grok config with the official CLI and an isolated loopback gateway."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import tomllib
from http.server import ThreadingHTTPServer
import importlib.util
module_spec = importlib.util.spec_from_file_location('codex_runtime', Path(__file__).with_name('verify-codex-runtime.py'))
module = importlib.util.module_from_spec(module_spec)
module_spec.loader.exec_module(module)
Gateway, ROOT = module.Gateway, module.ROOT

parser = argparse.ArgumentParser()
parser.add_argument('--grok', required=True)
args = parser.parse_args()
fixture = ROOT / 'src-tauri/target/debug/examples/codex_fixture'
subprocess.run(['cargo', 'build', '--manifest-path', str(ROOT / 'src-tauri/Cargo.toml'), '--no-default-features', '--example', 'codex_fixture'], check=True)
with tempfile.TemporaryDirectory(prefix='connect-grok-runtime-') as directory:
    home = Path(directory)
    (home / '.tokentoapi-codex-fixture').touch()
    gateway = ThreadingHTTPServer(('127.0.0.1', 0), Gateway)
    gateway.requests, gateway.model_requests, gateway.usage = [], [], 10
    gateway.manifest_payload = {'models': [dict(slug=id, context_window=500000, input_modalities=['text', 'image'], default_reasoning_level='high', supported_reasoning_levels=[{'effort':'low'}, {'effort':'high'}]) for id in ['grok-4.6', 'grok-4.7']]}
    gateway.plain_payload = {'data': [{'id': id} for id in ['grok-4.6', 'grok-4.7', 'grok-imagine-image']]}
    threading.Thread(target=gateway.serve_forever, daemon=True).start()
    spec = home / 'fixture.json'
    specification = dict(home=str(home), root=f'http://127.0.0.1:{gateway.server_port}', client='grok', selectedModels=['grok-4.6','grok-4.7'], defaultModel='grok-4.7')
    spec.write_text(json.dumps(specification))
    subprocess.run([str(fixture), str(spec)], check=True, capture_output=True)
    env = {k:v for k,v in os.environ.items() if not k.startswith(('GROK_', 'XAI_'))}
    env['GROK_HOME'] = str(home / '.grok')
    # Test-only settings avoid first-party telemetry and automatic CLI updates.
    config = home / '.grok/config.toml'
    original = config.read_text()
    config.write_text(original + '\n[telemetry]\nenabled = false\n')
    def run(*arguments):
        result = subprocess.run([str(Path(args.grok).resolve()), *arguments], cwd=home, env=env, text=True, capture_output=True, timeout=60)
        if result.returncode:
            raise AssertionError(f'Grok exited {result.returncode}: {result.stdout}\n{result.stderr}')
        assert 'unknown field' not in result.stderr.lower(), result.stderr
        return result.stdout
    print(run('--version').strip())
    models = run('models')
    assert 'grok-4.6' in models and 'grok-4.7' in models, models
    assert 'grok-imagine' not in models, models
    output = run('-p', 'Reply OK only. Do not use tools.', '--max-turns', '1', '--disable-web-search')
    assert 'OK' in output, output
    assert gateway.requests and gateway.requests[-1]['model'] == 'grok-4.7', gateway.requests
    assert all(request['model'] in ['grok-4.6','grok-4.7'] for request in gateway.requests)
    generated = tomllib.loads(original)
    alias = next(key for key, row in generated['model'].items() if row['model'] == 'grok-4.6')
    before = len(gateway.requests)
    assert 'OK' in run('-p', 'Reply OK only. Do not use tools.', '-m', alias, '--reasoning-effort', 'low', '--max-turns', '1', '--disable-web-search')
    selected_requests = [request for request in gateway.requests[before:] if request['model'] == 'grok-4.6']
    assert selected_requests and any(request.get('reasoning', {}).get('effort') == 'low' for request in selected_requests), selected_requests
    config.write_text(original)
    specification['restore'] = True
    spec.write_text(json.dumps(specification))
    subprocess.run([str(fixture), str(spec)], check=True, capture_output=True)
    assert not config.exists()
    gateway.shutdown()
    print('PASS: discovery, selected-only picker, authenticated Responses inference, original model ID, restore.')
