// Player-specific platform helpers (browser / path detection)

export const isAppleMobileBrowser = (): boolean => {
  if (typeof navigator === 'undefined') return false;
  const ua = navigator.userAgent || '';
  const isiPhoneOrIPad = /iPad|iPhone|iPod/.test(ua);
  const isModernIPad = navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1;
  return isiPhoneOrIPad || isModernIPad;
};

export const isStrmPath = (path?: string): boolean =>
  path?.toLowerCase().split('?')[0].endsWith('.strm') ?? false;

const mineSectionPathPrefixes = [
  '/mine',
  '/history',
  '/favorites',
  '/personalization',
  '/cache',
  '/notifications',
  '/statistics',
  '/about',
];

const alwaysHiddenMiniPlayerPathPrefixes = [
  '/admin',
  '/settings',
  '/plugin-pages',
];

export const isMineSectionPath = (pathname: string): boolean =>
  mineSectionPathPrefixes.some((path) => pathname.startsWith(path));

export const isMiniPlayerHiddenPath = (pathname: string): boolean =>
  isMineSectionPath(pathname) ||
  alwaysHiddenMiniPlayerPathPrefixes.some((path) => pathname.startsWith(path));
