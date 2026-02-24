import { create } from "zustand";

interface AuthState {
  authenticated: boolean;
  userId: string | null;
  displayName: string | null;
  countryCode: string;
  checking: boolean;
  loginPending: boolean;
  userCode: string | null;
  verificationUri: string | null;
  loginError: string | null;
  setAuth: (authenticated: boolean, userId: string | null, displayName: string | null, countryCode: string) => void;
  setChecking: (checking: boolean) => void;
  setLoginPending: (pending: boolean, userCode?: string | null, verificationUri?: string | null) => void;
  setLoginError: (error: string | null) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  authenticated: false,
  userId: null,
  displayName: null,
  countryCode: "US",
  checking: true,
  loginPending: false,
  userCode: null,
  verificationUri: null,
  loginError: null,
  setAuth: (authenticated, userId, displayName, countryCode) =>
    set({ authenticated, userId, displayName, countryCode, checking: false, loginPending: false, userCode: null, verificationUri: null, loginError: null }),
  setChecking: (checking) => set({ checking }),
  setLoginPending: (pending, userCode = null, verificationUri = null) =>
    set({ loginPending: pending, userCode, verificationUri, loginError: null }),
  setLoginError: (error) =>
    set({ loginError: error, loginPending: false, userCode: null, verificationUri: null }),
}));
