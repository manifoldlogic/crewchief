import React, { useState, useRef, useCallback, useEffect } from 'react';

interface SplitPaneProps {
  children: [React.ReactNode, React.ReactNode];
  direction?: 'horizontal' | 'vertical';
  initialSplit?: number; // Percentage (0-100)
  minSize?: number; // Minimum size in pixels
  maxSize?: number; // Maximum size in pixels
  className?: string;
  onSplitChange?: (split: number) => void;
  persistKey?: string; // Key for localStorage persistence
  disabled?: boolean;
}

const STORAGE_PREFIX = 'crewchief-splitpane-';

export const SplitPane: React.FC<SplitPaneProps> = ({
  children,
  direction = 'horizontal',
  initialSplit = 50,
  minSize = 100,
  maxSize,
  className = '',
  onSplitChange,
  persistKey,
  disabled = false,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [split, setSplit] = useState(initialSplit);
  const [startPos, setStartPos] = useState(0);
  const [startSplit, setStartSplit] = useState(0);

  // Load persisted split position
  useEffect(() => {
    if (persistKey) {
      const saved = localStorage.getItem(`${STORAGE_PREFIX}${persistKey}`);
      if (saved) {
        const savedSplit = parseFloat(saved);
        if (savedSplit >= 0 && savedSplit <= 100) {
          setSplit(savedSplit);
        }
      }
    }
  }, [persistKey]);

  // Save split position to localStorage
  const saveSplit = useCallback((newSplit: number) => {
    if (persistKey) {
      localStorage.setItem(`${STORAGE_PREFIX}${persistKey}`, newSplit.toString());
    }
  }, [persistKey]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (disabled) return;
    
    e.preventDefault();
    setIsDragging(true);
    setStartPos(direction === 'horizontal' ? e.clientX : e.clientY);
    setStartSplit(split);
    
    // Add cursor style to body
    document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';
  }, [disabled, direction, split]);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging || !containerRef.current) return;

    const container = containerRef.current;
    const containerRect = container.getBoundingClientRect();
    const containerSize = direction === 'horizontal' 
      ? containerRect.width 
      : containerRect.height;

    const currentPos = direction === 'horizontal' ? e.clientX : e.clientY;
    const containerStart = direction === 'horizontal' 
      ? containerRect.left 
      : containerRect.top;
    
    const delta = currentPos - startPos;
    const deltaPercent = (delta / containerSize) * 100;
    let newSplit = startSplit + deltaPercent;

    // Apply min/max constraints
    const minPercent = (minSize / containerSize) * 100;
    const maxPercent = maxSize ? (maxSize / containerSize) * 100 : 100 - minPercent;

    newSplit = Math.max(minPercent, Math.min(maxPercent, newSplit));
    newSplit = Math.max(0, Math.min(100, newSplit));

    setSplit(newSplit);
    onSplitChange?.(newSplit);
  }, [isDragging, direction, startPos, startSplit, minSize, maxSize, onSplitChange]);

  const handleMouseUp = useCallback(() => {
    if (!isDragging) return;

    setIsDragging(false);
    saveSplit(split);
    
    // Remove cursor style from body
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }, [isDragging, split, saveSplit]);

  // Handle double-click to reset
  const handleDoubleClick = useCallback(() => {
    if (disabled) return;
    
    const resetSplit = 50;
    setSplit(resetSplit);
    onSplitChange?.(resetSplit);
    saveSplit(resetSplit);
  }, [disabled, onSplitChange, saveSplit]);

  // Handle keyboard navigation
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (disabled) return;
    
    let newSplit = split;
    const step = 5; // 5% steps
    
    switch (e.key) {
      case 'ArrowLeft':
      case 'ArrowUp':
        e.preventDefault();
        newSplit = Math.max(0, split - step);
        break;
      case 'ArrowRight':
      case 'ArrowDown':
        e.preventDefault();
        newSplit = Math.min(100, split + step);
        break;
      case 'Home':
        e.preventDefault();
        newSplit = 0;
        break;
      case 'End':
        e.preventDefault();
        newSplit = 100;
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        newSplit = 50;
        break;
      default:
        return;
    }
    
    setSplit(newSplit);
    onSplitChange?.(newSplit);
    saveSplit(newSplit);
  }, [disabled, split, onSplitChange, saveSplit]);

  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isDragging, handleMouseMove, handleMouseUp]);

  const isHorizontal = direction === 'horizontal';
  const firstPaneStyle = {
    [isHorizontal ? 'width' : 'height']: `${split}%`,
  };
  const secondPaneStyle = {
    [isHorizontal ? 'width' : 'height']: `${100 - split}%`,
  };

  const dividerClasses = `
    group relative flex-shrink-0 bg-gray-200 dark:bg-gray-700 
    transition-colors duration-200 hover:bg-gray-300 dark:hover:bg-gray-600
    ${isHorizontal ? 'w-2 cursor-col-resize' : 'h-2 cursor-row-resize'}
    ${isDragging ? 'bg-primary-500 dark:bg-primary-400' : ''}
    ${disabled ? 'cursor-not-allowed opacity-50' : ''}
  `;

  const gripClasses = `
    absolute inset-0 flex items-center justify-center text-gray-400 
    group-hover:text-gray-600 dark:group-hover:text-gray-300
    transition-colors duration-200
    ${isDragging ? 'text-white' : ''}
  `;

  return (
    <div
      ref={containerRef}
      className={`flex ${isHorizontal ? 'flex-row' : 'flex-col'} h-full w-full ${className}`}
      role="separator"
      aria-orientation={direction}
      aria-label="Resizable pane divider"
    >
      {/* First pane */}
      <div
        className="flex-shrink-0 overflow-hidden"
        style={firstPaneStyle}
      >
        {children[0]}
      </div>

      {/* Divider */}
      <div
        className={dividerClasses}
        onMouseDown={handleMouseDown}
        onDoubleClick={handleDoubleClick}
        onKeyDown={handleKeyDown}
        tabIndex={disabled ? -1 : 0}
        role="separator"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(split)}
        aria-label={`Resize panes. Current split: ${Math.round(split)}%. Use arrow keys to adjust, Enter to reset to 50%, double-click to reset.`}
        title={`Current split: ${Math.round(split)}%. Double-click to reset. Use keyboard arrows to adjust.`}
      >
        <div className={gripClasses}>
          {isHorizontal ? (
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
            </svg>
          ) : (
            <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h.01M12 12h.01M19 12h.01M6 12a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0z" />
            </svg>
          )}
        </div>
      </div>

      {/* Second pane */}
      <div
        className="flex-shrink-0 overflow-hidden"
        style={secondPaneStyle}
      >
        {children[1]}
      </div>

      {/* Accessibility instructions */}
      <div className="sr-only" aria-live="polite">
        {isDragging && `Resizing panes: ${Math.round(split)}%`}
      </div>
    </div>
  );
};