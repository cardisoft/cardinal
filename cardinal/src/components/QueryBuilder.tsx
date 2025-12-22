import React, { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-dialog';
import { parseQueryToBuilder } from '../utils/queryParser';
import './QueryBuilder.css';

export type SearchField =
  | 'name'
  | 'ext'
  | 'type'
  | 'size'
  | 'dm'
  | 'dc'
  | 'parent'
  | 'infolder'
  | 'nosubfolders'
  | 'content'
  | 'regex'
  | 'tag'
  | 'kind';

export type SearchOperator = 'contains' | 'is' | 'gt' | 'lt';

export interface FilterRule {
  id: string;
  type: 'rule';
  field: SearchField;
  operator: SearchOperator;
  value: string;
  exclude: boolean;
}

export interface RuleGroup {
  id: string;
  type: 'group';
  combinator: 'and' | 'or';
  items: SearchItem[];
  exclude?: boolean;
}

export type SearchItem = FilterRule | RuleGroup;

type QueryBuilderProps = {
  onApplyQuery: (query: string) => void;
  onClose: () => void;
  initialQuery?: string;
};

const INITIAL_ROOT: RuleGroup = {
  id: 'root',
  type: 'group',
  combinator: 'and',
  items: [],
  exclude: false,
};

export function QueryBuilder({
  onApplyQuery,
  onClose,
  initialQuery,
}: QueryBuilderProps): React.JSX.Element {
  const { t } = useTranslation();
  const [rootGroup, setRootGroup] = useState<RuleGroup>(INITIAL_ROOT);
  const [parseWarning, setParseWarning] = useState<string | null>(null);

  // Parse initial query on mount
  useEffect(() => {
    if (initialQuery && initialQuery.trim()) {
      parseQueryToBuilder(initialQuery)
        .then((parsed) => {
          if (parsed) {
            setRootGroup(parsed);
            setParseWarning(null);
          } else {
            setParseWarning(
              t('queryBuilder.parseWarning', 'Could not parse query. Starting with empty builder.'),
            );
          }
        })
        .catch((err) => {
          console.error('Parse error:', err);
          setParseWarning(t('queryBuilder.parseError', 'Failed to parse query.'));
        });
    }
  }, [initialQuery, t]);

  // Recursive query generation
  const processGroup = useCallback((group: RuleGroup): string => {
    const parts = group.items
      .map((item) => {
        if (item.type === 'group') {
          const groupQuery = processGroup(item);
          return groupQuery ? `(${groupQuery})` : '';
        } else {
          return generateRuleString(item);
        }
      })
      .filter((part) => part !== '');

    if (parts.length === 0) return '';

    let result = '';
    if (group.combinator === 'and') {
      result = parts.join(' ');
    } else {
      // OR logic: (A) | (B)
      result = parts
        .map((p) => {
          if (p.startsWith('(') && p.endsWith(')')) return p;
          if (p.includes(' ')) return `(${p})`;
          return p;
        })
        .join(' | ');
    }

    // Handle Group Negation: !(...)
    if (group.exclude) {
      return `!(${result})`;
    }

    return result;
  }, []);

  const generateRuleString = (rule: FilterRule): string => {
    const { field, value, exclude } = rule;
    if (!value) return '';

    const cleanValue = value.trim();
    const prefix = exclude ? '!' : '';
    let term = '';

    switch (field) {
      case 'name':
        return exclude ? `!${cleanValue}` : cleanValue;
      case 'ext':
        term = `${prefix}ext:${cleanValue}`;
        break;
      case 'type':
        term = `${prefix}type:${cleanValue}`;
        break;
      case 'size':
        term = `${prefix}size:${cleanValue}`;
        break;
      case 'dm':
        term = `${prefix}dm:${cleanValue}`;
        break;
      case 'dc':
        term = `${prefix}dc:${cleanValue}`;
        break;
      case 'parent':
        term = `${prefix}parent:${cleanValue}`;
        break;
      case 'infolder':
        term = `${prefix}infolder:${cleanValue}`;
        break;
      case 'nosubfolders':
        term = `${prefix}nosubfolders:${cleanValue}`;
        break;
      case 'content':
        term = `${prefix}content:${cleanValue}`;
        break;
      case 'regex':
        term = `${prefix}regex:${cleanValue}`;
        break;
      case 'tag':
        term = `${prefix}tag:${cleanValue}`;
        break;
      case 'kind':
        term = `${prefix}${cleanValue}:`;
        break;
      default:
        term = '';
    }
    return term;
  };

  const handleApply = () => {
    const query = processGroup(rootGroup);
    onApplyQuery(query);
    onClose();
  };

  const updateRoot = (newRoot: RuleGroup) => {
    setRootGroup(newRoot);
  };

  return (
    <div className="query-builder-overlay" onClick={onClose}>
      <div className="query-builder-modal" onClick={(e) => e.stopPropagation()}>
        <div className="query-builder-header">
          <span className="query-builder-title">{t('queryBuilder.title', 'Advanced Search')}</span>
          <button className="query-builder-close" onClick={onClose}>
            &times;
          </button>
        </div>

        <div className="query-builder-content">
          {parseWarning && <div className="query-builder-warning">⚠️ {parseWarning}</div>}
          <Group
            group={rootGroup}
            onChange={updateRoot}
            isRoot={true}
            onRemove={() => {}} // Root cannot be removed
          />
        </div>

        <div className="query-builder-footer">
          <div className="query-preview">
            <span className="query-preview-label">
              {t('queryBuilder.preview', 'Query Preview')}:{' '}
            </span>
            <code className="query-preview-code">{processGroup(rootGroup) || '...'}</code>
          </div>
          <button className="query-builder-apply-btn" onClick={handleApply}>
            {t('queryBuilder.search', 'Search')}
          </button>
        </div>
      </div>
    </div>
  );
}

interface GroupProps {
  group: RuleGroup;
  onChange: (g: RuleGroup) => void;
  onRemove: () => void;
  isRoot?: boolean;
}

function Group({ group, onChange, onRemove, isRoot = false }: GroupProps) {
  const { t } = useTranslation();

  const updateCombinator = (combinator: 'and' | 'or') => {
    onChange({ ...group, combinator });
  };

  const toggleExclude = () => {
    onChange({ ...group, exclude: !group.exclude });
  };

  const addItem = (type: 'rule' | 'group') => {
    const newItem: SearchItem =
      type === 'rule'
        ? {
            id: crypto.randomUUID(),
            type: 'rule',
            field: 'name',
            operator: 'contains',
            value: '',
            exclude: false,
          }
        : {
            id: crypto.randomUUID(),
            type: 'group',
            combinator: 'and',
            items: [], // Start empty as requested
            exclude: false,
          };
    onChange({ ...group, items: [...group.items, newItem] });
  };

  const updateItem = (id: string, newItem: SearchItem) => {
    onChange({
      ...group,
      items: group.items.map((i) => (i.id === id ? newItem : i)),
    });
  };

  const removeItem = (id: string) => {
    onChange({
      ...group,
      items: group.items.filter((i) => i.id !== id),
    });
  };

  return (
    <div className={`query-group ${isRoot ? 'root-group' : ''}`}>
      <div className="query-group-header">
        <div className="match-type-toggle">
          {/* Integrated NOT toggle */}
          <button
            className={`match-type-btn ${group.exclude ? 'active is-not' : ''}`}
            onClick={toggleExclude}
            title={t('queryBuilder.excludeGroup', 'Exclude group')}
            style={{ borderRight: '1px solid var(--color-border)', marginRight: 4 }}
          >
            {t('queryBuilder.not', 'NOT')}
          </button>

          <button
            className={`match-type-btn ${group.combinator === 'and' ? 'active' : ''}`}
            onClick={() => updateCombinator('and')}
          >
            {t('queryBuilder.matchAll', 'Match All')}
          </button>
          <button
            className={`match-type-btn ${group.combinator === 'or' ? 'active' : ''}`}
            onClick={() => updateCombinator('or')}
          >
            {t('queryBuilder.matchAny', 'Match Any')}
          </button>
        </div>

        {!isRoot && (
          <button
            className="query-group-remove-btn"
            onClick={onRemove}
            title={t('queryBuilder.removeGroup', 'Remove group')}
          >
            &times;
          </button>
        )}
      </div>

      <div className="query-group-items">
        {group.items.map((item) => (
          <div key={item.id} className="query-group-item-wrapper">
            {item.type === 'group' ? (
              <Group
                group={item}
                onChange={(g) => updateItem(item.id, g)}
                onRemove={() => removeItem(item.id)}
              />
            ) : (
              <Rule
                rule={item}
                onChange={(r) => updateItem(item.id, r)}
                onRemove={() => removeItem(item.id)}
                canRemove={group.items.length > 0} // Always allow remove in updated logic
              />
            )}
            {/* Connecting line or logic label could go here */}
          </div>
        ))}
      </div>

      <div className="query-group-actions">
        <button className="query-builder-add-btn" onClick={() => addItem('rule')}>
          + {t('queryBuilder.addRule', 'Rule')}
        </button>
        <button className="query-builder-add-btn" onClick={() => addItem('group')}>
          + {t('queryBuilder.addGroup', 'Group')}
        </button>
      </div>
    </div>
  );
}

interface RuleProps {
  rule: FilterRule;
  onChange: (r: FilterRule) => void;
  onRemove: () => void;
  canRemove: boolean;
}

function Rule({ rule, onChange, onRemove }: RuleProps) {
  const { t } = useTranslation();

  const updateRule = (updates: Partial<FilterRule>) => {
    onChange({ ...rule, ...updates });
  };

  return (
    <div className="query-rule-row">
      {/* Exclude Toggle */}
      <button
        className={`query-builder-exclude-btn ${rule.exclude ? 'is-active' : ''}`}
        onClick={() => updateRule({ exclude: !rule.exclude })}
        title={t('queryBuilder.exclude', 'Exclude')}
      >
        {t('queryBuilder.not', 'NOT')}
      </button>

      {/* Field Selector */}
      <select
        className="query-builder-select field-select"
        value={rule.field}
        onChange={(e) => updateRule({ field: e.target.value as SearchField, value: '' })}
      >
        <option value="name">{t('queryBuilder.field.name', 'Name')}</option>
        <option value="kind">{t('queryBuilder.field.kind', 'Kind')}</option>
        <option value="ext">{t('queryBuilder.field.ext', 'Extension')}</option>
        <option value="type">{t('queryBuilder.field.type', 'File Type')}</option>
        <option value="size">{t('queryBuilder.field.size', 'Size')}</option>
        <option value="dm">{t('queryBuilder.field.dm', 'Date Modified')}</option>
        <option value="dc">{t('queryBuilder.field.dc', 'Date Created')}</option>
        <option value="tag">{t('queryBuilder.field.tag', 'Tag')}</option>
        <option value="content">{t('queryBuilder.field.content', 'Content')}</option>
        <option value="regex">{t('queryBuilder.field.regex', 'Regex')}</option>
        <option value="parent">{t('queryBuilder.field.parent', 'Parent Folder')}</option>
        <option value="infolder">{t('queryBuilder.field.infolder', 'In Folder')}</option>
      </select>

      {/* Operator / Value Inputs based on Field */}
      <RuleValueInput rule={rule} onChange={(val) => updateRule({ value: val })} />

      {/* Remove Button */}
      <button
        className="query-builder-remove-btn"
        onClick={onRemove}
        title={t('queryBuilder.removeRule', 'Remove rule')}
      >
        &minus;
      </button>
    </div>
  );
}

function RuleValueInput({ rule, onChange }: { rule: FilterRule; onChange: (val: string) => void }) {
  const { t } = useTranslation();

  const handleSelectFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        if (path) {
          onChange(path);
        }
      }
    } catch (error) {
      console.error('Failed to open folder picker:', error);
    }
  };

  if (rule.field === 'kind') {
    return (
      <select
        className="query-builder-select value-input"
        value={rule.value}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">{t('queryBuilder.selectKind', 'Select kind...')}</option>
        <option value="file">{t('queryBuilder.kind.file', 'File')}</option>
        <option value="folder">{t('queryBuilder.kind.folder', 'Folder')}</option>
      </select>
    );
  }

  if (rule.field === 'type') {
    return (
      <select
        className="query-builder-select value-input"
        value={rule.value}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">{t('queryBuilder.selectType', 'Select type...')}</option>
        <option value="image">{t('queryBuilder.type.image', 'Image')}</option>
        <option value="video">{t('queryBuilder.type.video', 'Video')}</option>
        <option value="audio">{t('queryBuilder.type.audio', 'Audio')}</option>
        <option value="doc">{t('queryBuilder.type.doc', 'Document')}</option>
        <option value="code">{t('queryBuilder.type.code', 'Code')}</option>
        <option value="archive">{t('queryBuilder.type.archive', 'Archive')}</option>
        <option value="exe">{t('queryBuilder.type.exe', 'Executable')}</option>
      </select>
    );
  }

  if (rule.field === 'dm' || rule.field === 'dc') {
    return (
      <select
        className="query-builder-select value-input"
        value={rule.value}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">{t('queryBuilder.selectDate', 'Select date...')}</option>
        <option value="today">{t('queryBuilder.date.today', 'Today')}</option>
        <option value="yesterday">{t('queryBuilder.date.yesterday', 'Yesterday')}</option>
        <option value="thisweek">{t('queryBuilder.date.thisweek', 'This Week')}</option>
        <option value="pastweek">{t('queryBuilder.date.pastweek', 'Past Week')}</option>
        <option value="thismonth">{t('queryBuilder.date.thismonth', 'This Month')}</option>
        <option value="pastmonth">{t('queryBuilder.date.pastmonth', 'Past Month')}</option>
        <option value="thisyear">{t('queryBuilder.date.thisyear', 'This Year')}</option>
        <option value="pastyear">{t('queryBuilder.date.pastyear', 'Past Year')}</option>
      </select>
    );
  }

  if (rule.field === 'size') {
    return (
      <div className="query-builder-size-input-group">
        <select
          className="query-builder-select"
          value={rule.value}
          onChange={(e) => onChange(e.target.value)}
        >
          <option value="">{t('queryBuilder.selectSize', 'Select size...')}</option>
          <option value="empty">{t('queryBuilder.size.empty', 'Empty (0kb)')}</option>
          <option value="small">{t('queryBuilder.size.small', 'Small')}</option>
          <option value="medium">{t('queryBuilder.size.medium', 'Medium')}</option>
          <option value="large">{t('queryBuilder.size.large', 'Large')}</option>
          <option value=">1mb">{t('queryBuilder.size.gt1mb', '> 1MB')}</option>
          <option value=">1gb">{t('queryBuilder.size.gt1gb', '> 1GB')}</option>
        </select>
      </div>
    );
  }

  const placeholders: Record<string, string> = {
    name: t('queryBuilder.inputPlaceholder', 'Enter value...'),
    ext: 'jpg;png',
    parent: '/Users/demo/Documents',
    infolder: '/Users/demo/Projects',
    regex: '^Report.*2024$',
    tag: 'Important',
    content: 'TODO',
  };

  const isPathField = rule.field === 'parent' || rule.field === 'infolder';

  return (
    <div className="query-builder-input-wrapper" style={{ display: 'flex', gap: 8, width: '100%' }}>
      <input
        type="text"
        className="query-builder-input value-input"
        value={rule.value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholders[rule.field] || placeholders.name}
        style={{ flex: 1, width: 'auto' }}
      />
      {isPathField && (
        <button
          className="query-builder-folder-btn"
          onClick={handleSelectFolder}
          title={t('queryBuilder.selectFolder', 'Select Folder')}
          type="button"
          style={{ minWidth: 36 }}
        >
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="#6b7280"
            style={{ width: 18, height: 18, minWidth: 18, minHeight: 18, flexShrink: 0 }}
          >
            <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" />
          </svg>
        </button>
      )}
    </div>
  );
}
