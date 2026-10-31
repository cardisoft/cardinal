import React from 'react';
import { useTranslation } from 'react-i18next';
import { LANGUAGE_OPTIONS, normalizeLanguageTag } from '../i18n/config';
import { broadcastUiPreferencesChanged } from '../runtime/uiPreferences';

type LanguageSwitcherProps = {
  className: string;
};

const LanguageSwitcher = ({ className }: LanguageSwitcherProps): React.JSX.Element => {
  const { t, i18n } = useTranslation();

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const nextLang = event.target.value;
    // `changeLanguage` persists the choice; the other window has its own i18n instance.
    void i18n.changeLanguage(nextLang).then(() => broadcastUiPreferencesChanged());
  };

  const currentCode = normalizeLanguageTag(i18n.language);

  return (
    <div className={className}>
      <span className="sr-only">{t('language.label')}</span>
      <div className="language-switcher">
        <span className="language-switcher__text">{t('language.trigger')}</span>
        <select
          className="language-switcher__select"
          value={currentCode}
          onChange={handleChange}
          aria-label={t('language.label')}
        >
          {LANGUAGE_OPTIONS.map((option) => (
            <option key={option.code} value={option.code}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
};

export default LanguageSwitcher;
