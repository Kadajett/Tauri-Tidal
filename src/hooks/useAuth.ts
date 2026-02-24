import { useCallback, useRef } from "react";
import { useAuthStore } from "@/stores/authStore";
import * as tauri from "@/lib/tauri";

export function useAuth() {
  const setAuth = useAuthStore((s) => s.setAuth);
  const setChecking = useAuthStore((s) => s.setChecking);
  const setLoginPending = useAuthStore((s) => s.setLoginPending);
  const setLoginError = useAuthStore((s) => s.setLoginError);
  const pollingRef = useRef(false);

  const checkAuth = useCallback(async () => {
    setChecking(true);
    try {
      const status = await tauri.checkAuthStatus();
      setAuth(
        status.authenticated,
        status.userId ?? null,
        status.displayName ?? null,
        status.countryCode,
      );
    } catch (err) {
      console.error("Auth check failed:", err);
      setAuth(false, null, null, "US");
    }
  }, [setAuth, setChecking]);

  const startLogin = useCallback(async () => {
    if (pollingRef.current) return;

    try {
      const deviceAuth = await tauri.login();

      setLoginPending(true, deviceAuth.userCode, deviceAuth.verificationUri);

      // Open the verification URL in the system browser
      let uri = deviceAuth.verificationUriComplete ?? deviceAuth.verificationUri;
      if (uri && !uri.startsWith("http")) {
        uri = `https://${uri}`;
      }
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(uri);

      // Start polling for authorization
      pollingRef.current = true;
      const interval = (deviceAuth.interval || 5) * 1000;
      const expiresAt = Date.now() + deviceAuth.expiresIn * 1000;

      const poll = async () => {
        if (!pollingRef.current) return;
        if (Date.now() > expiresAt) {
          pollingRef.current = false;
          setLoginError("Login expired. Please try again.");
          return;
        }

        try {
          const status = await tauri.pollLogin();
          if (status.authenticated) {
            pollingRef.current = false;
            setAuth(
              true,
              status.userId ?? null,
              status.displayName ?? null,
              status.countryCode,
            );
            return;
          }
          // Still pending, poll again after interval
          setTimeout(poll, interval);
        } catch (err) {
          pollingRef.current = false;
          const msg = err instanceof Error ? err.message : String(err);
          setLoginError(msg);
        }
      };

      setTimeout(poll, interval);
    } catch (err) {
      console.error("Login failed:", err);
      const msg = err instanceof Error ? err.message : String(err);
      setLoginError(msg);
    }
  }, [setAuth, setLoginPending, setLoginError]);

  const cancelLogin = useCallback(() => {
    pollingRef.current = false;
    setLoginPending(false);
  }, [setLoginPending]);

  const handleCallback = useCallback(
    async (code: string) => {
      try {
        const status = await tauri.handleAuthCallback(code);
        setAuth(
          status.authenticated,
          status.userId ?? null,
          status.displayName ?? null,
          status.countryCode,
        );
      } catch (err) {
        console.error("Auth callback failed:", err);
      }
    },
    [setAuth],
  );

  const handleLogout = useCallback(async () => {
    try {
      await tauri.logout();
      setAuth(false, null, null, "US");
    } catch (err) {
      console.error("Logout failed:", err);
    }
  }, [setAuth]);

  return { checkAuth, startLogin, cancelLogin, handleCallback, handleLogout };
}
