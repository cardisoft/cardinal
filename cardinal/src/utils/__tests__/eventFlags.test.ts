import { describe, expect, it } from 'vitest';
import { describeEventFlags, FLAG_BITS } from '../eventFlags';

describe('describeEventFlags', () => {
  describe('invalid inputs', () => {
    it('returns a sentinel payload for undefined', () => {
      expect(describeEventFlags(undefined)).toEqual({ labels: ['—'], tooltip: 'No event flags' });
    });

    it('returns a sentinel payload for null', () => {
      expect(describeEventFlags(null)).toEqual({ labels: ['—'], tooltip: 'No event flags' });
    });

    it('returns a sentinel payload for positive infinity', () => {
      expect(describeEventFlags(Number.POSITIVE_INFINITY)).toEqual({
        labels: ['—'],
        tooltip: 'No event flags',
      });
    });

    it('returns a sentinel payload for negative infinity', () => {
      expect(describeEventFlags(Number.NEGATIVE_INFINITY)).toEqual({
        labels: ['—'],
        tooltip: 'No event flags',
      });
    });

    it('returns a sentinel payload for NaN', () => {
      expect(describeEventFlags(Number.NaN)).toEqual({
        labels: ['—'],
        tooltip: 'No event flags',
      });
    });
  });

  describe('zero flag', () => {
    it('returns "Other" when flagBits is 0', () => {
      const result = describeEventFlags(0);
      expect(result.labels).toEqual(['Other']);
      expect(result.tooltip).toBe('0x0');
    });
  });

  describe('single action flags', () => {
    it('identifies ITEM_CREATED', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_CREATED);
      expect(result.labels).toEqual(['Created']);
      expect(result.tooltip).toContain('ItemCreated (0x100): File or directory was created');
      expect(result.tooltip.trim().endsWith('flags: 0x100)')).toBe(true);
    });

    it('identifies ITEM_REMOVED', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_REMOVED);
      expect(result.labels).toEqual(['Removed']);
      expect(result.tooltip).toContain('ItemRemoved (0x200): File or directory was removed');
      expect(result.tooltip.trim().endsWith('flags: 0x200)')).toBe(true);
    });

    it('identifies ITEM_RENAMED', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_RENAMED);
      expect(result.labels).toEqual(['Renamed']);
      expect(result.tooltip).toContain('ItemRenamed (0x800): Path was renamed or moved');
      expect(result.tooltip.trim().endsWith('flags: 0x800)')).toBe(true);
    });

    it('identifies ITEM_MODIFIED', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_MODIFIED);
      expect(result.labels).toEqual(['Modified']);
      expect(result.tooltip).toContain('ItemModified (0x1000): File contents changed');
      expect(result.tooltip.trim().endsWith('flags: 0x1000)')).toBe(true);
    });

    it('identifies ITEM_CLONED', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_CLONED);
      expect(result.labels).toEqual(['Cloned']);
      expect(result.tooltip).toContain('ItemCloned (0x400000): Cloned from another file');
      expect(result.tooltip.trim().endsWith('flags: 0x400000)')).toBe(true);
    });

    it('identifies ITEM_INODE_META_MOD', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_INODE_META_MOD);
      expect(result.labels).toEqual(['InodeMetaMod']);
      expect(result.tooltip).toContain('ItemInodeMetaMod (0x400): Inode metadata changed');
      expect(result.tooltip.trim().endsWith('flags: 0x400)')).toBe(true);
    });

    it('identifies ITEM_FINDER_INFO_MOD', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_FINDER_INFO_MOD);
      expect(result.labels).toEqual(['FinderInfoMod']);
      expect(result.tooltip).toContain('ItemFinderInfoMod (0x2000): Finder info changed');
      expect(result.tooltip.trim().endsWith('flags: 0x2000)')).toBe(true);
    });

    it('identifies ITEM_CHANGE_OWNER', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_CHANGE_OWNER);
      expect(result.labels).toEqual(['ChangeOwner']);
      expect(result.tooltip).toContain('ItemChangeOwner (0x4000): Owner changed');
      expect(result.tooltip.trim().endsWith('flags: 0x4000)')).toBe(true);
    });

    it('identifies ITEM_XATTR_MOD', () => {
      const result = describeEventFlags(FLAG_BITS.ITEM_XATTR_MOD);
      expect(result.labels).toEqual(['XattrMod']);
      expect(result.tooltip).toContain('ItemXattrMod (0x8000): Extended attributes changed');
      expect(result.tooltip.trim().endsWith('flags: 0x8000)')).toBe(true);
    });
  });

  describe('single system flags', () => {
    it('identifies ROOT_CHANGED', () => {
      const result = describeEventFlags(FLAG_BITS.ROOT_CHANGED);
      expect(result.labels).toEqual(['RootChanged']);
      expect(result.tooltip).toContain('RootChanged (0x20): Watched root moved or removed');
      expect(result.tooltip.trim().endsWith('flags: 0x20)')).toBe(true);
    });

    it('identifies MOUNT', () => {
      const result = describeEventFlags(FLAG_BITS.MOUNT);
      expect(result.labels).toEqual(['Mount']);
      expect(result.tooltip).toContain('Mount (0x40): Volume mounted');
      expect(result.tooltip.trim().endsWith('flags: 0x40)')).toBe(true);
    });

    it('identifies UNMOUNT', () => {
      const result = describeEventFlags(FLAG_BITS.UNMOUNT);
      expect(result.labels).toEqual(['Unmount']);
      expect(result.tooltip).toContain('Unmount (0x80): Volume unmounted');
      expect(result.tooltip.trim().endsWith('flags: 0x80)')).toBe(true);
    });

    it('identifies MUST_SCAN_SUBDIRS', () => {
      const result = describeEventFlags(FLAG_BITS.MUST_SCAN_SUBDIRS);
      expect(result.labels).toEqual(['MustScanSubDirs']);
      expect(result.tooltip).toContain('MustScanSubDirs (0x1): Client must rescan subtree');
      expect(result.tooltip.trim().endsWith('flags: 0x1)')).toBe(true);
    });

    it('identifies USER_DROPPED', () => {
      const result = describeEventFlags(FLAG_BITS.USER_DROPPED);
      expect(result.labels).toEqual(['UserDropped']);
      expect(result.tooltip).toContain('UserDropped (0x2): User-space dropped events');
      expect(result.tooltip.trim().endsWith('flags: 0x2)')).toBe(true);
    });

    it('identifies KERNEL_DROPPED', () => {
      const result = describeEventFlags(FLAG_BITS.KERNEL_DROPPED);
      expect(result.labels).toEqual(['KernelDropped']);
      expect(result.tooltip).toContain('KernelDropped (0x4): Kernel dropped events');
      expect(result.tooltip.trim().endsWith('flags: 0x4)')).toBe(true);
    });

    it('identifies EVENT_IDS_WRAPPED', () => {
      const result = describeEventFlags(FLAG_BITS.EVENT_IDS_WRAPPED);
      expect(result.labels).toEqual(['EventIdsWrapped']);
      expect(result.tooltip).toContain('EventIdsWrapped (0x8): FSEvent IDs wrapped');
      expect(result.tooltip.trim().endsWith('flags: 0x8)')).toBe(true);
    });

    it('identifies HISTORY_DONE', () => {
      const result = describeEventFlags(FLAG_BITS.HISTORY_DONE);
      expect(result.labels).toEqual(['HistoryDone']);
      expect(result.tooltip).toContain('HistoryDone (0x10): Historical replay finished');
      expect(result.tooltip.trim().endsWith('flags: 0x10)')).toBe(true);
    });
  });

  describe('OWN_EVENT flag', () => {
    it('identifies OWN_EVENT flag alone', () => {
      const result = describeEventFlags(FLAG_BITS.OWN_EVENT);
      expect(result.labels).toEqual(['OwnEvent']);
      expect(result.tooltip).toContain('OwnEvent (0x80000): Generated by this process');
      expect(result.tooltip.trim().endsWith('flags: 0x80000)')).toBe(true);
    });

    it('includes OWN_EVENT in combination with action flags', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.OWN_EVENT;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('Created');
      expect(result.labels).toContain('OwnEvent');
      expect(result.tooltip).toContain('ItemCreated (0x100): File or directory was created');
      expect(result.tooltip).toContain('OwnEvent (0x80000): Generated by this process');
    });
  });

  describe('type flags (should be filtered out of labels)', () => {
    it('does not show ITEM_IS_FILE in labels', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_IS_FILE;
      const result = describeEventFlags(bits);
      expect(result.labels).not.toContain('IsFile');
      expect(result.labels).toContain('Created');
    });

    it('does not show ITEM_IS_DIR in labels', () => {
      const bits = FLAG_BITS.ITEM_REMOVED | FLAG_BITS.ITEM_IS_DIR;
      const result = describeEventFlags(bits);
      expect(result.labels).not.toContain('IsDir');
      expect(result.labels).toContain('Removed');
    });

    it('does not show ITEM_IS_SYMLINK in labels', () => {
      const bits = FLAG_BITS.ITEM_MODIFIED | FLAG_BITS.ITEM_IS_SYMLINK;
      const result = describeEventFlags(bits);
      expect(result.labels).not.toContain('IsSymlink');
      expect(result.labels).toContain('Modified');
    });

    it('does not show ITEM_IS_HARDLINK in labels', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_IS_HARDLINK;
      const result = describeEventFlags(bits);
      expect(result.labels).not.toContain('IsHardlink');
      expect(result.labels).toContain('Created');
    });

    it('does not show ITEM_IS_LAST_HARDLINK in labels', () => {
      const bits = FLAG_BITS.ITEM_REMOVED | FLAG_BITS.ITEM_IS_LAST_HARDLINK;
      const result = describeEventFlags(bits);
      expect(result.labels).not.toContain('IsLastHardlink');
      expect(result.labels).toContain('Removed');
    });
  });

  describe('action flag priority', () => {
    it('prioritizes action flags ahead of system flags in the labels list', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ROOT_CHANGED;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Created', 'RootChanged']);
      expect(result.tooltip).toContain('ItemCreated (0x100): File or directory was created');
      expect(result.tooltip).toContain('RootChanged (0x20): Watched root moved or removed');
      expect(result.tooltip.trim().endsWith('flags: 0x120)')).toBe(true);
    });

    it('prioritizes first matching action flag when multiple action flags are present', () => {
      const bits = FLAG_BITS.ITEM_MODIFIED | FLAG_BITS.ITEM_CREATED;
      const result = describeEventFlags(bits);
      expect(result.labels[0]).toBe('Created');
      expect(result.labels).toContain('Modified');
    });

    it('falls back to system flags when no action flags are present', () => {
      const result = describeEventFlags(FLAG_BITS.ROOT_CHANGED);
      expect(result.labels).toEqual(['RootChanged']);
      expect(result.tooltip).toContain('RootChanged (0x20): Watched root moved or removed');
    });
  });

  describe('multiple flag combinations', () => {
    it('preserves action flag declaration order across multiple flags', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_MODIFIED;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Created', 'Modified']);
      expect(result.tooltip).toContain('ItemCreated (0x100): File or directory was created');
      expect(result.tooltip).toContain('ItemModified (0x1000): File contents changed');
    });

    it('handles complex multi-flag combinations', () => {
      const bits =
        FLAG_BITS.ITEM_RENAMED |
        FLAG_BITS.ITEM_INODE_META_MOD |
        FLAG_BITS.ITEM_XATTR_MOD |
        FLAG_BITS.OWN_EVENT;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('Renamed');
      expect(result.labels).toContain('InodeMetaMod');
      expect(result.labels).toContain('XattrMod');
      expect(result.labels).toContain('OwnEvent');
      expect(result.tooltip).toContain('ItemRenamed (0x800)');
      expect(result.tooltip).toContain('ItemInodeMetaMod (0x400)');
      expect(result.tooltip).toContain('ItemXattrMod (0x8000)');
      expect(result.tooltip).toContain('OwnEvent (0x80000)');
    });

    it('handles all action flags combined', () => {
      const bits =
        FLAG_BITS.ITEM_CREATED |
        FLAG_BITS.ITEM_REMOVED |
        FLAG_BITS.ITEM_RENAMED |
        FLAG_BITS.ITEM_MODIFIED |
        FLAG_BITS.ITEM_CLONED |
        FLAG_BITS.ITEM_INODE_META_MOD |
        FLAG_BITS.ITEM_FINDER_INFO_MOD |
        FLAG_BITS.ITEM_CHANGE_OWNER |
        FLAG_BITS.ITEM_XATTR_MOD;
      const result = describeEventFlags(bits);
      expect(result.labels.length).toBe(9);
      expect(result.labels).toContain('Created');
      expect(result.labels).toContain('Removed');
      expect(result.labels).toContain('Renamed');
      expect(result.labels).toContain('Modified');
      expect(result.labels).toContain('Cloned');
    });

    it('handles all system flags combined', () => {
      const bits =
        FLAG_BITS.MUST_SCAN_SUBDIRS |
        FLAG_BITS.USER_DROPPED |
        FLAG_BITS.KERNEL_DROPPED |
        FLAG_BITS.EVENT_IDS_WRAPPED |
        FLAG_BITS.HISTORY_DONE |
        FLAG_BITS.ROOT_CHANGED |
        FLAG_BITS.MOUNT |
        FLAG_BITS.UNMOUNT;
      const result = describeEventFlags(bits);
      expect(result.labels.length).toBe(8);
      expect(result.labels).toContain('MustScanSubDirs');
      expect(result.labels).toContain('RootChanged');
      expect(result.labels).toContain('Mount');
      expect(result.labels).toContain('Unmount');
    });

    it('handles system flags combined with type flags', () => {
      const bits = FLAG_BITS.MOUNT | FLAG_BITS.ITEM_IS_DIR;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Mount']);
      expect(result.tooltip).toContain('Mount (0x40)');
    });
  });

  describe('unknown flags', () => {
    it('adds an unknown entry for bits outside the known mask', () => {
      const UNKNOWN = 0x80000000;
      const bits = FLAG_BITS.ITEM_CREATED | UNKNOWN;
      const result = describeEventFlags(bits);
      expect(result.labels[0]).toBe('Created');
      expect(result.tooltip).toContain('ItemCreated (0x100): File or directory was created');
      expect(result.tooltip).toContain('Unknown: 0x-80000000');
      expect(result.tooltip.trim().endsWith('flags: 0x-7fffff00)')).toBe(true);
    });

    it('handles only unknown flags', () => {
      const UNKNOWN = 0x10000000;
      const result = describeEventFlags(UNKNOWN);
      expect(result.labels).toEqual(['Other']);
      expect(result.tooltip).toContain('Unknown: 0x10000000');
      expect(result.tooltip.trim().endsWith('flags: 0x10000000)')).toBe(true);
    });

    it('handles multiple unknown bits', () => {
      const UNKNOWN1 = 0x10000000;
      const UNKNOWN2 = 0x20000000;
      const bits = FLAG_BITS.ITEM_MODIFIED | UNKNOWN1 | UNKNOWN2;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('Modified');
      expect(result.tooltip).toContain('ItemModified (0x1000)');
      expect(result.tooltip).toContain('Unknown: 0x30000000');
    });
  });

  describe('tooltip formatting', () => {
    it('formats tooltip with newline-separated entries', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_MODIFIED;
      const result = describeEventFlags(bits);
      const lines = result.tooltip.split('\n');
      expect(lines.length).toBeGreaterThan(2);
      expect(lines[0]).toContain('ItemCreated (0x100)');
      expect(lines[1]).toContain('ItemModified (0x1000)');
      expect(lines[lines.length - 1]).toContain('flags: 0x1100');
    });

    it('includes hex representation of total flags in tooltip', () => {
      const bits = FLAG_BITS.ITEM_RENAMED | FLAG_BITS.OWN_EVENT;
      const result = describeEventFlags(bits);
      expect(result.tooltip.trim().endsWith('flags: 0x80800)')).toBe(true);
    });

    it('formats simple tooltip for zero flags', () => {
      const result = describeEventFlags(0);
      expect(result.tooltip).toBe('0x0');
    });
  });

  describe('realistic file operation scenarios', () => {
    it('handles typical file creation event', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_IS_FILE;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Created']);
      expect(result.tooltip).toContain('ItemCreated (0x100)');
    });

    it('handles typical file modification event', () => {
      const bits = FLAG_BITS.ITEM_MODIFIED | FLAG_BITS.ITEM_IS_FILE | FLAG_BITS.OWN_EVENT;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('Modified');
      expect(result.labels).toContain('OwnEvent');
    });

    it('handles directory creation event', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_IS_DIR;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Created']);
    });

    it('handles file rename/move event', () => {
      const bits = FLAG_BITS.ITEM_RENAMED | FLAG_BITS.ITEM_INODE_META_MOD | FLAG_BITS.ITEM_IS_FILE;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('Renamed');
      expect(result.labels).toContain('InodeMetaMod');
    });

    it('handles symlink creation', () => {
      const bits = FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_IS_SYMLINK;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Created']);
    });

    it('handles volume mount event', () => {
      const bits = FLAG_BITS.MOUNT;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['Mount']);
      expect(result.tooltip).toContain('Volume mounted');
    });

    it('handles permission change event', () => {
      const bits = FLAG_BITS.ITEM_CHANGE_OWNER | FLAG_BITS.ITEM_INODE_META_MOD;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('ChangeOwner');
      expect(result.labels).toContain('InodeMetaMod');
    });

    it('handles extended attribute modification', () => {
      const bits = FLAG_BITS.ITEM_XATTR_MOD | FLAG_BITS.ITEM_IS_FILE;
      const result = describeEventFlags(bits);
      expect(result.labels).toEqual(['XattrMod']);
    });

    it('handles APFS cloning event', () => {
      const bits = FLAG_BITS.ITEM_CLONED | FLAG_BITS.ITEM_CREATED | FLAG_BITS.ITEM_IS_FILE;
      const result = describeEventFlags(bits);
      expect(result.labels).toContain('Created');
      expect(result.labels).toContain('Cloned');
    });
  });

  describe('edge cases', () => {
    it('handles maximum safe integer', () => {
      const result = describeEventFlags(Number.MAX_SAFE_INTEGER);
      expect(result.labels).toBeDefined();
      expect(result.tooltip).toBeDefined();
      expect(result.tooltip).toContain('Unknown');
    });

    it('handles negative numbers', () => {
      const result = describeEventFlags(-1);
      expect(result.labels).toBeDefined();
      expect(result.tooltip).toBeDefined();
    });

    it('handles very small positive numbers', () => {
      const result = describeEventFlags(1);
      expect(result.labels).toEqual(['MustScanSubDirs']);
      expect(result.tooltip).toContain('MustScanSubDirs (0x1)');
    });

    it('handles power-of-two boundary', () => {
      const result = describeEventFlags(0x00800000);
      expect(result.labels).toEqual(['Other']);
      expect(result.tooltip).toContain('Unknown: 0x800000');
    });
  });
});
