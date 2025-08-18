import { useEffect, useCallback, useRef } from 'react';

export interface KeyboardShortcut {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  action: () => void;
  description?: string;
  preventDefault?: boolean;
  stopPropagation?: boolean;
}

export interface UseKeyboardShortcutsOptions {
  enabled?: boolean;
  target?: HTMLElement | Document | null;
  preventDefault?: boolean;
  stopPropagation?: boolean;
}

export const useKeyboardShortcuts = (
  shortcuts: KeyboardShortcut[],
  options: UseKeyboardShortcutsOptions = {}
) => {
  const {
    enabled = true,
    target,
    preventDefault = true,
    stopPropagation = false,
  } = options;

  const shortcutsRef = useRef(shortcuts);
  const optionsRef = useRef(options);

  // Update refs when props change
  useEffect(() => {
    shortcutsRef.current = shortcuts;
    optionsRef.current = options;
  }, [shortcuts, options]);

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!enabled) return;

    const { key, ctrlKey, metaKey, shiftKey, altKey } = event;

    // Find matching shortcut
    const matchingShortcut = shortcutsRef.current.find(shortcut => {
      const keyMatch = shortcut.key.toLowerCase() === key.toLowerCase();
      const ctrlMatch = (shortcut.ctrlKey ?? false) === ctrlKey;
      const metaMatch = (shortcut.metaKey ?? false) === metaKey;
      const shiftMatch = (shortcut.shiftKey ?? false) === shiftKey;
      const altMatch = (shortcut.altKey ?? false) === altKey;

      return keyMatch && ctrlMatch && metaMatch && shiftMatch && altMatch;
    });

    if (matchingShortcut) {
      // Use shortcut-specific settings, fallback to global options
      const shouldPreventDefault = matchingShortcut.preventDefault ?? preventDefault;
      const shouldStopPropagation = matchingShortcut.stopPropagation ?? stopPropagation;

      if (shouldPreventDefault) {
        event.preventDefault();
      }
      if (shouldStopPropagation) {
        event.stopPropagation();
      }

      try {
        matchingShortcut.action();
      } catch (error) {
        console.error('Error executing keyboard shortcut:', error);
      }
    }
  }, [enabled, preventDefault, stopPropagation]);

  useEffect(() => {
    if (!enabled) return;

    const eventTarget = target || document;
    eventTarget.addEventListener('keydown', handleKeyDown);

    return () => {
      eventTarget.removeEventListener('keydown', handleKeyDown);
    };
  }, [enabled, target, handleKeyDown]);
};

/**
 * Hook for search-specific keyboard shortcuts
 */
export const useSearchKeyboardShortcuts = (actions: {
  focusSearch?: () => void;
  clearSearch?: () => void;
  navigateHistory?: (direction: 'up' | 'down') => void;
  submitSearch?: () => void;
  toggleFilters?: () => void;
  exportResults?: () => void;
}) => {
  const {
    focusSearch,
    clearSearch,
    navigateHistory,
    submitSearch,
    toggleFilters,
    exportResults,
  } = actions;

  const shortcuts: KeyboardShortcut[] = [
    // Focus search (Cmd/Ctrl + K)
    ...(focusSearch ? [{
      key: 'k',
      ctrlKey: true,
      metaKey: false,
      action: focusSearch,
      description: 'Focus search input',
    }, {
      key: 'k',
      ctrlKey: false,
      metaKey: true,
      action: focusSearch,
      description: 'Focus search input (Mac)',
    }] : []),

    // Clear search (Escape)
    ...(clearSearch ? [{
      key: 'Escape',
      action: clearSearch,
      description: 'Clear search',
    }] : []),

    // Navigate history (Arrow Up/Down)
    ...(navigateHistory ? [{
      key: 'ArrowUp',
      action: () => navigateHistory('up'),
      description: 'Previous search',
      preventDefault: false, // Let components handle this
    }, {
      key: 'ArrowDown',
      action: () => navigateHistory('down'),
      description: 'Next search',
      preventDefault: false, // Let components handle this
    }] : []),

    // Submit search (Enter)
    ...(submitSearch ? [{
      key: 'Enter',
      action: submitSearch,
      description: 'Submit search',
      preventDefault: false, // Let forms handle this
    }] : []),

    // Toggle filters (Cmd/Ctrl + F)
    ...(toggleFilters ? [{
      key: 'f',
      ctrlKey: true,
      action: toggleFilters,
      description: 'Toggle filters',
    }, {
      key: 'f',
      metaKey: true,
      action: toggleFilters,
      description: 'Toggle filters (Mac)',
    }] : []),

    // Export results (Cmd/Ctrl + E)
    ...(exportResults ? [{
      key: 'e',
      ctrlKey: true,
      action: exportResults,
      description: 'Export results',
    }, {
      key: 'e',
      metaKey: true,
      action: exportResults,
      description: 'Export results (Mac)',
    }] : []),
  ];

  useKeyboardShortcuts(shortcuts, {
    enabled: true,
    preventDefault: true,
  });

  return shortcuts;
};

/**
 * Utility function to format keyboard shortcut for display
 */
export const formatShortcut = (shortcut: KeyboardShortcut): string => {
  const parts: string[] = [];

  const isMac = typeof navigator !== 'undefined' && navigator.platform.includes('Mac');

  if (shortcut.ctrlKey && !isMac) parts.push('Ctrl');
  if (shortcut.metaKey || (shortcut.ctrlKey && isMac)) parts.push('⌘');
  if (shortcut.altKey) parts.push(isMac ? '⌥' : 'Alt');
  if (shortcut.shiftKey) parts.push(isMac ? '⇧' : 'Shift');

  // Format key name
  let keyName = shortcut.key;
  const keyMap: Record<string, string> = {
    'ArrowUp': '↑',
    'ArrowDown': '↓',
    'ArrowLeft': '←',
    'ArrowRight': '→',
    'Escape': 'Esc',
    'Enter': '↵',
    ' ': 'Space',
  };

  if (keyMap[keyName]) {
    keyName = keyMap[keyName];
  } else if (keyName.length === 1) {
    keyName = keyName.toUpperCase();
  }

  parts.push(keyName);

  return parts.join(isMac ? '' : '+');
};

/**
 * Hook to display keyboard shortcuts help
 */
export const useShortcutHelp = (shortcuts: KeyboardShortcut[]) => {
  const helpText = shortcuts
    .filter(s => s.description)
    .map(s => `${formatShortcut(s)}: ${s.description}`)
    .join('\n');

  return {
    helpText,
    shortcuts: shortcuts.filter(s => s.description),
  };
};