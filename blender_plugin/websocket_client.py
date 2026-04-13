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


RECONNECT_DELAYS = [3, 9, 27]  # powers of 3, ~39s total
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
        retry_index = 0
        is_first_connect = True

        while self.running:
            try:
                async with websockets.connect(self.url) as ws:
                    self.ws = ws
                    self.last_close_code = None
                    self.last_close_reason = ""
                    self.connected_event.set()
                    state.connected = True
                    state.evicted = False
                    state.reconnecting = False
                    state.reconnect_attempt = 0
                    retry_index = 0  # reset on successful connection

                    # re-send JoinSession only on reconnect (not first connect)
                    if not is_first_connect and self.session_id and self.display_name:
                        join_msg = json.dumps({
                            "event_type": "JoinSession",
                            "payload": {
                                "session_id": self.session_id,
                                "display_name": self.display_name,
                            }
                        })
                        await ws.send(join_msg)

                    is_first_connect = False

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
                            state.reconnecting = True
                            state.reconnect_attempt = 0
                            self.ws = None
                            break

            except Exception as e:
                state.connected = False
                self.ws = None
                print(f"[Meerkat] WebSocket listen/connect error: {type(e).__name__}: {e}")
                pass  # fall through to retry logic below

            # --- Retry logic ---
            if not self.running or state.intentional_disconnect:
                break

            if retry_index >= len(RECONNECT_DELAYS):
                print("[Meerkat] Reconnect failed after all attempts.")
                state.reconnecting = False
                state.reconnect_attempt = 0
                state.connected = False
                break

            delay = RECONNECT_DELAYS[retry_index]
            retry_index += 1
            state.reconnecting = True
            state.reconnect_attempt = retry_index
            print(f"[Meerkat] Connection lost. Reconnecting ({retry_index}/{len(RECONNECT_DELAYS)}) in {delay}s...")

            try:
                await asyncio.sleep(delay)
            except asyncio.CancelledError:
                break

            if not self.running or state.intentional_disconnect:
                break

            # loop back to the top -> websockets.connect() again

        self.ws = None
        self.running = False

    def send(self, message_dict):
        ws = self.ws
        if ws and self.loop and self.running and not ws.closed:
            data = json.dumps(message_dict)
            asyncio.run_coroutine_threadsafe(ws.send(data), self.loop)

    def disconnect(self):
        self.running = False
        if self.ws and self.loop:
            asyncio.run_coroutine_threadsafe(self.ws.close(), self.loop)
        self.ws = None
        self.last_close_code = None
        self.last_close_reason = ""

    def is_evicted(self):
        ws = self.ws
        if ws is not None and ws.closed:
            return ws.close_code == EVICTED_CLOSE_CODE
        return self.last_close_code == EVICTED_CLOSE_CODE
