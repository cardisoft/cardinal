import React, { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { getTooltipAttributes } from '../utils/tooltip';
import { pickFolder } from '../utils/pickFolder';

export type SearchFiltersBarAction =
  | { kind: 'append-token'; token: string }
  | { kind: 'merge-extension'; extension: string }
  | { kind: 'replace-family'; family: 'kind' | 'type' | 'date' | 'size' | 'scope'; token: string };

export type SearchFiltersBarProps = {
  onApplyAction: (action: SearchFiltersBarAction) => void;
  onOpenHelp: () => void;
  helpButtonLabel: string;
};

type FilterOption = {
  label: string;
  value: string;
};

type FilterSelectProps = {
  ariaLabel: string;
  tooltip: string;
  options: FilterOption[];
  onChange: (event: React.ChangeEvent<HTMLSelectElement>) => void;
};

const FilterSelect = ({
  ariaLabel,
  tooltip,
  options,
  onChange,
}: FilterSelectProps): React.JSX.Element => (
  <div className="search-filter-select-wrap">
    <select
      className="search-filter-select"
      defaultValue=""
      aria-label={ariaLabel}
      {...getTooltipAttributes(tooltip)}
      onChange={onChange}
    >
      <option value="">{ariaLabel}</option>
      {options.map((option) => (
        <option key={option.value} value={option.value}>
          {option.label}
        </option>
      ))}
    </select>
    <span className="search-filter-select-caret" aria-hidden="true">
      ▼
    </span>
  </div>
);

const quoteFilterArgument = (value: string): string => {
  if (!/[\s";]/.test(value)) {
    return value;
  }

  const escapedValue = value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return `"${escapedValue}"`;
};

export const SearchFiltersBar = ({
  onApplyAction,
  onOpenHelp,
  helpButtonLabel,
}: SearchFiltersBarProps): React.JSX.Element => {
  const { t } = useTranslation();
  const inFolderTooltip = t('search.filterBar.tooltips.scope.in');
  const parentFolderTooltip = t('search.filterBar.tooltips.scope.parent');
  const kindTooltip = t('search.filterBar.tooltips.kind');
  const typeTooltip = t('search.filterBar.tooltips.type');
  const extensionTooltip = t('search.filterBar.tooltips.extension');
  const dateTooltip = t('search.filterBar.tooltips.date');
  const sizeTooltip = t('search.filterBar.tooltips.size');
  const regexTooltip = t('search.filterBar.tooltips.regexExample');

  const kindOptions: FilterOption[] = [
    { label: t('search.filterBar.kind.files'), value: 'file:' },
    { label: t('search.filterBar.kind.folders'), value: 'folder:' },
  ];
  const typeOptions: FilterOption[] = [
    { label: t('search.filterBar.type.picture'), value: 'type:picture' },
    { label: t('search.filterBar.type.video'), value: 'type:video' },
    { label: t('search.filterBar.type.audio'), value: 'type:audio' },
    { label: t('search.filterBar.type.document'), value: 'type:doc' },
    { label: t('search.filterBar.type.presentation'), value: 'type:presentation' },
    { label: t('search.filterBar.type.spreadsheet'), value: 'type:spreadsheet' },
    { label: t('search.filterBar.type.pdf'), value: 'type:pdf' },
    { label: t('search.filterBar.type.archive'), value: 'type:archive' },
    { label: t('search.filterBar.type.code'), value: 'type:code' },
    { label: t('search.filterBar.type.executable'), value: 'type:exe' },
  ];
  const extensionOptions: FilterOption[] = [
    { label: 'PDF', value: 'pdf' },
    { label: 'MD', value: 'md' },
    { label: 'TXT', value: 'txt' },
    { label: 'JSON', value: 'json' },
    { label: 'TS', value: 'ts' },
    { label: 'TSX', value: 'tsx' },
    { label: 'RS', value: 'rs' },
    { label: 'JPG', value: 'jpg' },
    { label: 'PNG', value: 'png' },
  ];
  const dateOptions: FilterOption[] = [
    { label: t('search.filterBar.date.modifiedToday'), value: 'dm:today' },
    { label: t('search.filterBar.date.modifiedThisWeek'), value: 'dm:thisweek' },
    { label: t('search.filterBar.date.modifiedLastMonth'), value: 'dm:lastmonth' },
    { label: t('search.filterBar.date.createdToday'), value: 'dc:today' },
    { label: t('search.filterBar.date.createdThisWeek'), value: 'dc:thisweek' },
    { label: t('search.filterBar.date.createdLastMonth'), value: 'dc:lastmonth' },
  ];
  const sizeOptions: FilterOption[] = [
    { label: t('search.filterBar.size.empty'), value: 'size:empty' },
    { label: t('search.filterBar.size.tiny'), value: 'size:tiny' },
    { label: t('search.filterBar.size.small'), value: 'size:small' },
    { label: t('search.filterBar.size.medium'), value: 'size:medium' },
    { label: t('search.filterBar.size.large'), value: 'size:large' },
    { label: t('search.filterBar.size.huge'), value: 'size:huge' },
    { label: t('search.filterBar.size.rangeOneToTenMb'), value: 'size:1mb..10mb' },
    { label: t('search.filterBar.size.overHundredMb'), value: 'size:>100mb' },
  ];

  const handleScopePick = useCallback(
    async (scopePrefix: 'in' | 'parent') => {
      const selectedPath = await pickFolder();
      if (!selectedPath) {
        return;
      }

      const quotedPath = quoteFilterArgument(selectedPath);
      onApplyAction({
        kind: 'replace-family',
        family: 'scope',
        token: `${scopePrefix}:${quotedPath}`,
      });
    },
    [onApplyAction],
  );

  const handleReplaceSelection = useCallback(
    (event: React.ChangeEvent<HTMLSelectElement>, family: 'kind' | 'type' | 'date' | 'size') => {
      const selectedToken = event.target.value;
      event.target.value = '';
      if (!selectedToken) {
        return;
      }

      onApplyAction({ kind: 'replace-family', family, token: selectedToken });
    },
    [onApplyAction],
  );

  const handleExtensionSelection = useCallback(
    (event: React.ChangeEvent<HTMLSelectElement>) => {
      const selectedExtension = event.target.value;
      event.target.value = '';
      if (!selectedExtension) {
        return;
      }

      onApplyAction({ kind: 'merge-extension', extension: selectedExtension });
    },
    [onApplyAction],
  );

  return (
    <div className="search-filters-bar" aria-label={t('search.filterBar.ariaLabel')}>
      <div className="search-filters-group">
        <button
          className="search-filter-button"
          type="button"
          {...getTooltipAttributes(inFolderTooltip)}
          onClick={() => void handleScopePick('in')}
        >
          {t('search.filterBar.scope.in')}
        </button>
        <button
          className="search-filter-button"
          type="button"
          {...getTooltipAttributes(parentFolderTooltip)}
          onClick={() => void handleScopePick('parent')}
        >
          {t('search.filterBar.scope.parent')}
        </button>
      </div>

      <span className="search-filters-separator" aria-hidden="true" />

      <div className="search-filters-group">
        <FilterSelect
          ariaLabel={t('search.filterBar.kind.label')}
          tooltip={kindTooltip}
          options={kindOptions}
          onChange={(event) => handleReplaceSelection(event, 'kind')}
        />
        <FilterSelect
          ariaLabel={t('search.filterBar.type.label')}
          tooltip={typeTooltip}
          options={typeOptions}
          onChange={(event) => handleReplaceSelection(event, 'type')}
        />
        <FilterSelect
          ariaLabel={t('search.filterBar.extension.label')}
          tooltip={extensionTooltip}
          options={extensionOptions}
          onChange={handleExtensionSelection}
        />
      </div>

      <span className="search-filters-separator" aria-hidden="true" />

      <div className="search-filters-group">
        <FilterSelect
          ariaLabel={t('search.filterBar.date.label')}
          tooltip={dateTooltip}
          options={dateOptions}
          onChange={(event) => handleReplaceSelection(event, 'date')}
        />
        <FilterSelect
          ariaLabel={t('search.filterBar.size.label')}
          tooltip={sizeTooltip}
          options={sizeOptions}
          onChange={(event) => handleReplaceSelection(event, 'size')}
        />
      </div>

      <span className="search-filters-separator" aria-hidden="true" />

      <div className="search-filters-group">
        <button
          className="search-filter-button search-filter-button--example"
          type="button"
          {...getTooltipAttributes(regexTooltip)}
          onClick={() => onApplyAction({ kind: 'append-token', token: 'regex:^README\\.md$' })}
        >
          {t('search.filterBar.regexExample')}
        </button>
      </div>

      <div className="search-filters-help-slot">
        <button
          className="search-help-trigger search-help-trigger--filters search-help-trigger--text"
          type="button"
          aria-label={helpButtonLabel}
          {...getTooltipAttributes(helpButtonLabel)}
          onClick={onOpenHelp}
        >
          Info
        </button>
      </div>
    </div>
  );
};
