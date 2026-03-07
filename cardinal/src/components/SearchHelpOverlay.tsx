import React, { useEffect, useMemo } from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useTranslation } from 'react-i18next';
import { getTooltipAttributes } from '../utils/tooltip';

const SEARCH_SYNTAX_DOCS_URL =
  'https://github.com/cardisoft/cardinal/blob/master/doc/pub/search-syntax.md';

type SearchExample = {
  query: string;
  description: string;
};

export type SearchHelpOverlayProps = {
  open: boolean;
  onClose: () => void;
  onApplyExample: (example: string) => void;
};

export function SearchHelpOverlay({
  open,
  onClose,
  onApplyExample,
}: SearchHelpOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();

  useEffect(() => {
    if (!open) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  const fileExamples = useMemo<SearchExample[]>(
    () => [
      {
        query: 'test report',
        description: t('search.help.exampleDescriptions.plainWord'),
      },
      {
        query: '*.sqlite',
        description: t('search.help.exampleDescriptions.extensionWildcard'),
      },
      {
        query: 'content:"TODO"',
        description: t('search.help.exampleDescriptions.content'),
      },
      {
        query: 'audio:',
        description: t('search.help.exampleDescriptions.typeMacro'),
      },
      {
        query: 'dm:today',
        description: t('search.help.exampleDescriptions.todayModified'),
      },
      {
        query: 'ext:pdf',
        description: t('search.help.exampleDescriptions.extensionFilter'),
      },
      {
        query: 'size:1mb..10mb',
        description: t('search.help.exampleDescriptions.sizeRange'),
      },
      {
        query: 'src/**/Cargo.toml',
        description: t('search.help.exampleDescriptions.globstarPath'),
      },
      {
        query: 'regex:^README\\.md$',
        description: t('search.help.exampleDescriptions.regex'),
      },
      {
        query: '"Application Support"',
        description: t('search.help.exampleDescriptions.quotedPhrase'),
      },
      {
        query: 'type:picture',
        description: t('search.help.exampleDescriptions.typeFilter'),
      },
      {
        query: 'in:/Users/demo/Projects',
        description: t('search.help.exampleDescriptions.inScope'),
      },
      {
        query: 'parent:/Users/demo/Documents',
        description: t('search.help.exampleDescriptions.parentScope'),
      },
      {
        query: 'nosubfolders:/Users/demo/Projects',
        description: t('search.help.exampleDescriptions.noSubfoldersScope'),
      },
    ],
    [t],
  );
  const guideGroups = useMemo(
    () => [
      {
        title: t('search.help.groups.matching.title'),
        items: [
          t('search.help.files.fullPath'),
          t('search.help.files.words'),
          t('search.help.files.phrases'),
          t('search.help.files.pathSegments'),
        ],
      },
      {
        title: t('search.help.groups.logic.title'),
        items: [
          t('search.help.files.implicitAnd'),
          t('search.help.files.orOperator'),
          t('search.help.files.notOperator'),
          t('search.help.files.grouping'),
          t('search.help.files.precedence'),
        ],
      },
      {
        title: t('search.help.groups.filters.title'),
        items: [
          t('search.help.filters.fileFolder'),
          t('search.help.filters.extensions'),
          t('search.help.filters.types'),
          t('search.help.filters.typeMacros'),
          t('search.help.filters.scope'),
          t('search.help.filters.dates'),
          t('search.help.filters.size'),
          t('search.help.filters.regexContent'),
        ],
      },
      {
        title: t('search.help.groups.patterns.title'),
        items: [
          t('search.help.files.expandOnTilde'),
          t('search.help.files.wildcards'),
          t('search.help.files.globstar'),
          t('search.help.files.literalWildcards'),
          t('search.help.files.segmentAnchors'),
          t('search.help.files.caseSensitivity'),
        ],
      },
    ],
    [t],
  );

  if (!open) {
    return null;
  }

  const handleOverlayClick = (event: React.MouseEvent<HTMLDivElement>): void => {
    if (event.target === event.currentTarget) {
      onClose();
    }
  };

  const handleOpenDocs = async (): Promise<void> => {
    try {
      await openUrl(SEARCH_SYNTAX_DOCS_URL);
    } catch (error) {
      console.error('Failed to open search syntax docs', error);
    }
  };

  const handleApplyExample = (example: string): void => {
    onApplyExample(example);
  };

  const handleDocsClick = (event: React.MouseEvent<HTMLAnchorElement>): void => {
    event.preventDefault();
    void handleOpenDocs();
  };

  return (
    <div
      className="search-help-overlay"
      role="dialog"
      aria-modal="true"
      onClick={handleOverlayClick}
    >
      <div className="search-help-card">
        <header className="search-help-card__header">
          <div className="search-help-card__header-main">
            <div>
              <h1 className="search-help-card__title">{t('search.help.title')}</h1>
            </div>
            <a className="search-help-docs" href={SEARCH_SYNTAX_DOCS_URL} onClick={handleDocsClick}>
              {t('search.help.openDocs')}
              <span aria-hidden="true" className="search-help-docs__icon">
                ↗
              </span>
            </a>
          </div>
        </header>

        <div className="search-help-content">
          <section className="search-help-examples-panel">
            <div className="search-help-section-copy">
              <h2>{t('search.help.examplesTitle')}</h2>
              <p className="search-help-card__subtitle">{t('search.help.subtitle')}</p>
              <p>{t('search.help.examplesDescription')}</p>
            </div>
            <div className="search-help-examples">
              {fileExamples.map((example) => (
                <button
                  key={example.query}
                  className="search-help-example"
                  type="button"
                  onClick={() => handleApplyExample(example.query)}
                  data-tooltip-class-name="search-help-tooltip"
                  {...getTooltipAttributes(example.description)}
                >
                  <code>{example.query}</code>
                </button>
              ))}
            </div>
          </section>

          <section className="search-help-section">
            <div className="search-help-section-copy">
              <h2>{t('search.help.guideTitle')}</h2>
              <p>{t('search.help.files.description')}</p>
            </div>
            <div className="search-help-guide-grid">
              {guideGroups.map((group) => (
                <article key={group.title} className="search-help-guide-card">
                  <h3>{group.title}</h3>
                  <ul className="search-help-list">
                    {group.items.map((item) => (
                      <li key={item}>{item}</li>
                    ))}
                  </ul>
                </article>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
