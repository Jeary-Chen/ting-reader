type JsonRecord = Record<string, unknown>;

type CollectionContract = {
  fields: readonly string[];
  requiredStringField: string;
};

const collectionContracts: Record<string, CollectionContract> = {
  '/api/progress/recent': {
    fields: ['progress', 'recent', 'items'],
    requiredStringField: 'book_id',
  },
  '/api/books': {
    fields: ['books', 'items'],
    requiredStringField: 'id',
  },
  '/api/favorites': {
    fields: ['books', 'favorites', 'items'],
    requiredStringField: 'id',
  },
  '/api/v1/series': {
    fields: ['series', 'items'],
    requiredStringField: 'id',
  },
  '/api/playlists': {
    fields: ['playlists', 'items'],
    requiredStringField: 'id',
  },
};

const envelopeFields = ['data', 'result'] as const;

const isJsonRecord = (value: unknown): value is JsonRecord =>
  value !== null && typeof value === 'object' && !Array.isArray(value);

const getRequestPath = (url?: string) => {
  if (!url) return '';

  try {
    return new URL(url, window.location.origin).pathname;
  } catch {
    return url.split(/[?#]/, 1)[0];
  }
};

const findCollection = (
  payload: unknown,
  fields: readonly string[],
): unknown[] | undefined => {
  const queue: unknown[] = [payload];
  const visited = new Set<object>();

  for (let index = 0; index < queue.length && index < 16; index += 1) {
    const value = queue[index];
    if (Array.isArray(value)) return value;
    if (!isJsonRecord(value) || visited.has(value)) continue;

    visited.add(value);
    for (const field of [...fields, ...envelopeFields]) {
      if (field in value) queue.push(value[field]);
    }
  }

  return undefined;
};

export const normalizeKnownCollectionResponse = (
  url: string | undefined,
  payload: unknown,
): unknown => {
  const requestPath = getRequestPath(url);
  const endpoint = Object.keys(collectionContracts).find(
    (path) => requestPath === path || requestPath.endsWith(path),
  );
  if (!endpoint) return payload;

  const contract = collectionContracts[endpoint];
  const values = findCollection(payload, contract.fields);
  if (!values) {
    console.warn(`Expected an array response from ${endpoint}; using an empty list instead`);
    return [];
  }

  const normalized = values.filter(
    (value): value is JsonRecord =>
      isJsonRecord(value) &&
      typeof value[contract.requiredStringField] === 'string',
  );

  if (normalized.length !== values.length) {
    console.warn(
      `Ignored ${values.length - normalized.length} invalid item(s) from ${endpoint}`,
    );
  }

  return normalized;
};
