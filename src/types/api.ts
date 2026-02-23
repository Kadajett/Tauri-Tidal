export interface AppError {
  kind: string;
  message: string;
}

export interface AuthStatus {
  authenticated: boolean;
  userId?: string;
  countryCode: string;
}
