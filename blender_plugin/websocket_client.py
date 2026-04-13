# websocket_client.py — background thread + asyncio event loop
# Blender runs Python on a single main thread. Blocking it freezes the UI.
# The WebSocket lives on a daemon thread with its own asyncio loop.
# A queue.Queue bridges the two threads: background puts messages in,
# main thread's timer (Step 2.5) pulls them out.

import threading
import asyncio
import json
import queue
import time
import websockets


EVICTED_CLOSE_CODE = 4008


class WebSocketClient:

    def __init__(self, url):
        self.url = url
        self.ws = None
        self.loop: asyncio.AbstractEventLoop | None = None
        self.thread = None
        self.incoming = queue.Queue()
        self.running = False
        self.connected_event = threading.Event()
        self.session_id = ""
        self.display_name = ""
        self.last_close_code = None
        self.last_close_reason = ""

    def connect(self, session_id="", display_name=""):
        self.session_id = session_id
        self.display_name = display_name
        self.running = True
        self.connected_event.clear()
        self.loop = asyncio.new_event_loop()
        self.thread = threading.Thread(target=self._run_loop, daemon=True)
        self.thread.start()
        if not self.connected_event.wait(timeout=5):
            self.running = False
            raise ConnectionError("WebSocket handshake timed out")

    def _run_loop(self):
        assert self.loop is not None
        asyncio.set_event_loop(self.loop)
        self.loop.run_until_complete(self._listen())

    async def _listen(self):
        from .state import PluginState
        state = PluginState()

        try:
            async with websockets.connect(self.url) as ws:
                self.ws = ws
                self.connected_event.set()
                state.connected = True
                state.evicted = False
                state.reconnecting = False
                state.reconnect_attempt = 0

                while self.running:
                    try:
                        raw = await asyncio.wait_for(ws.recv(), timeout=1.0)
                        msg = json.loads(raw)
                        self.incoming.put(msg)
                    except asyncio.TimeoutError:
                        continue
                    except websockets.ConnectionClosed as e:
                        self.last_close_code = e.code
                        self.last_close_reason = e.reason or ""
                        state.evicted = (e.code == EVICTED_CLOSE_CODE)
                        state.connected = False
                        break
        except Exception as e:
            state.connected = False
            print(f"[Meerkat] WebSocket error: {type(e).__name__}: {e}")

        self.ws = None
        self.running = False

    def send(self, message_dict):
        from .state import PluginState
        state = PluginState()
        ws = self.ws
        if not ws or not self.loop or not self.running:
            return
        envelope = {
            "event_type": message_dict.get("event_type"),
            "timestamp": int(time.time() * 1000),
            "source_user_id": state.user_id,
            "payload": message_dict.get("payload", {})
        }
        data = json.dumps(envelope)
        future = asyncio.run_coroutine_threadsafe(ws.send(data), self.loop)
        future.add_done_callback(self._on_send_done)


    def _on_send_done(self, future):    
        try:
            future.result()
        except Exception as e:  
            print(f"[Meerkat] Error sending message: {type(e).__name__}: {e}")

    def disconnect(self):
        from .state import PluginState
        state = PluginState()
        state.intentional_disconnect = True
        self.running = False
        if self.ws and self.loop:
            future = asyncio.run_coroutine_threadsafe(self.ws.close(), self.loop)
            future.add_done_callback(self._on_disconnect_done)
        else:
            self._clear_connection_state()

    def _on_disconnect_done(self, future):
        try:
            future.result()
        except Exception as e:
            print(f"[Meerkat] Error closing WebSocket: {type(e).__name__}: {e}")
        self._clear_connection_state()

    def _clear_connection_state(self):
        from .state import PluginState
        state = PluginState()
        self.ws = None
        self.last_close_code = None
        self.last_close_reason = ""
        state.reconnecting = False
        state.reconnect_attempt = 0

    def is_evicted(self):
        ws = self.ws
        # websockets 16.x: use ws.open to check if connection is open
        if ws is not None and not getattr(ws, "open", True):
            return getattr(ws, "close_code", None) == EVICTED_CLOSE_CODE
        return self.last_close_code == EVICTED_CLOSE_CODE
    