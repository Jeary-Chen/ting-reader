import { create } from 'zustand';
import type { User } from '../types';
import { safeStorage } from '../utils/storage';
import {
  clearAuthCookie,
  clearSessionRestoreMarkers,
  persistAuthCookie,
} from '../utils/sessionRestore';
import { getRuntimeBaseUrl } from '../utils/runtimeUrl';

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
  user: JSON.parse(safeStorage.getItem('user') || 'null'),
  token: storedToken,
  serverUrl: safeStorage.getItem('server_url') || getRuntimeBaseUrl(),
  activeUrl: safeStorage.getItem('active_url') || safeStorage.getItem('server_url') || getRuntimeBaseUrl(),
  isAuthenticated: !!storedToken,
  setAuth: (user, token) => {
    safeStorage.setItem('auth_token', token);
    safeStorage.setItem('user', JSON.stringify(user));
    persistAuthCookie(token);
    set({ user, token, isAuthenticated: true });
  },
  setUser: (user) => {
    safeStorage.setItem('user', JSON.stringify(user));
    set({ user });
  },
  setToken: (token) => {
    safeStorage.setItem('auth_token', token);
    persistAuthCookie(token);
    set({ token, isAuthenticated: true });
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

