import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Search, X, Clock, Loader2 } from 'lucide-react';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';

export interface SearchHistory {
  query: string;
  timestamp: number;
  filters?: any;
}

export interface SearchBarProps {
  value: string;
  onChange: (value: string) => void;
  onSearch: (query: string) => void;
  loading?: boolean;
  placeholder?: string;
  className?: string;
  debounceMs?: number;
  showHistory?: boolean;
  maxHistoryItems?: number;
}

export const SearchBar: React.FC<SearchBarProps> = ({
  value,
  onChange,
  onSearch,
  loading = false,
  placeholder = "Search for functions, classes, or concepts...",
  className,
  debounceMs = 300,
  showHistory = true,
  maxHistoryItems = 20,
}) => {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const [searchHistory, setSearchHistory] = useState<SearchHistory[]>([]);
  const [selectedHistoryIndex, setSelectedHistoryIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<NodeJS.Timeout>();

  // Load search history from localStorage
  useEffect(() => {
    if (!showHistory) return;
    
    try {
      const saved = localStorage.getItem('crewchief-search-history');
      if (saved) {
        const history = JSON.parse(saved);
        setSearchHistory(Array.isArray(history) ? history : []);
      }
    } catch (error) {
      console.warn('Failed to load search history:', error);
    }
  }, [showHistory]);

  // Save search history to localStorage
  const saveSearchHistory = useCallback((history: SearchHistory[]) => {
    if (!showHistory) return;
    
    try {
      localStorage.setItem('crewchief-search-history', JSON.stringify(history));
    } catch (error) {
      console.warn('Failed to save search history:', error);
    }
  }, [showHistory]);

  // Add to search history
  const addToHistory = useCallback((query: string) => {
    if (!showHistory || !query.trim()) return;

    const newEntry: SearchHistory = {
      query: query.trim(),
      timestamp: Date.now(),
    };

    setSearchHistory(prev => {
      // Remove duplicate if exists
      const filtered = prev.filter(item => item.query !== newEntry.query);
      // Add to beginning and limit to maxHistoryItems
      const updated = [newEntry, ...filtered].slice(0, maxHistoryItems);
      saveSearchHistory(updated);
      return updated;
    });
  }, [showHistory, maxHistoryItems, saveSearchHistory]);

  // Clear search history
  const clearHistory = useCallback(() => {
    setSearchHistory([]);
    saveSearchHistory([]);
  }, [saveSearchHistory]);

  // Debounced search
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (value.trim()) {
      debounceRef.current = setTimeout(() => {
        onSearch(value.trim());
      }, debounceMs);
    }

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [value, onSearch, debounceMs]);

  // Handle input change
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    onChange(newValue);
    setSelectedHistoryIndex(-1);
  };

  // Handle form submission
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim()) {
      addToHistory(value);
      onSearch(value.trim());
      setIsDropdownOpen(false);
    }
  };

  // Handle focus
  const handleFocus = () => {
    if (showHistory && searchHistory.length > 0) {
      setIsDropdownOpen(true);
    }
  };

  // Handle blur
  const handleBlur = (e: React.FocusEvent) => {
    // Don't close if clicking within dropdown
    if (dropdownRef.current?.contains(e.relatedTarget as Node)) {
      return;
    }
    setTimeout(() => setIsDropdownOpen(false), 150);
  };

  // Handle keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!isDropdownOpen || searchHistory.length === 0) {
      if (e.key === 'Escape') {
        onChange('');
        setIsDropdownOpen(false);
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedHistoryIndex(prev => 
          prev < searchHistory.length - 1 ? prev + 1 : prev
        );
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedHistoryIndex(prev => prev > 0 ? prev - 1 : -1);
        break;
      case 'Enter':
        e.preventDefault();
        if (selectedHistoryIndex >= 0) {
          const selectedQuery = searchHistory[selectedHistoryIndex].query;
          onChange(selectedQuery);
          onSearch(selectedQuery);
          setIsDropdownOpen(false);
        } else {
          handleSubmit(e);
        }
        break;
      case 'Escape':
        setIsDropdownOpen(false);
        setSelectedHistoryIndex(-1);
        break;
    }
  };

  // Handle history item click
  const handleHistoryClick = (query: string) => {
    onChange(query);
    onSearch(query);
    setIsDropdownOpen(false);
    inputRef.current?.focus();
  };

  // Clear input
  const handleClear = () => {
    onChange('');
    setIsDropdownOpen(false);
    inputRef.current?.focus();
  };

  return (
    <div className={cn("relative w-full", className)}>
      <form onSubmit={handleSubmit} className="relative">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={inputRef}
            type="text"
            value={value}
            onChange={handleInputChange}
            onFocus={handleFocus}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            className="pl-10 pr-20"
            disabled={loading}
          />
          <div className="absolute right-2 top-1/2 flex -translate-y-1/2 items-center gap-1">
            {loading && (
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
            )}
            {value && !loading && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0"
                onClick={handleClear}
              >
                <X className="h-3 w-3" />
              </Button>
            )}
            <Button
              type="submit"
              variant="ghost"
              size="sm"
              className="h-6 px-2"
              disabled={loading || !value.trim()}
            >
              {loading ? 'Searching...' : 'Search'}
            </Button>
          </div>
        </div>
      </form>

      {/* Search History Dropdown */}
      {showHistory && isDropdownOpen && searchHistory.length > 0 && (
        <div
          ref={dropdownRef}
          className="absolute z-50 mt-1 w-full rounded-md border bg-popover p-1 shadow-md"
        >
          <div className="flex items-center justify-between px-2 py-1 text-xs text-muted-foreground">
            <span>Recent searches</span>
            <Button
              variant="ghost"
              size="sm"
              className="h-auto p-1 text-xs"
              onClick={clearHistory}
            >
              Clear
            </Button>
          </div>
          <div className="max-h-60 overflow-y-auto">
            {searchHistory.map((item, index) => (
              <button
                key={`${item.query}-${item.timestamp}`}
                className={cn(
                  "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent hover:text-accent-foreground",
                  selectedHistoryIndex === index && "bg-accent text-accent-foreground"
                )}
                onClick={() => handleHistoryClick(item.query)}
              >
                <Clock className="h-3 w-3 text-muted-foreground" />
                <span className="flex-1 truncate">{item.query}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};