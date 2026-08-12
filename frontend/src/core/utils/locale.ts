import i18n from '../i18n';
import { normalizeLanguage } from '../i18n/locales';
import { getApplicationTimeZone } from './timeZone';

export const getCurrentLocale = () => normalizeLanguage(i18n.resolvedLanguage || i18n.language);

export const localeCompare = (a: string, b: string) => a.localeCompare(b, getCurrentLocale());

export const formatLocalizedNumber = (value: number) => (
  new Intl.NumberFormat(getCurrentLocale()).format(value)
);

export const formatLocalizedDate = (
  value: Date,
  options: Intl.DateTimeFormatOptions,
) => new Intl.DateTimeFormat(getCurrentLocale(), {
  ...options,
  timeZone: getApplicationTimeZone(),
}).format(value);
