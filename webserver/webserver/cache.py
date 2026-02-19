"""
In-memory cache for timeline data with TTL support.

Provides a simple thread-safe cache implementation for storing
timeline responses from the Rust backend.
"""

from typing import Optional, Dict, Any, Tuple
from time import time
from threading import Lock


class TimelineCache:
    """
    Thread-safe in-memory cache with time-to-live (TTL) support.

    Stores timeline data to reduce load on the Rust analyzer backend.
    Cache entries automatically expire after their TTL.
    """

    def __init__(self):
        self._cache: Dict[str, Tuple[Any, float]] = {}
        self._lock = Lock()

    def get(self, key: str) -> Optional[Any]:
        """
        Retrieve value from cache if it exists and hasn't expired.

        Args:
            key: Cache key

        Returns:
            Cached value if found and not expired, None otherwise
        """
        with self._lock:
            if key in self._cache:
                value, expiry = self._cache[key]
                if time() < expiry:
                    return value
                else:
                    # Expired - remove from cache
                    del self._cache[key]
        return None

    def set(self, key: str, value: Any, ttl_seconds: int = 900) -> None:
        """
        Store value in cache with TTL.

        Args:
            key: Cache key
            value: Value to cache
            ttl_seconds: Time-to-live in seconds (default: 900 = 15 minutes)
        """
        with self._lock:
            self._cache[key] = (value, time() + ttl_seconds)

    def clear(self) -> None:
        """Clear all entries from cache."""
        with self._lock:
            self._cache.clear()

    def size(self) -> int:
        """Return number of entries in cache (including expired)."""
        with self._lock:
            return len(self._cache)

    def purge_expired(self) -> int:
        """
        Remove all expired entries from cache.

        Returns:
            Number of entries removed
        """
        current_time = time()
        with self._lock:
            expired_keys = [
                key for key, (_, expiry) in self._cache.items()
                if current_time >= expiry
            ]
            for key in expired_keys:
                del self._cache[key]
            return len(expired_keys)


# Global cache instance
timeline_cache = TimelineCache()
