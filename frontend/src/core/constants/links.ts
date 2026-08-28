export const TING_READER_WEBSITE_URL = "https://www.tingreader.cn";

const isEnglish = (language?: string) =>
  language?.toLowerCase().startsWith("en") === true;

export const getLocalizedTingReaderWebsiteUrl = (language?: string) =>
  isEnglish(language)
    ? `${TING_READER_WEBSITE_URL}/en`
    : TING_READER_WEBSITE_URL;

const getLocalizedDocumentUrl = (path: string, language?: string) =>
  `${TING_READER_WEBSITE_URL}${path}${isEnglish(language) ? "/en" : ""}`;

export const getLocalizedUserAgreementUrl = (language?: string) =>
  getLocalizedDocumentUrl("/about/user-agreement", language);

export const getLocalizedPrivacyPolicyUrl = (language?: string) =>
  getLocalizedDocumentUrl("/about/privacy-policy", language);

export const getLocalizedChangelogUrl = (language?: string) =>
  getLocalizedDocumentUrl("/about/changelog", language);

export const getLocalizedUpdateGuideUrl = (language?: string) =>
  getLocalizedDocumentUrl("/guide/update", language);
