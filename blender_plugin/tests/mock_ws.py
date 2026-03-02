"""Mock WebSocket client that captures outgoing messages for testing."""
import queue


class MockWebSocketClient:
    def __init__(self):
        self.sent_messages = []
        self.incoming = queue.Queue()
        self.connected = True

    def send(self, message):
        self.sent_messages.append(message)

    def disconnect(self):
        self.connected = False

    def connect(self):
        self.connected = True

    def clear(self):
        self.sent_messages.clear()
        while not self.incoming.empty():
            self.incoming.get_nowait()

    def get_sent(self, event_type=None):
        """Return sent messages, optionally filtered by event_type."""
        if event_type is None:
            return self.sent_messages
        return [m for m in self.sent_messages if m.get("event_type") == event_type]
