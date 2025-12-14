import React from 'react';
import { useTranslation } from 'react-i18next';
import { LANGUAGE_OPTIONS } from '../i18n/config';

type LanguageSwitcherProps = {
  className?: string;
};

const LanguageSwitcher = ({ className }: LanguageSwitcherProps): React.JSX.Element => {
  const { t, i18n } = useTranslation();

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const nextLang = event.target.value;
    void i18n.changeLanguage(nextLang);
  };

  // Match exact code first, then fall back to the longest prefix so values like "en-US" map to "en".
  const currentCode =
    LANGUAGE_OPTIONS.find((option) => option.code === i18n.language)?.code ??
    [...LANGUAGE_OPTIONS]
      .sort((a, b) => b.code.length - a.code.length)
      .find((option) => i18n.language.startsWith(option.code))?.code ??
    LANGUAGE_OPTIONS[0].code;

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
