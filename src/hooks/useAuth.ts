import { useCallback } from "react";
import { useAuthStore } from "@/stores/authStore";
import * as tauri from "@/lib/tauri";

export function useAuth() {
  const setAuth = useAuthStore((s) => s.setAuth);
  const setChecking = useAuthStore((s) => s.setChecking);

  const checkAuth = useCallback(async () => {
    setChecking(true);
    try {
      const status = await tauri.checkAuthStatus();
      setAuth(
        status.authenticated,
        status.userId ?? null,
        status.countryCode,
      );
    } catch (err) {
      console.error("Auth check failed:", err);
      setAuth(false, null, "US");
    }
  }, [setAuth, setChecking]);

  const startLogin = useCallback(async () => {
    try {
      const authUrl = await tauri.login();
      // Open the Tidal login page in the system browser
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(authUrl);
    } catch (err) {
      console.error("Login failed:", err);
    }
  }, []);

  const handleCallback = useCallback(
    async (code: string) => {
      try {
        const status = await tauri.handleAuthCallback(code);
        setAuth(
          status.authenticated,
          status.userId ?? null,
          status.countryCode,
        );
      } catch (err) {
        console.error("Auth callback failed:", err);
      }
    },
    [setAuth],
  );

  return { checkAuth, startLogin, handleCallback };
}
