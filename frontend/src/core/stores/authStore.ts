import { create } from 'zustand';
import type { User } from '../types';
import { safeStorage } from '../utils/storage';
import {
  clearAuthCookie,
  clearSessionRestoreMarkers,
  persistAuthCookie,
} from '../utils/sessionRestore';
import { getRuntimeBaseUrl } from '../utils/runtimeUrl';

const isStoredUser = (value: unknown): value is User => {
  if (!value || typeof value !== 'object') return false;

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.id === 'string' &&
    typeof candidate.username === 'string' &&
    (candidate.role === 'admin' || candidate.role === 'user')
  );
};

const readStoredUser = (): User | null => {
  const rawUser = safeStorage.getItem('user');
  if (!rawUser || rawUser === 'null') return null;

  try {
    const parsedUser: unknown = JSON.parse(rawUser);
    if (parsedUser === null) return null;
    if (isStoredUser(parsedUser)) return parsedUser;
  } catch {
    // A previous failed login/session restore could have persisted the
    // string "undefined". Treat the cache as stale instead of preventing the
    // entire React application from mounting.
  }

  // Remove malformed or incompatible cached data so the next reload is clean.
  safeStorage.removeItem('user');
  return null;
};

const storedToken = safeStorage.getItem('auth_token');
if (storedToken) {
  persistAuthCookie(storedToken);
}

interface AuthState {
  user: User | null;
  token: string | null;
  serverUrl: string; // The original URL input by user
  activeUrl: string; // The resolved URL (after redirect)
  isAuthenticated: boolean;
  setAuth: (user: User, token: string) => void;
  setUser: (user: User) => void;
  setToken: (token: string) => void;
  setServerUrl: (url: string) => void;
  setActiveUrl: (url: string) => void;
  logout: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: readStoredUser(),
  token: storedToken,
  serverUrl: safeStorage.getItem('server_url') || getRuntimeBaseUrl(),
  activeUrl: safeStorage.getItem('active_url') || safeStorage.getItem('server_url') || getRuntimeBaseUrl(),
  isAuthenticated: !!storedToken,
  setAuth: (user, token) => {
    const validUser = isStoredUser(user) ? user : null;
    const validToken = typeof token === 'string' && token.length > 0 ? token : null;

    if (validToken) {
      safeStorage.setItem('auth_token', validToken);
      persistAuthCookie(validToken);
    } else {
      safeStorage.removeItem('auth_token');
    }

    if (validUser) {
      safeStorage.setItem('user', JSON.stringify(validUser));
    } else {
      safeStorage.removeItem('user');
    }

    set({ user: validUser, token: validToken, isAuthenticated: !!validToken });
  },
  setUser: (user) => {
    const validUser = isStoredUser(user) ? user : null;
    if (validUser) {
      safeStorage.setItem('user', JSON.stringify(validUser));
    } else {
      safeStorage.removeItem('user');
    }
    set({ user: validUser });
  },
  setToken: (token) => {
    const validToken = typeof token === 'string' && token.length > 0 ? token : null;
    if (validToken) {
      safeStorage.setItem('auth_token', validToken);
      persistAuthCookie(validToken);
    } else {
      safeStorage.removeItem('auth_token');
    }
    set({ token: validToken, isAuthenticated: !!validToken });
  },
  setServerUrl: (url) => {
    safeStorage.setItem('server_url', url);
    set({ serverUrl: url });
  },
  setActiveUrl: (url) => {
    safeStorage.setItem('active_url', url);
    set({ activeUrl: url });
  },
  logout: () => {
    safeStorage.removeItem('auth_token');
    safeStorage.removeItem('user');
    clearAuthCookie();
    clearSessionRestoreMarkers();
    set({ user: null, token: null, isAuthenticated: false });
  },
}));

