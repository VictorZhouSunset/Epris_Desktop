import { useCallback } from 'react';

export function useWaitPort() {
  const waitPort = useCallback(async (url: string, timeout = 10000) => {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      try {
        const res = await fetch(url, { mode: 'no-cors' });
        if (res.status !== 0 || res.type === 'opaque') {
          // 'no-cors' fetch to local port usually returns status 0/opaque if server is listening
          return true;
        }
      } catch (e) {
        // Continue polling
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    return false;
  }, []);

  return { waitPort };
}
