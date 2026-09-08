import { describe, expect, it } from 'vitest';
import arSA from '../resources/ar-SA.json';
import deDE from '../resources/de-DE.json';
import enUS from '../resources/en-US.json';
import esES from '../resources/es-ES.json';
import frFR from '../resources/fr-FR.json';
import hiIN from '../resources/hi-IN.json';
import itIT from '../resources/it-IT.json';
import jaJP from '../resources/ja-JP.json';
import koKR from '../resources/ko-KR.json';
import ptBR from '../resources/pt-BR.json';
import ruRU from '../resources/ru-RU.json';
import trTR from '../resources/tr-TR.json';
import ukUA from '../resources/uk-UA.json';
import zhCN from '../resources/zh-CN.json';
import zhTW from '../resources/zh-TW.json';

const flattenKeys = (value: unknown, prefix = ''): string[] => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    flattenKeys(child, prefix ? `${prefix}.${key}` : key),
  );
};

const resources = {
  arSA,
  deDE,
  esES,
  frFR,
  hiIN,
  itIT,
  jaJP,
  koKR,
  ptBR,
  ruRU,
  trTR,
  ukUA,
  zhCN,
  zhTW,
};

const featurePrefixes = [
  'search.options.newest',
  'search.filters.',
  'contextMenu.moveToTrash_',
  'contextMenu.confirmTrash_',
  'contextMenu.trashFailed',
  'ignorePaths.',
  'workspace.',
  'menu.search',
  'menu.focusSearch',
  'menu.showNewest',
  'menu.savedSearches',
  'menu.favorites',
  'menu.coverage',
  'menu.operations',
  'menu.duplicates',
  'menu.showFiles',
  'menu.showEvents',
  'menu.showWorkspaceToolbar',
  'menu.indexing',
  'menu.rescan',
  'menu.searchSettings',
];

const featureKeys = flattenKeys(enUS).filter((key) =>
  featurePrefixes.some((prefix) => key === prefix || key.startsWith(prefix)),
);

const valueAtPath = (resource: unknown, path: string): unknown =>
  path
    .split('.')
    .reduce<unknown>(
      (value, key) =>
        value && typeof value === 'object' ? (value as Record<string, unknown>)[key] : undefined,
      resource,
    );

const interpolationVariables = (value: unknown): string[] =>
  typeof value === 'string' ? (value.match(/{{\w+}}/g) ?? []).sort() : [];

describe('translation resources', () => {
  it.each(Object.entries(resources))(
    '%s has every added feature translation key',
    (_locale, resource) => {
      const resourceKeys = new Set(flattenKeys(resource));
      expect(featureKeys.filter((key) => !resourceKeys.has(key))).toEqual([]);
    },
  );

  it.each(Object.entries(resources))(
    '%s preserves added feature variables',
    (_locale, resource) => {
      for (const key of featureKeys) {
        expect(interpolationVariables(valueAtPath(resource, key))).toEqual(
          interpolationVariables(valueAtPath(enUS, key)),
        );
      }
    },
  );
});
