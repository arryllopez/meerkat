"""Tests for WebSocketClient reconnect logic.

Mocks websockets.connect to simulate connection drops and retries,
then drives _listen() directly via asyncio.run() to verify behavior.
"""
import asyncio
import json
import sys
import os

plugin_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if plugin_dir not in sys.path:
    sys.path.insert(0, os.path.dirname(plugin_dir))

from blender_plugin.state import PluginState
from blender_plugin.websocket_client import WebSocketClient, RECONNECT_DELAYS, EVICTED_CLOSE_CODE
from websockets.exceptions import ConnectionClosed as WsConnectionClosed


# ── Mock WebSocket connection ──────────────────────────────────────────────

class MockWS:
    """Fake async websocket that records sent messages and can simulate drops."""

    def __init__(self, recv_messages=None, drop_after=None, close_code=None, close_reason=None):
        self.sent = []
        self._recv_messages = list(recv_messages or [])
        self._recv_index = 0
        self._drop_after = drop_after  # drop after N recv() calls
        self._recv_count = 0
        self.closed = False
        self.close_code = close_code
        self._close_code = close_code
        self._close_reason = close_reason

    async def send(self, data):
        self.sent.append(data)

    async def recv(self):
        self._recv_count += 1
        if self._drop_after is not None and self._recv_count > self._drop_after:
            raise WsConnectionClosed(self._close_code, self._close_reason)
        if self._recv_index < len(self._recv_messages):
            msg = self._recv_messages[self._recv_index]
            self._recv_index += 1
            return json.dumps(msg)
        # block until timeout
        await asyncio.sleep(10)
        return ""

    async def close(self):
        self.closed = True


class _SingleConnectCM:
    """One-shot async context manager that returns a single MockWS."""

    def __init__(self, ws):
        self._ws = ws

    async def __aenter__(self):
        if self._ws is None:
            raise ConnectionRefusedError("No more mock connections")
        return self._ws

    async def __aexit__(self, *args):
        pass


class MockConnectContext:
    """Callable replacing websockets.connect().

    Takes a list of MockWS instances. Each call returns a fresh context manager
    wrapping the next MockWS. When exhausted, raises ConnectionRefusedError.
    """

    def __init__(self, ws_sequence):
        self._sequence = list(ws_sequence)
        self._index = 0

    def __call__(self, url):
        if self._index >= len(self._sequence):
            return _SingleConnectCM(None)
        ws = self._sequence[self._index]
        self._index += 1
        return _SingleConnectCM(ws)


# ── Helpers ────────────────────────────────────────────────────────────────

def _reset_state():
    state = PluginState()
    state.connected = True
    state.reconnecting = False
    state.reconnect_attempt = 0
    state.intentional_disconnect = False
    return state


def _make_client(session_id="test-room", display_name="Tester"):
    client = WebSocketClient("ws://fake:8080")
    client.session_id = session_id
    client.display_name = display_name
    client.running = True
    return client


_real_sleep = asyncio.sleep
_sleep_delays = []


async def _fake_sleep(delay):
    """Record the delay but don't actually wait."""
    _sleep_delays.append(delay)
    await _real_sleep(0)  # yield control


def _patch(client, mock_connect):
    """Monkey-patch websockets.connect and asyncio.sleep for testing."""
    import blender_plugin.websocket_client as ws_mod
    ws_mod.websockets = type(sys)("fake_websockets")
    ws_mod.websockets.connect = mock_connect
    ws_mod.websockets.ConnectionClosed = WsConnectionClosed


def _unpatch():
    import blender_plugin.websocket_client as ws_mod
    import websockets
    ws_mod.websockets = websockets


# ── Tests ──────────────────────────────────────────────────────────────────

def run(result):
    print("\n— test_reconnect —")

    test_initial_connect_no_join_from_listen(result)
    test_reconnect_sends_join_session(result)
    test_reconnect_delays_are_3_9_27(result)
    test_intentional_disconnect_skips_retry(result)
    test_all_retries_exhausted(result)
    test_reconnect_resets_retry_index(result)
    test_reconnect_only_one_join_per_reconnect(result)
    test_eviction_close_code_sets_client_evicted(result)
    test_connection_close_marks_state_disconnected_during_retry(result)


def test_initial_connect_no_join_from_listen(result):
    """On first connect, _listen does NOT send JoinSession (operator does it)."""
    name = "initial connect: _listen sends no JoinSession"
    state = _reset_state()

    # one connection that stays alive for 1 recv then we stop
    ws1 = MockWS(drop_after=1)
    mock_connect = MockConnectContext([ws1])
    client = _make_client()
    _patch(client, mock_connect)

    async def drive():
        # stop after first connection drops
        task = asyncio.create_task(client._listen())
        await asyncio.sleep(0)  # let it connect
        await asyncio.sleep(0)  # let it recv + drop
        await asyncio.sleep(0)
        client.running = False
        state.intentional_disconnect = True
        await asyncio.sleep(0)
        try:
            await asyncio.wait_for(task, timeout=0.5)
        except (asyncio.TimeoutError, asyncio.CancelledError):
            pass

    try:
        asyncio.run(drive())
        join_msgs = [m for m in ws1.sent if "JoinSession" in m]
        if len(join_msgs) == 0:
            result.ok(name)
        else:
            result.fail(name, f"expected 0 JoinSession from _listen, got {len(join_msgs)}")
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_reconnect_sends_join_session(result):
    """On reconnect, _listen sends exactly one JoinSession."""
    name = "reconnect sends JoinSession"
    state = _reset_state()

    # first connection drops immediately, second stays alive briefly
    ws1 = MockWS(drop_after=0)
    ws2 = MockWS(drop_after=1)
    mock_connect = MockConnectContext([ws1, ws2])
    client = _make_client()
    _patch(client, mock_connect)

    _sleep_delays.clear()
    original_sleep = asyncio.sleep

    async def drive():
        # patch sleep inside the coroutine
        import blender_plugin.websocket_client as ws_mod
        original_asyncio = asyncio

        async def fast_sleep(delay):
            _sleep_delays.append(delay)
            await original_sleep(0)

        # monkey-patch asyncio.sleep used in wait_for and _listen
        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            task = asyncio.create_task(client._listen())
            # give it time to connect, drop, retry, reconnect
            for _ in range(20):
                await original_sleep(0.01)
            client.running = False
            state.intentional_disconnect = True
            try:
                await asyncio.wait_for(task, timeout=1.0)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                pass
        finally:
            ws_mod.asyncio = original_asyncio

    try:
        asyncio.run(drive())
        # ws2 is the reconnect — should have exactly 1 JoinSession
        join_msgs = [m for m in ws2.sent if "JoinSession" in m]
        if len(join_msgs) == 1:
            parsed = json.loads(join_msgs[0])
            payload = parsed["payload"]
            if payload["session_id"] == "test-room" and payload["display_name"] == "Tester":
                result.ok(name)
            else:
                result.fail(name, f"wrong JoinSession payload: {payload}")
        else:
            result.fail(name, f"expected 1 JoinSession on ws2, got {len(join_msgs)}")
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_reconnect_delays_are_3_9_27(result):
    """Retry delays follow RECONNECT_DELAYS = [3, 9, 27]."""
    name = "reconnect delays are 3, 9, 27"
    state = _reset_state()

    # all connections fail — 3 retries, all refused
    mock_connect = MockConnectContext([])  # no connections available
    client = _make_client()
    _patch(client, mock_connect)

    recorded_delays = []
    original_sleep = asyncio.sleep

    async def drive():
        import blender_plugin.websocket_client as ws_mod

        async def fast_sleep(delay):
            recorded_delays.append(delay)
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            await client._listen()
        finally:
            ws_mod.asyncio = asyncio

    try:
        asyncio.run(drive())
        if recorded_delays == [3, 9, 27]:
            result.ok(name)
        else:
            result.fail(name, f"expected [3, 9, 27], got {recorded_delays}")
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_intentional_disconnect_skips_retry(result):
    """If intentional_disconnect is set, no retries happen."""
    name = "intentional disconnect skips retry"
    state = _reset_state()
    state.intentional_disconnect = True

    ws1 = MockWS(drop_after=0)
    mock_connect = MockConnectContext([ws1])
    client = _make_client()
    _patch(client, mock_connect)

    recorded_delays = []

    async def drive():
        import blender_plugin.websocket_client as ws_mod
        original_sleep = asyncio.sleep

        async def fast_sleep(delay):
            recorded_delays.append(delay)
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            await client._listen()
        finally:
            ws_mod.asyncio = asyncio

    try:
        asyncio.run(drive())
        if len(recorded_delays) == 0:
            result.ok(name)
        else:
            result.fail(name, f"expected 0 retry sleeps, got {recorded_delays}")
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_all_retries_exhausted(result):
    """After 3 failed retries, state.connected = False and state.reconnecting = False."""
    name = "all retries exhausted sets connected=False"
    state = _reset_state()

    mock_connect = MockConnectContext([])  # all connections fail
    client = _make_client()
    _patch(client, mock_connect)

    async def drive():
        import blender_plugin.websocket_client as ws_mod
        original_sleep = asyncio.sleep

        async def fast_sleep(delay):
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            await client._listen()
        finally:
            ws_mod.asyncio = asyncio

    try:
        asyncio.run(drive())
        errors = []
        if state.connected:
            errors.append("state.connected should be False")
        if state.reconnecting:
            errors.append("state.reconnecting should be False")
        if state.reconnect_attempt != 0:
            errors.append(f"state.reconnect_attempt should be 0, got {state.reconnect_attempt}")

        if not errors:
            result.ok(name)
        else:
            result.fail(name, "; ".join(errors))
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_reconnect_resets_retry_index(result):
    """After a successful reconnect, retry_index resets so future drops get fresh retries."""
    name = "successful reconnect resets retry index"
    state = _reset_state()

    # connection 1 drops, connection 2 succeeds then drops, connection 3 fails
    ws1 = MockWS(drop_after=0)
    ws2 = MockWS(drop_after=1)  # succeeds briefly, then drops
    # after ws2 drops, it should retry from index 0 again (3s, 9s, 27s)
    mock_connect = MockConnectContext([ws1, ws2])
    client = _make_client()
    _patch(client, mock_connect)

    recorded_delays = []

    async def drive():
        import blender_plugin.websocket_client as ws_mod
        original_sleep = asyncio.sleep

        async def fast_sleep(delay):
            recorded_delays.append(delay)
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            await client._listen()
        finally:
            ws_mod.asyncio = asyncio

    try:
        asyncio.run(drive())
        # first drop: sleep(3) then reconnect to ws2 succeeds
        # ws2 drops: retry_index should be reset to 0, so sleep(3) again
        # then 9, then 27 (all fail, no more mock connections)
        # expected: [3, 3, 9, 27]
        if len(recorded_delays) >= 2 and recorded_delays[0] == 3 and recorded_delays[1] == 3:
            result.ok(name)
        else:
            result.fail(name, f"expected retry reset (starts with [3, 3, ...]), got {recorded_delays}")
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_reconnect_only_one_join_per_reconnect(result):
    """Each successful reconnect sends exactly one JoinSession, not duplicates."""
    name = "only one JoinSession per reconnect"
    state = _reset_state()

    # drop, reconnect, drop again, reconnect again
    ws1 = MockWS(drop_after=0)
    ws2 = MockWS(drop_after=1)
    ws3 = MockWS(drop_after=1)
    mock_connect = MockConnectContext([ws1, ws2, ws3])
    client = _make_client()
    _patch(client, mock_connect)

    async def drive():
        import blender_plugin.websocket_client as ws_mod
        original_sleep = asyncio.sleep

        async def fast_sleep(delay):
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            task = asyncio.create_task(client._listen())
            for _ in range(200):
                await original_sleep(0.01)
            client.running = False
            state.intentional_disconnect = True
            try:
                await asyncio.wait_for(task, timeout=2.0)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                pass
        finally:
            ws_mod.asyncio = asyncio

    try:
        asyncio.run(drive())
        # ws1 is initial connect — no JoinSession from _listen
        ws1_joins = [m for m in ws1.sent if "JoinSession" in m]
        # ws2 is first reconnect — exactly 1
        ws2_joins = [m for m in ws2.sent if "JoinSession" in m]
        # ws3 is second reconnect — exactly 1
        ws3_joins = [m for m in ws3.sent if "JoinSession" in m]

        errors = []
        if len(ws1_joins) != 0:
            errors.append(f"ws1 (initial): expected 0 JoinSession, got {len(ws1_joins)}")
        if len(ws2_joins) != 1:
            errors.append(f"ws2 (reconnect 1): expected 1 JoinSession, got {len(ws2_joins)}")
        if len(ws3_joins) != 1:
            errors.append(f"ws3 (reconnect 2): expected 1 JoinSession, got {len(ws3_joins)}")

        if not errors:
            result.ok(name)
        else:
            result.fail(name, "; ".join(errors))
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_eviction_close_code_sets_client_evicted(result):
    """Close code 4008 should be captured and exposed as an eviction."""
    name = "eviction close code sets is_evicted"
    state = _reset_state()

    ws1 = MockWS(drop_after=0, close_code=EVICTED_CLOSE_CODE, close_reason="lagging")
    mock_connect = MockConnectContext([ws1])
    client = _make_client()
    _patch(client, mock_connect)

    async def drive():
        import blender_plugin.websocket_client as ws_mod
        original_sleep = asyncio.sleep

        async def fast_sleep(delay):
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            # Stop retries so test exits quickly after first close
            task = asyncio.create_task(client._listen())
            await original_sleep(0.01)
            state.intentional_disconnect = True
            client.running = False
            try:
                await asyncio.wait_for(task, timeout=1.0)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                pass
        finally:
            ws_mod.asyncio = asyncio

    try:
        asyncio.run(drive())
        errors = []
        if client.last_close_code != EVICTED_CLOSE_CODE:
            errors.append(f"expected close code {EVICTED_CLOSE_CODE}, got {client.last_close_code}")
        if not client.is_evicted():
            errors.append("is_evicted() should be True")
        if not state.evicted:
            errors.append("state.evicted should be True")

        if not errors:
            result.ok(name)
        else:
            result.fail(name, "; ".join(errors))
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()


def test_connection_close_marks_state_disconnected_during_retry(result):
    """Any connection close should set connected=False before retry loop."""
    name = "close sets connected false before retry"
    state = _reset_state()

    ws1 = MockWS(drop_after=0, close_code=1006, close_reason="abnormal")
    ws2 = MockWS(drop_after=1)
    mock_connect = MockConnectContext([ws1, ws2])
    client = _make_client()
    _patch(client, mock_connect)

    async def drive():
        import blender_plugin.websocket_client as ws_mod
        original_sleep = asyncio.sleep

        async def fast_sleep(delay):
            await original_sleep(0)

        ws_mod.asyncio = type(sys)("fake_asyncio")
        ws_mod.asyncio.__dict__.update(asyncio.__dict__)
        ws_mod.asyncio.sleep = fast_sleep

        try:
            task = asyncio.create_task(client._listen())
            await original_sleep(0.02)
            disconnected_during_retry = (not state.connected)
            client.running = False
            state.intentional_disconnect = True
            try:
                await asyncio.wait_for(task, timeout=1.0)
            except (asyncio.TimeoutError, asyncio.CancelledError):
                pass
            return disconnected_during_retry
        finally:
            ws_mod.asyncio = asyncio

    try:
        disconnected_during_retry = asyncio.run(drive())
        errors = []
        if not disconnected_during_retry:
            errors.append("state.connected should become False after close")
        if not state.reconnecting:
            errors.append("state.reconnecting should be True during retry")
        if state.evicted:
            errors.append("state.evicted should remain False for non-4008 close")

        if not errors:
            result.ok(name)
        else:
            result.fail(name, "; ".join(errors))
    except Exception as e:
        result.fail(name, str(e))
    finally:
        _unpatch()
