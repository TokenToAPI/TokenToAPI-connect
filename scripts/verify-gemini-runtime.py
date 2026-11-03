#!/usr/bin/env python3
"""Run the real Gemini CLI with adapter-generated settings and a loopback API."""
import argparse, http.server, json, os, pathlib, subprocess, tempfile, threading
p=argparse.ArgumentParser(); p.add_argument('--gemini',default='gemini'); p.add_argument('--gemini-script'); p.add_argument('--output',required=True); a=p.parse_args()
requests=[]
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,*args): pass
 def do_POST(self):
  body=json.loads(self.rfile.read(int(self.headers.get('Content-Length',0))))
  requests.append({'path':self.path,'auth':self.headers.get('x-goog-api-key')=='fixture-only-key','body':body})
  if ':countTokens' in self.path:
   data=json.dumps({'totalTokens':20}).encode();content='application/json'
  else:
   reply={'candidates':[{'content':{'parts':[{'text':'GEMINI_FIXTURE_OK'}],'role':'model'},'finishReason':'STOP','index':0}],'usageMetadata':{'promptTokenCount':20,'candidatesTokenCount':5,'totalTokenCount':25}}
   if ':streamGenerateContent' in self.path: data=('data: '+json.dumps(reply)+'\n\n').encode();content='text/event-stream'
   else: data=json.dumps(reply).encode();content='application/json'
  self.send_response(200);self.send_header('Content-Type',content);self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler);threading.Thread(target=server.serve_forever,daemon=True).start()
root=pathlib.Path(__file__).resolve().parents[1]
try:
 with tempfile.TemporaryDirectory(prefix='t2a-gemini-runtime-') as d:
  home=pathlib.Path(d);project=home/'project';project.mkdir()
  subprocess.run(['cargo','run','--quiet','--manifest-path',str(root/'src-tauri/Cargo.toml'),'--no-default-features','--example','gemini_fixture','--',d,f'http://127.0.0.1:{server.server_port}'],check=True)
  # Trust is restricted to the temporary fixture; normal user trust is untouched.
  (home/'.gemini/trustedFolders.json').write_text(json.dumps({str(project):'TRUST_FOLDER'}))
  env={k:v for k,v in os.environ.items() if not k.startswith(('GEMINI_','GOOGLE_','GCLOUD_')) and k.lower() not in ['http_proxy','https_proxy','all_proxy']}
  env.update(GEMINI_CLI_HOME=d,GEMINI_TELEMETRY_ENABLED='false',NO_COLOR='1')
  result=subprocess.run([a.gemini]+([a.gemini_script] if a.gemini_script else [])+['-p','Reply with GEMINI_FIXTURE_OK only.','--output-format','json'],cwd=project,env=env,capture_output=True,text=True,timeout=90)
  inference=[q for q in requests if ':generateContent' in q['path'] or ':streamGenerateContent' in q['path']]
  report={'cliExitCode':result.returncode,'requestPaths':[q['path'] for q in requests],'authenticated':bool(requests) and all(q['auth'] for q in requests),'nativeInference':bool(inference),'selectedModel':bool(inference) and all('/models/gemini-2.5-pro:' in q['path'] for q in inference),'responseReceived':'GEMINI_FIXTURE_OK' in result.stdout,'settingsGeneratedByAdapter':True,'realClientSettingsChanged':False}
  pathlib.Path(a.output).write_text(json.dumps(report,indent=2)+'\n')
  if not all([result.returncode==0,report['authenticated'],report['nativeInference'],report['selectedModel'],report['responseReceived']]):
   print(result.stdout[-1500:]); print(result.stderr[-2500:]); raise SystemExit(json.dumps(report))
  print(json.dumps(report))
finally: server.shutdown()
