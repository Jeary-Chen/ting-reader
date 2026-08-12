
import i18n from '../i18n';
import { normalizeLanguage } from '../i18n/locales';
import { getApplicationTimeZone } from './timeZone';

const currentLocale = () => normalizeLanguage(i18n.resolvedLanguage || i18n.language);

export const parseBackendDate = (dateString: string | null | undefined): Date | null => {
  if (!dateString) return null;
  try {
    let cleanDate = dateString.trim();
    
    // 2. Fix the ".3f" suffix issue from previous bad backend version
    // "2026-02-23T09:50:42.3fZ" -> "2026-02-23T09:50:42.300Z" (approximation)
    // Actually we just want to remove the 'f' and ensure Z is there
    if (cleanDate.includes('.3fZ')) {
       cleanDate = cleanDate.replace('.3fZ', '.000Z');
    }

    // 3. Handle SQL format "YYYY-MM-DD HH:MM:SS" -> "YYYY-MM-DDTHH:MM:SS"
    // Only replace the space between date and time (index 10)
    if (cleanDate.charAt(10) === ' ') {
      cleanDate = cleanDate.substring(0, 10) + 'T' + cleanDate.substring(11);
    }
    
    // 4. Truncate nanoseconds/microseconds to milliseconds
    // Example: .123456 -> .123
    // We look for a dot followed by more than 3 digits
    cleanDate = cleanDate.replace(/(\.\d{3})\d+/, '$1');

    if (/^\d{4}-\d{2}-\d{2}$/.test(cleanDate)) {
      cleanDate = `${cleanDate}T00:00:00Z`;
    } else if (!/(?:Z|[+-]\d{2}:?\d{2})$/i.test(cleanDate)) {
      // SQLite CURRENT_TIMESTAMP values have no offset but are always UTC.
      cleanDate = `${cleanDate}Z`;
    }

    const date = new Date(cleanDate);
    if (isNaN(date.getTime())) {
      console.warn(`解析日期失败: ${dateString} (cleaned: ${cleanDate})`);
      return null;
    }

    return date;
  } catch {
    return null;
  }
};

export const formatDate = (dateString: string | null | undefined): string => {
  const date = parseBackendDate(dateString);
  if (!date) return i18n.t('common.unknownTime');

  try {
    return new Intl.DateTimeFormat(currentLocale(), {
        timeZone: getApplicationTimeZone(),
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false
    }).format(date);
  } catch {
    return i18n.t('common.unknownTime');
  }
};
