"""A local WebSocket fixture: validate native receive and disconnect clearing."""
import base64
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import threading
import time

root = Path(__file__).resolve().parent
binary = root / 'target/Dossier HUD Demo.app/Contents/MacOS/macos-overlay'
listener = socket.socket()
listener.bind(('127.0.0.1', 0))
listener.listen(1)
listener.settimeout(8)
errors = []

def serve():
    try:
        conn, _ = listener.accept()
        with conn:
            conn.settimeout(3)
            request = b''
            while b'\r\n\r\n' not in request:
                chunk = conn.recv(4096)
                if not chunk or len(request) > 16384:
                    raise RuntimeError('invalid handshake')
                request += chunk
            key = next(line.split(b':', 1)[1].strip() for line in request.split(b'\r\n') if line.lower().startswith(b'sec-websocket-key:'))
            accept = base64.b64encode(hashlib.sha1(key + b'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest())
            conn.sendall(b'HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ' + accept + b'\r\n\r\n')
            event = dict(type='BeatmapStateWebSocketMessage', beatmap=dict(metadata=dict(artist='Fixture', title='Song'), difficulty_name='Hard', star_rating=5.2, maximum_pp=200, max_combo=1234, bpm=180), mods=[])
            data = json.dumps(event).encode()
            conn.sendall(b'\x81\x7e' + len(data).to_bytes(2, 'big') + data)
            time.sleep(0.3)
            # A broken connection must discard previously received map data.
    except Exception as error:
        errors.append(error)
    finally:
        listener.close()

worker = threading.Thread(target=serve)
worker.start()
result = subprocess.run([str(binary), '--api-test'], env={**os.environ, 'DOSSIER_TEST_PORT': str(listener.getsockname()[1])}, timeout=10)
worker.join()
assert not errors, errors
assert result.returncode == 0, result.returncode
