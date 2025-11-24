import { startDrag as startNativeDrag } from '@crabnebula/tauri-plugin-drag';

const DATA_URL_PREFIX = 'data:image/png;base64,';
const TRANSPARENT_PIXEL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/wIAAgEBAPzW4ioAAAAASUVORK5CYII=';

const toDragIcon = (icon?: string): string =>
  icon && icon.startsWith(DATA_URL_PREFIX) ? icon : TRANSPARENT_PIXEL;

export type NativeFileDragOptions = Readonly<{
  paths: readonly string[];
  icon?: string;
  mode?: 'copy' | 'move';
}>;

export const startNativeFileDrag = async ({
  paths,
  icon,
  mode = 'copy',
}: NativeFileDragOptions): Promise<void> => {
  const normalizedPaths = paths.filter(
    (filePath): filePath is string => typeof filePath === 'string' && filePath.length > 0,
  );
  if (normalizedPaths.length === 0) {
    return;
  }

  try {
    await startNativeDrag({
      item: normalizedPaths,
      icon: toDragIcon(icon),
      mode,
    });
  } catch (error) {
    console.warn('startNativeFileDrag failed; native drag unavailable', error);
  }
};
