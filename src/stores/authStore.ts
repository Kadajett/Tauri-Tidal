import { create } from "zustand";

interface AuthState {
  authenticated: boolean;
  userId: string | null;
  countryCode: string;
  checking: boolean;
  setAuth: (authenticated: boolean, userId: string | null, countryCode: string) => void;
  setChecking: (checking: boolean) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  authenticated: false,
  userId: null,
  countryCode: "US",
  checking: true,
  setAuth: (authenticated, userId, countryCode) =>
    set({ authenticated, userId, countryCode, checking: false }),
  setChecking: (checking) => set({ checking }),
}));
