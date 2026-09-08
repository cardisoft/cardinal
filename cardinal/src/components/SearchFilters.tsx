import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

type SearchFiltersProps = { query?: string; onChange: (query: string) => void };
type FilterState = { type: string; modified: string; size: string };
const EMPTY_FILTERS: FilterState = { type: '', modified: '', size: '' };

const composeFilterQuery = ({ type, modified, size }: FilterState): string =>
  [type && `type:${type}`, modified && `dm:${modified}`, size && `size:${size}`]
    .filter(Boolean)
    .join(' ');

const parseFilterQuery = (query = ''): FilterState => {
  const read = (prefix: string) =>
    query
      .split(/\s+/)
      .find((token) => token.startsWith(prefix))
      ?.slice(prefix.length) ?? '';
  return { type: read('type:'), modified: read('dm:'), size: read('size:') };
};

export function SearchFilters({ query = '', onChange }: SearchFiltersProps): React.JSX.Element {
  const { t } = useTranslation();
  const [filters, setFilters] = useState<FilterState>(() => parseFilterQuery(query));

  useEffect(() => {
    setFilters(parseFilterQuery(query));
  }, [query]);

  const updateFilter = (key: keyof FilterState, value: string): void => {
    const next = { ...filters, [key]: value };
    setFilters(next);
    onChange(composeFilterQuery(next));
  };
  const clearFilters = (): void => {
    setFilters(EMPTY_FILTERS);
    onChange('');
  };
  const hasFilters = Boolean(filters.type || filters.modified || filters.size);

  return (
    <div className="search-filters" aria-label={t('search.filters.label')}>
      <span className="search-filters__title">{t('search.filters.label')}</span>
      <label>
        <span>{t('search.filters.type')}</span>
        <select
          value={filters.type}
          onChange={(event) => updateFilter('type', event.target.value)}
          aria-label={t('search.filters.type')}
        >
          <option value="">{t('search.filters.anyType')}</option>
          <option value="folder">{t('search.filters.folders')}</option>
          <option value="picture">{t('search.filters.pictures')}</option>
          <option value="video">{t('search.filters.videos')}</option>
          <option value="audio">{t('search.filters.audio')}</option>
          <option value="doc">{t('search.filters.documents')}</option>
          <option value="pdf">PDF</option>
          <option value="archive">{t('search.filters.archives')}</option>
          <option value="code">{t('search.filters.code')}</option>
          <option value="app">{t('search.filters.apps')}</option>
        </select>
      </label>
      <label>
        <span>{t('search.filters.modified')}</span>
        <select
          value={filters.modified}
          onChange={(event) => updateFilter('modified', event.target.value)}
          aria-label={t('search.filters.modified')}
        >
          <option value="">{t('search.filters.anyTime')}</option>
          <option value="today">{t('search.filters.today')}</option>
          <option value="pastweek">{t('search.filters.pastWeek')}</option>
          <option value="pastmonth">{t('search.filters.pastMonth')}</option>
          <option value="pastyear">{t('search.filters.pastYear')}</option>
        </select>
      </label>
      <label>
        <span>{t('search.filters.size')}</span>
        <select
          value={filters.size}
          onChange={(event) => updateFilter('size', event.target.value)}
          aria-label={t('search.filters.size')}
        >
          <option value="">{t('search.filters.anySize')}</option>
          <option value=">10MB">10 MB+</option>
          <option value=">100MB">100 MB+</option>
          <option value=">1GB">1 GB+</option>
        </select>
      </label>
      <button type="button" onClick={clearFilters} disabled={!hasFilters}>
        {t('search.filters.clear')}
      </button>
    </div>
  );
}
