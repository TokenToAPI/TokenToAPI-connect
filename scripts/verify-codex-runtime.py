"""Verify the production Codex importer against a real app-server and loopback gateway."""
import argparse
import base64
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import shutil
import subprocess
import tarfile
import tempfile
import threading
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.162.0"
MODEL = "gpt-6.1-sol"
FAKE_KEY = "sk-tokentoapi-fixture-only"
IMAGE_URL = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEklEQVR4nGP4z8DAAMIM/4EAAB/uBfsL2WiLAAAAAElFTkSuQmCC"


def download_codex(directory):
    system, machine = platform.system(), platform.machine().lower()
    if system == "Windows":
        name = "codex-x86_64-pc-windows-msvc.exe"
    elif system == "Darwin":
        arch = "aarch64" if machine in ("arm64", "aarch64") else "x86_64"
        name = f"codex-{arch}-apple-darwin.tar.gz"
    else:
        raise RuntimeError("Pass --codex PATH outside macOS and Windows")
    headers = {"User-Agent": "TokenToAPI-Connect-verification"}
    if token := os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + token
    request = urllib.request.Request(
        f"https://api.github.com/repos/openai/codex/releases/tags/rust-v{VERSION}",
        headers=headers,
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        release = json.load(response)
    asset = next(asset for asset in release["assets"] if asset["name"] == name)
    assert asset["browser_download_url"].startswith("https://github.com/openai/codex/releases/download/")
    archive = directory / name
    urllib.request.urlretrieve(asset["browser_download_url"], archive)
    actual = "sha256:" + hashlib.sha256(archive.read_bytes()).hexdigest()
    assert actual == asset["digest"], f"Codex checksum mismatch: {actual}"
    if system == "Windows":
        return archive
    with tarfile.open(archive) as bundle:
        member = next(
            item
            for item in bundle.getmembers()
            if item.isfile() and Path(item.name).name == name.removesuffix(".tar.gz")
        )
        binary = directory / "codex"
        with bundle.extractfile(member) as source, binary.open("wb") as destination:
            shutil.copyfileobj(source, destination)
    binary.chmod(0o700)
    return binary


class Gateway(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        if self.headers.get("Authorization") != "Bearer " + FAKE_KEY:
            self.send_error(404)
            return
        self.server.model_requests.append(self.path)
        if self.path == "/v1/models?client_version=0.162.0":
            body = json.dumps(self.server.manifest_payload).encode()
        elif self.path == "/v1/models":
            body = json.dumps(self.server.plain_payload).encode()
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        if self.path != "/v1/responses" or self.headers.get("Authorization") != "Bearer " + FAKE_KEY:
            self.send_error(404)
            return
        body = self.rfile.read(int(self.headers["Content-Length"]))
        if self.headers.get("Content-Encoding") == "gzip":
            body = gzip.decompress(body)
        request = json.loads(body)
        self.server.requests.append(request)
        tokens = self.server.usage
        message = {
            "id": "msg_fixture",
            "type": "message",
            "status": "completed",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "OK", "annotations": []}],
        }
        response = {
            "id": f"resp_{len(self.server.requests)}",
            "object": "response",
            "model": request["model"],
            "status": "completed",
            "output": [message],
            "usage": {"input_tokens": tokens, "output_tokens": 1, "total_tokens": tokens + 1},
        }
        events = [
            {"type": "response.created", "response": {**response, "status": "in_progress", "output": []}},
            {"type": "response.output_item.added", "output_index": 0, "item": {**message, "status": "in_progress", "content": []}},
            {"type": "response.output_text.delta", "item_id": "msg_fixture", "output_index": 0, "content_index": 0, "delta": "OK"},
            {"type": "response.output_item.done", "output_index": 0, "item": message},
            {"type": "response.completed", "response": response},
        ]
        stream = "".join(
            f"event: {event['type']}\ndata: {json.dumps(event)}\n\n" for event in events
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Content-Length", str(len(stream)))
        self.end_headers()
        self.wfile.write(stream)


class Codex:
    def __init__(self, executable, home):
        env = os.environ.copy()
        for key in ("OPENAI_API_KEY", "OPENAI_BASE_URL", "CODEX_PROFILE"):
            env.pop(key, None)
        env["CODEX_HOME"] = str(home / ".codex")
        self.process = subprocess.Popen(
            [str(executable), "app-server", "--stdio"],
            cwd=home,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        self.incoming = queue.Queue()
        self.events = []
        self.errors = []
        self.sequence = 0
        threading.Thread(target=self._read_stdout, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()
        self.rpc(
            "initialize",
            {"clientInfo": {"name": "tokentoapi-connect-regression", "version": "1"},
             "capabilities": {"experimentalApi": True}},
        )
        self.send({"method": "initialized"})

    def _read_stdout(self):
        for line in self.process.stdout:
            try:
                self.incoming.put(json.loads(line))
            except json.JSONDecodeError:
                continue

    def _read_stderr(self):
        self.errors.extend(self.process.stderr)

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def receive(self, predicate):
        deadline = time.monotonic() + 35
        while time.monotonic() < deadline:
            try:
                message = self.incoming.get(timeout=max(0.1, deadline - time.monotonic()))
            except queue.Empty as error:
                raise RuntimeError("Codex timed out: " + "".join(self.errors[-8:])) from error
            self.events.append(message)
            if predicate(message):
                return message
        raise TimeoutError("Codex response timed out")

    def rpc(self, method, params):
        self.sequence += 1
        request_id = self.sequence
        self.send({"id": request_id, "method": method, "params": params})
        reply = self.receive(lambda message: message.get("id") == request_id)
        if "error" in reply:
            raise RuntimeError(f"Codex {method} failed: {reply['error']}\n" + "".join(self.errors[-8:]))
        return reply["result"]

    def start(self, home, model):
        reply = self.rpc(
            "thread/start",
            {"cwd": str(home), "model": model, "approvalPolicy": "never", "sandbox": "read-only", "ephemeral": True},
        )
        self.last_start = reply
        return reply["thread"]["id"]

    def turn(self, thread, model, effort=None, image=None):
        input_items = [{"type": "text", "text": "CTX_FIXTURE_: Reply OK without tools."}]
        if image:
            input_items.append(image)
        self.rpc(
            "turn/start",
            {"threadId": thread, "model": model, "effort": effort, "input": input_items},
        )
        completed = self.receive(
            lambda message: message.get("method") == "turn/completed"
            and message["params"]["threadId"] == thread
        )
        assert completed["params"]["turn"]["status"] == "completed", completed
        usage = [
            message["params"]["tokenUsage"]
            for message in self.events
            if message.get("method") == "thread/tokenUsage/updated"
            and message["params"]["threadId"] == thread
        ]
        assert usage, "Codex did not report the model context window"
        return usage[-1]["modelContextWindow"]

    def close(self):
        if self.process.poll() is None:
            if os.name == "nt":
                # Codex can leave a child process holding CODEX_HOME after the
                # stdio parent exits. Kill the private process tree so the
                # temporary fixture can be removed deterministically.
                subprocess.run(
                    ["taskkill", "/PID", str(self.process.pid), "/T", "/F"],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    check=False,
                )
            else:
                self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()


def run_fixture(executable, request_path, specification, restore=False):
    request_path.write_text(
        json.dumps({**specification, "restore": restore}), encoding="utf-8"
    )
    result = subprocess.run(
        [str(executable), str(request_path)],
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def run_case(
    label,
    binary,
    fixture,
    temporary,
    output,
    gateway,
    endpoint,
    payload,
    legacy=False,
    repair=False,
    runtime=False,
    vision=False,
    model=MODEL,
    selected_models=None,
    plain_payload=None,
    numeric=False,
):
    home = temporary / (label + " Unicode User")
    home.mkdir()
    (home / ".tokentoapi-codex-fixture").touch()
    codex_home = home / ".codex"
    codex_home.mkdir()
    original_config = (
        "# Original Codex settings\r\n"
        'model_context_window = 500000\r\n'
        'model_auto_compact_token_limit = 450000\r\n'
        'model_reasoning_effort = "ultra"\r\n'
        'plan_mode_reasoning_effort = "xhigh"\r\n'
        'approval_policy = "on-request"\r\n'
    ).encode()
    original_auth = b'{\r\n  "OPENAI_API_KEY": "original-fake-key"\r\n}\r\n'
    (codex_home / "config.toml").write_bytes(original_config)
    (codex_home / "auth.json").write_bytes(original_auth)
    specification = {
        "home": str(home),
        "root": endpoint,
        "client": "codex-desktop",
        "selectedModels": selected_models or [model],
        "defaultModel": model,
    }
    request_path = output / (label + ".json")
    catalog_path = codex_home / "tokentoapi-models.json"
    gateway.manifest_payload = payload
    gateway.plain_payload = plain_payload if plain_payload is not None else payload
    gateway.requests, gateway.usage = [], 100
    gateway.model_requests = []
    run_fixture(fixture, request_path, specification)

    if legacy:
        old = json.loads(catalog_path.read_text(encoding="utf-8"))
        old.pop("_tokentoapi_connect", None)
        old["models"][0].update(context_window=32768, max_context_window=32768)
        catalog_path.write_text(json.dumps(old), encoding="utf-8")
        run_fixture(fixture, request_path, specification)
    if repair:
        edited = json.loads(catalog_path.read_text(encoding="utf-8"))
        edited["models"][0]["context_window"] = 500000
        edited["models"][0]["auto_compact_token_limit"] = 440000
        catalog_path.write_text(json.dumps(edited), encoding="utf-8")
        gateway.manifest_payload = {"data": [{"id": model, "effective_context_window_percent": 90}]}
        run_fixture(fixture, request_path, specification)

    assert not run_fixture(fixture, request_path, specification)["changed"], "reimport was not stable"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    if legacy:
        assert catalog["models"][0]["context_window"] == 272000
        assert catalog["models"][0]["max_context_window"] == 872000
    if repair:
        row = catalog["models"][0]
        assert row["context_window"] == 500000
        assert row["max_context_window"] == 872000
        assert row["auto_compact_token_limit"] == 440000
        assert row["effective_context_window_percent"] == 90

    case = {"name": label, "passed": False, "models": []}
    if runtime:
        client = Codex(binary, home)
        try:
            listed = client.rpc("model/list", {"includeHidden": True, "limit": 100})["data"]
            picked = next(row for row in listed if row["model"] == model)
            efforts = [item["reasoningEffort"] for item in picked["supportedReasoningEfforts"]]
            expected_efforts = ["1024", "4096"] if numeric else ["low", "medium", "high", "xhigh", "max", "ultra"]
            expected_default = "4096" if numeric else "low"
            assert efforts == expected_efforts, efforts
            assert picked["defaultReasoningEffort"] == expected_default, picked
            thread = client.start(home, model)
            expected_start_effort = None if numeric else "ultra"
            assert client.last_start.get("reasoningEffort") == expected_start_effort, client.last_start
            before = len(gateway.requests)
            selected_effort = "4096" if numeric else "ultra"
            context_window = client.turn(thread, model, selected_effort)
            expected_window = 121600 if numeric else 450000 if repair else 475000
            assert context_window == expected_window, (context_window, expected_window)
            first_request = gateway.requests[before:]
            expected_request_effort = 4096 if numeric else "xhigh"
            assert len(first_request) == 1 and first_request[0]["model"] == model
            assert first_request[0]["reasoning"]["effort"] == expected_request_effort, first_request
            case["models"].append({"id": model, "context_window": context_window, "request_effort": expected_request_effort})

            if not numeric:
                gateway.usage = 451000
                client.turn(thread, model, "low")
                gateway.usage = 100
                event_start = len(client.events)
                client.turn(thread, model, "low")
                compactions = [
                    event for event in client.events[event_start:]
                    if event.get("method") == "item/started"
                    and event.get("params", {}).get("item", {}).get("type") == "contextCompaction"
                ]
                assert len(compactions) == 1, f"expected one compaction event, found {len(compactions)}"
                case.update(reported_usage=451000, compaction_events=len(compactions))
            if vision:
                models_by_id = {row["id"]: row for row in catalog["models"]}
                assert models_by_id["gpt-5.6-sol"]["input_modalities"] == ["text", "image"]
                assert models_by_id["text-only-alias"]["input_modalities"] == ["text"]
                assert all("?client_version=0.162.0" in request for request in gateway.model_requests)
                image_path = home / "image fixture.png"
                image_path.write_bytes(base64.b64decode(IMAGE_URL.split(",", 1)[1]))
                for image in [
                    {"type": "image", "url": IMAGE_URL},
                    {"type": "localImage", "path": str(image_path)},
                ]:
                    thread = client.start(home, model)
                    before = len(gateway.requests)
                    client.turn(thread, model, image=image)
                    sent = gateway.requests[before:]
                    assert len(sent) == 1 and "input_image" in json.dumps(sent[0].get("input", [])), sent
                    assert IMAGE_URL in json.dumps(sent[0]["input"]), sent
                case["image_attachment_types_sent"] = ["image", "localImage"]
        finally:
            client.close()
            (output / (label + "-events.json")).write_text(
                json.dumps(client.events, ensure_ascii=False, indent=2), encoding="utf-8"
            )
            (output / (label + "-stderr.log")).write_text("".join(client.errors), encoding="utf-8")
    case["passed"] = True

    run_fixture(fixture, request_path, specification, restore=True)
    assert (codex_home / "config.toml").read_bytes() == original_config
    assert (codex_home / "auth.json").read_bytes() == original_auth
    assert not catalog_path.exists()
    case["restore_exact"] = True
    print("PASS", label, flush=True)
    return case


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--codex", type=Path)
    parser.add_argument("--download-codex", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "test-results/codex-runtime")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    fixture = ROOT / "src-tauri/target/debug/examples" / (
        "codex_fixture.exe" if os.name == "nt" else "codex_fixture"
    )
    assert fixture.is_file(), "Build the codex_fixture example first"
    report = {
        "platform": platform.platform(),
        "provider_calls": "loopback-only",
        "user_configuration_changed": False,
        "cases": [],
    }
    with tempfile.TemporaryDirectory(prefix="tokentoapi-codex-runtime-") as temporary:
        temporary = Path(temporary).resolve()
        if args.codex:
            binary = args.codex.resolve()
        else:
            assert args.download_codex, "Pass --codex PATH or --download-codex"
            binary = download_codex(temporary)
        report["codex_version"] = subprocess.check_output([str(binary), "--version"], text=True).strip()
        gateway = ThreadingHTTPServer(("127.0.0.1", 0), Gateway)
        gateway.requests, gateway.usage = [], 100
        gateway.model_requests = []
        threading.Thread(target=gateway.serve_forever, daemon=True).start()
        endpoint = "http://127.0.0.1:" + str(gateway.server_port)
        known = {"data": [{"id": MODEL}]}
        try:
            report["cases"].append(
                run_case("profile-and-runtime", binary, fixture, temporary, args.output, gateway, endpoint, known, runtime=True)
            )
            numeric = {
                "data": [{
                    "id": "numeric-model",
                    "context_window": 128000,
                    "supported_reasoning_levels": [{"effort": 1024}, {"effort": 4096}],
                    "default_reasoning_level": 4096,
                }]
            }
            report["cases"].append(
                run_case(
                    "numeric-effort-runtime", binary, fixture, temporary, args.output,
                    gateway, endpoint, numeric, runtime=True, model="numeric-model", numeric=True
                )
            )
            report["cases"].append(
                run_case("legacy-32k-repair", binary, fixture, temporary, args.output, gateway, endpoint, known, legacy=True)
            )
            report["cases"].append(
                run_case("partial-gateway-repair", binary, fixture, temporary, args.output, gateway, endpoint, known, repair=True)
            )
            vision = {"models": [
                {"slug": "gpt-5.6-sol", "input_modalities": ["text", "image"], "context_window": 272000},
                {"slug": "text-only-alias", "input_modalities": ["text"], "context_window": 65536},
            ]}
            report["cases"].append(
                run_case("vision-image-input", binary, fixture, temporary, args.output, gateway, endpoint,
                         vision, model="gpt-5.6-sol", vision=True,
                         selected_models=["gpt-5.6-sol", "text-only-alias"],
                         plain_payload={"data": [{"id": "gpt-5.6-sol"}, {"id": "text-only-alias"}]})
            )
            report["passed"] = True
        finally:
            gateway.shutdown()
            gateway.server_close()
            (args.output / "summary.json").write_text(
                json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
    print(json.dumps({"passed": report["passed"], "cases": len(report["cases"]), "codex": report["codex_version"]}))


if __name__ == "__main__":
    main()
