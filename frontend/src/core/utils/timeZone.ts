import { safeStorage } from './storage';
import { useSyncExternalStore } from 'react';

const STORAGE_KEY = 'ting-reader.application-time-zone';
const DEFAULT_TIME_ZONE = 'UTC';
const FALLBACK_TIME_ZONES = [
  'UTC',
  'Asia/Shanghai',
  'Asia/Tokyo',
  'Asia/Singapore',
  'Asia/Seoul',
  'Asia/Kolkata',
  'Australia/Sydney',
  'Europe/London',
  'Europe/Berlin',
  'America/New_York',
  'America/Chicago',
  'America/Denver',
  'America/Los_Angeles',
];

let applicationTimeZone = safeStorage.getItem(STORAGE_KEY) || DEFAULT_TIME_ZONE;
const listeners = new Set<() => void>();

const isValidTimeZone = (value: string) => {
  try {
    new Intl.DateTimeFormat('en-US', { timeZone: value }).format();
    return true;
  } catch {
    return false;
  }
};

export const getApplicationTimeZone = () => applicationTimeZone;

export const setApplicationTimeZone = (value: string) => {
  if (!isValidTimeZone(value)) return false;
  if (applicationTimeZone === value) return true;
  applicationTimeZone = value;
  safeStorage.setItem(STORAGE_KEY, value);
  listeners.forEach((listener) => listener());
  return true;
};

export const subscribeToApplicationTimeZone = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

export const useApplicationTimeZone = () => useSyncExternalStore(
  subscribeToApplicationTimeZone,
  getApplicationTimeZone,
  getApplicationTimeZone,
);

export const getCurrentHour = (date = new Date()) => {
  const parts = new Intl.DateTimeFormat('en-US', {
    hour: 'numeric',
    hourCycle: 'h23',
    timeZone: applicationTimeZone,
  }).formatToParts(date);
  const hour = Number(parts.find((part) => part.type === 'hour')?.value);
  return Number.isFinite(hour) ? hour : date.getHours();
};

const applicationDateParts = (date: Date) => {
  const parts = new Intl.DateTimeFormat('en-CA', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    timeZone: applicationTimeZone,
  }).formatToParts(date);
  const value = (type: Intl.DateTimeFormatPartTypes) => Number(
    parts.find((part) => part.type === type)?.value,
  );

  return { year: value('year'), month: value('month'), day: value('day') };
};

export const getApplicationDayDifference = (date: Date, reference = new Date()) => {
  const from = applicationDateParts(date);
  const to = applicationDateParts(reference);
  const fromDay = Date.UTC(from.year, from.month - 1, from.day);
  const toDay = Date.UTC(to.year, to.month - 1, to.day);
  return Math.round((toDay - fromDay) / 86400000);
};

export const formatInApplicationTimeZone = (
  date: Date,
  locale: string,
  options: Intl.DateTimeFormatOptions,
) => new Intl.DateTimeFormat(locale, {
  ...options,
  timeZone: applicationTimeZone,
}).format(date);

const supportedTimeZones = (
  Intl as typeof Intl & { supportedValuesOf?: (key: 'timeZone') => string[] }
).supportedValuesOf?.('timeZone') || FALLBACK_TIME_ZONES;

export const APPLICATION_TIME_ZONE_OPTIONS = Array.from(
  new Set([DEFAULT_TIME_ZONE, ...supportedTimeZones]),
).map((value) => ({ value, label: value.replace('_', ' ') }));
