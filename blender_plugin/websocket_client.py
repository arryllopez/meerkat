# websocket_client.py — background thread + asyncio event loop
# Blender runs Python on a single main thread. Blocking it freezes the UI.
# The WebSocket lives on a daemon thread with its own asyncio loop.
# A queue.Queue bridges the two threads: background puts messages in,
# main thread's timer (Step 2.5) pulls them out.

import threading
import asyncio
import json
import queue
import websockets


class WebSocketClient:

    def __init__(self, url):
        self.url = url
        self.ws = None
        self.loop: asyncio.AbstractEventLoop | None = None
        self.thread = None
        self.incoming = queue.Queue()
        self.running = False

    def connect(self):
        self.running = True
        self.loop = asyncio.new_event_loop()
        self.thread = threading.Thread(target=self._run_loop, daemon=True)
        self.thread.start()

    def _run_loop(self):
        assert self.loop is not None
        asyncio.set_event_loop(self.loop)
        self.loop.run_until_complete(self._listen())

    async def _listen(self):
        try:
            async with websockets.connect(self.url) as ws:
                self.ws = ws
                while self.running:
                    try:
                        raw = await asyncio.wait_for(ws.recv(), timeout=1.0)
                        msg = json.loads(raw)
                        self.incoming.put(msg)
                    except asyncio.TimeoutError:
                        continue
                    except websockets.ConnectionClosed:
                        break
        except Exception as e:
            self.incoming.put({"event_type": "Error", "payload": {"code": "CONNECTION_ERROR", "message": str(e)}})
        finally:
            self.running = False

    def send(self, message_dict):
        if self.ws and self.loop and self.running:
            data = json.dumps(message_dict)
            asyncio.run_coroutine_threadsafe(self.ws.send(data), self.loop)

    def disconnect(self):
        self.running = False
        if self.ws and self.loop:
            asyncio.run_coroutine_threadsafe(self.ws.close(), self.loop)
        self.ws = None
