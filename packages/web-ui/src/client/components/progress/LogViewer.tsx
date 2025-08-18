import React, { useEffect, useState, useRef, useCallback, useMemo } from 'react';
import { FixedSizeList as List } from 'react-window';
import { motion, AnimatePresence } from 'framer-motion';
import { Search, Download, Filter, Pause, Play, Trash2, ChevronDown, ChevronUp } from 'lucide-react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/button';
import { Input } from '../ui/input';

export interface LogEntry {
  id: string;
  timestamp: Date;
  level: 'debug' | 'info' | 'warn' | 'error';
  source: string;
  message: string;
  metadata?: Record<string, any>;
}

export interface LogViewerProps {
  /** Log entries to display */
  logs: LogEntry[];
  /** Height of the log viewer */
  height?: number;
  /** Whether logs are being streamed */
  isStreaming?: boolean;
  /** Whether to auto-scroll to bottom */
  autoScroll?: boolean;
  /** Maximum number of logs to keep in memory */
  maxLogs?: number;
  /** Search placeholder text */
  searchPlaceholder?: string;
  /** Custom className */
  className?: string;
  /** Custom row height */
  rowHeight?: number;
  /** Show search functionality */
  showSearch?: boolean;
  /** Show controls (pause, clear, etc.) */
  showControls?: boolean;
  /** Show log levels filter */
  showLevelFilter?: boolean;
  /** Show source filter */
  showSourceFilter?: boolean;
  /** Show export functionality */
  showExport?: boolean;
  /** Callback when streaming is paused/resumed */
  onStreamingToggle?: (paused: boolean) => void;
  /** Callback when logs are cleared */
  onClear?: () => void;
  /** Callback when logs are exported */
  onExport?: (logs: LogEntry[]) => void;
  /** Custom log entry renderer */
  renderLogEntry?: (log: LogEntry, index: number) => React.ReactNode;
}

interface LogViewerState {
  searchTerm: string;
  levelFilter: Set<LogEntry['level']>;
  sourceFilter: string;
  isPaused: boolean;
  isSearchFocused: boolean;
  userScrolled: boolean;
}

// Performance-optimized log entry component
const LogEntryRow: React.FC<{
  index: number;
  style: React.CSSProperties;
  data: {
    logs: LogEntry[];
    searchTerm: string;
    highlightTerm: (text: string, term: string) => React.ReactNode;
  };
}> = ({ index, style, data }) => {
  const log = data.logs[index];
  const isNewEntry = useRef(false);

  // Check if this is a new entry (for highlight animation)
  useEffect(() => {
    isNewEntry.current = true;
    const timer = setTimeout(() => {
      isNewEntry.current = false;
    }, 1000);
    return () => clearTimeout(timer);
  }, [log.id]);

  const levelColors = {
    debug: 'text-gray-500 dark:text-gray-400',
    info: 'text-blue-600 dark:text-blue-400',
    warn: 'text-yellow-600 dark:text-yellow-400',
    error: 'text-red-600 dark:text-red-400',
  };

  const levelBgColors = {
    debug: 'bg-gray-100 dark:bg-gray-800',
    info: 'bg-blue-50 dark:bg-blue-900/20',
    warn: 'bg-yellow-50 dark:bg-yellow-900/20',
    error: 'bg-red-50 dark:bg-red-900/20',
  };

  const formatTimestamp = (timestamp: Date): string => {
    return timestamp.toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      fractionalSecondDigits: 3,
    });
  };

  return (
    <motion.div
      style={style}
      className={cn(
        'flex items-start space-x-3 px-3 py-2 text-sm font-mono border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-900/50',
        levelBgColors[log.level],
        isNewEntry.current && 'animate-pulse',
      )}
      initial={isNewEntry.current ? { backgroundColor: 'rgb(59 130 246 / 0.1)' } : false}
      animate={isNewEntry.current ? { backgroundColor: 'transparent' } : false}
      transition={{ duration: 1 }}
    >
      {/* Timestamp */}
      <span className="text-xs text-gray-500 dark:text-gray-400 shrink-0 w-20">
        {formatTimestamp(log.timestamp)}
      </span>

      {/* Level */}
      <span className={cn(
        'text-xs font-bold uppercase shrink-0 w-12',
        levelColors[log.level],
      )}>
        {log.level}
      </span>

      {/* Source */}
      <span className="text-xs text-gray-600 dark:text-gray-300 shrink-0 w-20 truncate" title={log.source}>
        {log.source}
      </span>

      {/* Message */}
      <span className="flex-1 break-words">
        {data.searchTerm ? data.highlightTerm(log.message, data.searchTerm) : log.message}
      </span>

      {/* Metadata indicator */}
      {log.metadata && Object.keys(log.metadata).length > 0 && (
        <span className="text-xs text-gray-400 shrink-0">
          📎
        </span>
      )}
    </motion.div>
  );
};

export const LogViewer: React.FC<LogViewerProps> = ({
  logs,
  height = 400,
  isStreaming = false,
  autoScroll = true,
  maxLogs = 10000,
  searchPlaceholder = 'Search logs...',
  className,
  rowHeight = 32,
  showSearch = true,
  showControls = true,
  showLevelFilter = true,
  showSourceFilter = true,
  showExport = true,
  onStreamingToggle,
  onClear,
  onExport,
  renderLogEntry,
}) => {
  const [state, setState] = useState<LogViewerState>({
    searchTerm: '',
    levelFilter: new Set(['debug', 'info', 'warn', 'error']),
    sourceFilter: '',
    isPaused: false,
    isSearchFocused: false,
    userScrolled: false,
  });

  const listRef = useRef<List>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const lastLogCountRef = useRef(logs.length);

  // Filter and search logs
  const filteredLogs = useMemo(() => {
    let filtered = logs.slice(-maxLogs); // Keep only recent logs for memory efficiency

    // Apply level filter
    filtered = filtered.filter(log => state.levelFilter.has(log.level));

    // Apply source filter
    if (state.sourceFilter) {
      filtered = filtered.filter(log => 
        log.source.toLowerCase().includes(state.sourceFilter.toLowerCase())
      );
    }

    // Apply search filter
    if (state.searchTerm) {
      const searchLower = state.searchTerm.toLowerCase();
      filtered = filtered.filter(log => 
        log.message.toLowerCase().includes(searchLower) ||
        log.source.toLowerCase().includes(searchLower)
      );
    }

    return filtered;
  }, [logs, state.levelFilter, state.sourceFilter, state.searchTerm, maxLogs]);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (
      autoScroll && 
      !state.isPaused && 
      !state.userScrolled && 
      filteredLogs.length > 0 &&
      logs.length > lastLogCountRef.current
    ) {
      listRef.current?.scrollToItem(filteredLogs.length - 1, 'end');
    }
    lastLogCountRef.current = logs.length;
  }, [filteredLogs.length, autoScroll, state.isPaused, state.userScrolled, logs.length]);

  // Highlight search terms
  const highlightTerm = useCallback((text: string, term: string): React.ReactNode => {
    if (!term) return text;
    
    const regex = new RegExp(`(${term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    const parts = text.split(regex);
    
    return parts.map((part, index) => 
      regex.test(part) ? (
        <mark key={index} className="bg-yellow-200 dark:bg-yellow-900/50 px-0.5 rounded">
          {part}
        </mark>
      ) : part
    );
  }, []);

  // Handle search
  const handleSearch = (term: string) => {
    setState(prev => ({ ...prev, searchTerm: term }));
  };

  // Handle level filter toggle
  const handleLevelToggle = (level: LogEntry['level']) => {
    setState(prev => {
      const newLevelFilter = new Set(prev.levelFilter);
      if (newLevelFilter.has(level)) {
        newLevelFilter.delete(level);
      } else {
        newLevelFilter.add(level);
      }
      return { ...prev, levelFilter: newLevelFilter };
    });
  };

  // Handle streaming toggle
  const handleStreamingToggle = () => {
    const newPaused = !state.isPaused;
    setState(prev => ({ ...prev, isPaused: newPaused }));
    onStreamingToggle?.(newPaused);
  };

  // Handle clear logs
  const handleClear = () => {
    onClear?.();
  };

  // Handle export logs
  const handleExport = () => {
    onExport?.(filteredLogs);
  };

  // Handle scroll
  const handleScroll = ({ scrollDirection, scrollOffset }: any) => {
    const isAtBottom = scrollOffset >= (filteredLogs.length - 1) * rowHeight - height + rowHeight;
    setState(prev => ({ 
      ...prev, 
      userScrolled: scrollDirection === 'backward' || !isAtBottom 
    }));
  };

  // Handle scroll to bottom
  const scrollToBottom = () => {
    listRef.current?.scrollToItem(filteredLogs.length - 1, 'end');
    setState(prev => ({ ...prev, userScrolled: false }));
  };

  // Handle scroll to top
  const scrollToTop = () => {
    listRef.current?.scrollToItem(0, 'start');
  };

  // Get unique sources for filtering
  const sources = useMemo(() => {
    const sourceSet = new Set(logs.map(log => log.source));
    return Array.from(sourceSet).sort();
  }, [logs]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        switch (e.key) {
          case 'f':
            e.preventDefault();
            searchInputRef.current?.focus();
            break;
          case 'k':
            e.preventDefault();
            handleClear();
            break;
          case 's':
            e.preventDefault();
            handleExport();
            break;
        }
      }
      
      if (e.key === 'Escape' && state.isSearchFocused) {
        searchInputRef.current?.blur();
        setState(prev => ({ ...prev, searchTerm: '', isSearchFocused: false }));
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [state.isSearchFocused]);

  return (
    <div className={cn('flex flex-col border rounded-lg bg-background', className)}>
      {/* Header with controls */}
      {(showSearch || showControls) && (
        <div className="flex items-center justify-between p-3 border-b bg-muted/50">
          {/* Search and filters */}
          <div className="flex items-center space-x-2 flex-1">
            {showSearch && (
              <div className="relative">
                <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  ref={searchInputRef}
                  placeholder={searchPlaceholder}
                  value={state.searchTerm}
                  onChange={(e) => handleSearch(e.target.value)}
                  onFocus={() => setState(prev => ({ ...prev, isSearchFocused: true }))}
                  onBlur={() => setState(prev => ({ ...prev, isSearchFocused: false }))}
                  className="pl-8 w-64"
                />
              </div>
            )}

            {showLevelFilter && (
              <div className="flex items-center space-x-1">
                {(['debug', 'info', 'warn', 'error'] as const).map(level => (
                  <Button
                    key={level}
                    size="sm"
                    variant={state.levelFilter.has(level) ? 'default' : 'outline'}
                    onClick={() => handleLevelToggle(level)}
                    className="text-xs h-7"
                  >
                    {level}
                  </Button>
                ))}
              </div>
            )}

            {showSourceFilter && sources.length > 0 && (
              <select
                value={state.sourceFilter}
                onChange={(e) => setState(prev => ({ ...prev, sourceFilter: e.target.value }))}
                className="h-7 text-xs border rounded px-2 bg-background"
              >
                <option value="">All sources</option>
                {sources.map(source => (
                  <option key={source} value={source}>{source}</option>
                ))}
              </select>
            )}
          </div>

          {/* Controls */}
          {showControls && (
            <div className="flex items-center space-x-1">
              <Button
                size="sm"
                variant="outline"
                onClick={scrollToTop}
                className="h-7 w-7 p-0"
                title="Scroll to top"
              >
                <ChevronUp className="h-3 w-3" />
              </Button>
              
              <Button
                size="sm"
                variant="outline"
                onClick={scrollToBottom}
                className="h-7 w-7 p-0"
                title="Scroll to bottom"
              >
                <ChevronDown className="h-3 w-3" />
              </Button>

              <Button
                size="sm"
                variant="outline"
                onClick={handleStreamingToggle}
                className="h-7 w-7 p-0"
                title={state.isPaused ? 'Resume streaming' : 'Pause streaming'}
              >
                {state.isPaused ? <Play className="h-3 w-3" /> : <Pause className="h-3 w-3" />}
              </Button>

              <Button
                size="sm"
                variant="outline"
                onClick={handleClear}
                className="h-7 w-7 p-0"
                title="Clear logs (Ctrl+K)"
              >
                <Trash2 className="h-3 w-3" />
              </Button>

              {showExport && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={handleExport}
                  className="h-7 w-7 p-0"
                  title="Export logs (Ctrl+S)"
                >
                  <Download className="h-3 w-3" />
                </Button>
              )}
            </div>
          )}
        </div>
      )}

      {/* Status bar */}
      <div className="flex items-center justify-between px-3 py-1 text-xs text-muted-foreground bg-muted/30 border-b">
        <div className="flex items-center space-x-4">
          <span>{filteredLogs.length} of {logs.length} logs</span>
          {state.searchTerm && (
            <span>Searching: "{state.searchTerm}"</span>
          )}
          {state.sourceFilter && (
            <span>Source: {state.sourceFilter}</span>
          )}
        </div>
        
        <div className="flex items-center space-x-2">
          {isStreaming && !state.isPaused && (
            <motion.div
              className="flex items-center space-x-1"
              animate={{ opacity: [1, 0.5, 1] }}
              transition={{ duration: 1, repeat: Infinity }}
            >
              <div className="w-2 h-2 bg-green-500 rounded-full" />
              <span>Live</span>
            </motion.div>
          )}
          {state.isPaused && (
            <div className="flex items-center space-x-1 text-yellow-600">
              <Pause className="h-3 w-3" />
              <span>Paused</span>
            </div>
          )}
        </div>
      </div>

      {/* Log list */}
      <div className="flex-1 relative">
        {filteredLogs.length === 0 ? (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <div className="text-center">
              <div className="text-2xl mb-2">📝</div>
              <div>No logs to display</div>
              {state.searchTerm && (
                <div className="text-xs mt-1">Try a different search term</div>
              )}
            </div>
          </div>
        ) : (
          <List
            ref={listRef}
            height={height - (showSearch || showControls ? 80 : 30)}
            itemCount={filteredLogs.length}
            itemSize={rowHeight}
            onScroll={handleScroll}
            itemData={{
              logs: filteredLogs,
              searchTerm: state.searchTerm,
              highlightTerm,
            }}
          >
            {renderLogEntry || LogEntryRow}
          </List>
        )}

        {/* Scroll to bottom indicator */}
        <AnimatePresence>
          {state.userScrolled && filteredLogs.length > 0 && (
            <motion.div
              className="absolute bottom-4 right-4"
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.8 }}
            >
              <Button
                size="sm"
                onClick={scrollToBottom}
                className="shadow-lg"
              >
                <ChevronDown className="h-4 w-4 mr-1" />
                New logs
              </Button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
};

export default LogViewer;