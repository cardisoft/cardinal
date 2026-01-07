import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  applyThemePreference,
  getStoredThemePreference,
  persistThemePreference,
  type ThemePreference,
} from '../theme';

type ThemeSwitcherProps = {
  className?: string;
  resetToken?: number;
};

type ThemeOption = {
  value: ThemePreference;
  icon: string;
  labelKey: string;
};

const THEME_OPTIONS: ThemeOption[] = [
  { value: 'system', icon: '🌓', labelKey: 'theme.options.system' },
  { value: 'light', icon: '🌕', labelKey: 'theme.options.light' },
  { value: 'dark', icon: '🌙', labelKey: 'theme.options.dark' },
];

const ThemeSwitcher = ({ className, resetToken }: ThemeSwitcherProps): React.JSX.Element => {
  const { t } = useTranslation();
  const [preference, setPreference] = useState<ThemePreference>(() => getStoredThemePreference());

  useEffect(() => {
    persistThemePreference(preference);
    applyThemePreference(preference);
  }, [preference]);

  useEffect(() => {
    if (resetToken === undefined) return;
    setPreference(getStoredThemePreference());
  }, [resetToken]);

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const nextPreference = event.target.value as ThemePreference;
    setPreference(nextPreference);
  };

  const activeOption =
    THEME_OPTIONS.find((option) => option.value === preference) ?? THEME_OPTIONS[0];

  return (
    <div className={className}>
      <span className="sr-only">{t('theme.label')}</span>
      <div className="theme-switcher">
        <span className="theme-switcher__icon">{activeOption.icon}</span>
        <select
          className="theme-switcher__select"
          value={preference}
          onChange={handleChange}
          aria-label={t('theme.label')}
        >
          {THEME_OPTIONS.map((option) => (
            <option key={option.value} value={option.value}>
              {t(option.labelKey)}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
};

export default ThemeSwitcher;
