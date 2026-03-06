"""Simple in-memory metrics for request counting per service."""

import threading
import time


class RequestCounter:
    """Thread-safe per-service request counter."""

    def __init__(self) -> None:
        self._counts: dict[str, int] = {}
        self._lock = threading.Lock()
        self._start_time = time.monotonic()

    def increment(self, service: str) -> None:
        with self._lock:
            self._counts[service] = self._counts.get(service, 0) + 1

    def get(self, service: str) -> int:
        with self._lock:
            return self._counts.get(service, 0)

    def get_all(self) -> dict[str, int]:
        with self._lock:
            return dict(self._counts)

    def reset(self) -> None:
        with self._lock:
            self._counts.clear()
            self._start_time = time.monotonic()

    @property
    def uptime_seconds(self) -> float:
        return time.monotonic() - self._start_time


# Global singleton
request_counter = RequestCounter()
