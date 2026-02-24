export interface AppError {
  kind: string;
  message: string;
}

export interface AuthStatus {
  authenticated: boolean;
  userId?: string;
  displayName?: string;
  countryCode: string;
}

export interface DeviceAuthResponse {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  verificationUriComplete?: string;
  expiresIn: number;
  interval: number;
}
