import { useEffect, useState } from 'react';

/**
 * Returns true after the browser has had a chance to prioritize initial work (e.g. JS chunks).
 * Use with TanStack Query's `enabled` so non-critical API requests don't compete with script loading.
 */
export function useDeferredReady(): boolean {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const id =
      typeof requestIdleCallback !== 'undefined'
        ? requestIdleCallback(() => setReady(true), { timeout: 100 })
        : (setTimeout(() => setReady(true), 0) as unknown as number);

    return () => {
      if (typeof cancelIdleCallback !== 'undefined') {
        cancelIdleCallback(id as ReturnType<typeof requestIdleCallback>);
      } else {
        clearTimeout(id);
      }
    };
  }, []);

  return ready;
}
