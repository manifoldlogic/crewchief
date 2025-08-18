import React, { useState, useRef, useCallback, useMemo } from 'react';
import { AlertCircle, HelpCircle, Settings, Loader2 } from 'lucide-react';
import { SearchBar } from '../components/search/SearchBar';
import { FilterPanel } from '../components/search/FilterPanel';
import { SearchResults } from '../components/search/SearchResults';
import { CodePreview } from '../components/search/CodePreview';
import { Button } from '../components/ui/button';
import { useMaproomSearch, SearchFilters } from '../hooks/useMaproomSearch';
import { useSearchKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import { exportToJSON, exportToCSV, copyToClipboard, validateExportData } from '../utils/exportUtils';
import { cn } from '../lib/utils';

const DEFAULT_FILTERS: SearchFilters = {
  languages: [],
  fileTypes: [],
  paths: [],
  dateRange: {},
  relevanceThreshold: 0.0,
  maxResults: 100,
};

const Search: React.FC = () => {
  // State
  const [searchQuery, setSearchQuery] = useState('');
  const [filters, setFilters] = useState<SearchFilters>(DEFAULT_FILTERS);
  const [selectedResults, setSelectedResults] = useState<Set<string>>(new Set());
  const [expandedPreview, setExpandedPreview] = useState<string | null>(null);
  const [filtersCollapsed, setFiltersCollapsed] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [exportLoading, setExportLoading] = useState(false);

  // Refs
  const searchBarRef = useRef<HTMLInputElement>(null);

  // Maproom search hook
  const {
    results,
    loading,
    error,
    executionTime,
    totalCount,
    search,
    clearResults,
    clearError,
    retry,
    hasSearched,
    isRetrying,
  } = useMaproomSearch({
    debounceMs: 300,
    maxRetries: 2,
    cacheResults: true,
  });

  // Search handlers
  const handleSearch = useCallback((query: string) => {
    search(query, filters);
  }, [search, filters]);

  const handleQueryChange = useCallback((query: string) => {
    setSearchQuery(query);
  }, []);

  const handleFiltersChange = useCallback((newFilters: SearchFilters) => {
    setFilters(newFilters);
    if (searchQuery.trim()) {
      search(searchQuery, newFilters);
    }
  }, [search, searchQuery]);

  const handleFiltersReset = useCallback(() => {
    setFilters(DEFAULT_FILTERS);
    if (searchQuery.trim()) {
      search(searchQuery, DEFAULT_FILTERS);
    }
  }, [search, searchQuery]);

  // Result handlers
  const handleResultClick = useCallback((result: any) => {
    setExpandedPreview(expandedPreview === result.id ? null : result.id);
  }, [expandedPreview]);

  const handleResultSelect = useCallback((result: any) => {
    setSelectedResults(prev => {
      const newSet = new Set(prev);
      if (newSet.has(result.id)) {
        newSet.delete(result.id);
      } else {
        newSet.add(result.id);
      }
      return newSet;
    });
  }, []);

  // Export handlers
  const handleExport = useCallback(async (exportResults: any[], format: 'json' | 'csv') => {
    if (!exportResults.length) return;

    setExportLoading(true);
    try {
      validateExportData(exportResults);
      
      if (format === 'json') {
        await exportToJSON(exportResults, {
          includeContext: true,
          includeMetadata: true,
          timestamp: true,
        });
      } else {
        await exportToCSV(exportResults, {
          includeContext: false,
          timestamp: true,
        });
      }
    } catch (error) {
      console.error('Export failed:', error);
      // You could show a toast notification here
    } finally {
      setExportLoading(false);
    }
  }, []);

  const handleCopyResults = useCallback(async (copyResults: any[]) => {
    if (!copyResults.length) return;

    try {
      await copyToClipboard(copyResults, 'text');
      // You could show a toast notification here
    } catch (error) {
      console.error('Copy failed:', error);
    }
  }, []);

  // Keyboard shortcuts
  const focusSearch = useCallback(() => {
    searchBarRef.current?.focus();
  }, []);

  const clearSearch = useCallback(() => {
    setSearchQuery('');
    clearResults();
    setSelectedResults(new Set());
    setExpandedPreview(null);
  }, [clearResults]);

  const toggleFilters = useCallback(() => {
    setFiltersCollapsed(!filtersCollapsed);
  }, [filtersCollapsed]);

  const exportSelectedResults = useCallback(() => {
    const selectedItems = results.filter(r => selectedResults.has(r.id));
    const exportData = selectedItems.length > 0 ? selectedItems : results;
    if (exportData.length > 0) {
      handleExport(exportData, 'json');
    }
  }, [results, selectedResults, handleExport]);

  useSearchKeyboardShortcuts({
    focusSearch,
    clearSearch,
    toggleFilters,
    exportResults: exportSelectedResults,
  });

  // Get expanded preview result
  const expandedResult = useMemo(() => {
    return expandedPreview ? results.find(r => r.id === expandedPreview) : null;
  }, [expandedPreview, results]);

  // Performance metrics display
  const performanceText = useMemo(() => {
    if (!hasSearched) return '';
    return `${totalCount} results in ${executionTime}ms`;
  }, [hasSearched, totalCount, executionTime]);

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <div className="border-b bg-card">
        <div className="container mx-auto px-4 py-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h1 className="text-2xl font-bold">Code Search</h1>
              <p className="text-muted-foreground">
                Search through your codebase using semantic search powered by Maproom
              </p>
            </div>
            
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowHelp(!showHelp)}
              >
                <HelpCircle className="h-4 w-4 mr-1" />
                Help
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={toggleFilters}
              >
                <Settings className="h-4 w-4 mr-1" />
                Filters
              </Button>
            </div>
          </div>

          {/* Search Bar */}
          <div className="mb-4">
            <SearchBar
              value={searchQuery}
              onChange={handleQueryChange}
              onSearch={handleSearch}
              loading={loading}
              placeholder="Search for functions, classes, concepts, or code patterns..."
              debounceMs={300}
              showHistory={true}
              maxHistoryItems={20}
            />
          </div>

          {/* Performance Info */}
          {(performanceText || error) && (
            <div className="flex items-center justify-between text-sm text-muted-foreground">
              <div className="flex items-center gap-4">
                {performanceText && <span>{performanceText}</span>}
                {error && (
                  <div className="flex items-center gap-1 text-destructive">
                    <AlertCircle className="h-4 w-4" />
                    <span>{error.message}</span>
                    <Button variant="ghost" size="sm" onClick={retry} disabled={isRetrying}>
                      {isRetrying ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        'Retry'
                      )}
                    </Button>
                  </div>
                )}
              </div>
              
              {selectedResults.size > 0 && (
                <span>{selectedResults.size} selected</span>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Main Content */}
      <div className="container mx-auto px-4 py-6">
        <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
          {/* Filters Sidebar */}
          {!filtersCollapsed && (
            <div className="lg:col-span-1">
              <FilterPanel
                filters={filters}
                onChange={handleFiltersChange}
                onReset={handleFiltersReset}
                collapsed={false}
                onToggleCollapse={() => setFiltersCollapsed(true)}
              />
            </div>
          )}

          {/* Results Area */}
          <div className={cn("space-y-6", filtersCollapsed ? "lg:col-span-4" : "lg:col-span-3")}>
            {/* Collapsed Filters */}
            {filtersCollapsed && (
              <FilterPanel
                filters={filters}
                onChange={handleFiltersChange}
                onReset={handleFiltersReset}
                collapsed={true}
                onToggleCollapse={() => setFiltersCollapsed(false)}
              />
            )}

            {/* Search Results */}
            <SearchResults
              results={results}
              loading={loading}
              error={error?.message}
              onResultClick={handleResultClick}
              onResultSelect={handleResultSelect}
              selectedResults={selectedResults}
              itemHeight={180}
              pageSize={25}
              showPagination={true}
              onExport={handleExport}
              onCopyResults={handleCopyResults}
            />

            {/* Code Preview */}
            {expandedResult && (
              <div className="mt-6">
                <CodePreview
                  content={expandedResult.content}
                  filePath={expandedResult.file_path}
                  language={expandedResult.language}
                  lineStart={expandedResult.line_start}
                  lineEnd={expandedResult.line_end}
                  searchTerms={searchQuery ? [searchQuery] : []}
                  expanded={true}
                  showLineNumbers={true}
                  title={`Preview: ${expandedResult.file_path}`}
                  onToggleExpanded={() => setExpandedPreview(null)}
                />
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Help Modal */}
      {showHelp && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="fixed inset-0 bg-black/50" onClick={() => setShowHelp(false)} />
          <div className="relative bg-card border rounded-lg p-6 max-w-md w-full mx-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold">Keyboard Shortcuts</h3>
              <Button variant="ghost" size="sm" onClick={() => setShowHelp(false)}>
                ×
              </Button>
            </div>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span>Focus search</span>
                <code className="text-xs bg-muted px-1 rounded">Cmd/Ctrl + K</code>
              </div>
              <div className="flex justify-between">
                <span>Clear search</span>
                <code className="text-xs bg-muted px-1 rounded">Escape</code>
              </div>
              <div className="flex justify-between">
                <span>Toggle filters</span>
                <code className="text-xs bg-muted px-1 rounded">Cmd/Ctrl + F</code>
              </div>
              <div className="flex justify-between">
                <span>Export results</span>
                <code className="text-xs bg-muted px-1 rounded">Cmd/Ctrl + E</code>
              </div>
              <div className="flex justify-between">
                <span>Navigate history</span>
                <code className="text-xs bg-muted px-1 rounded">↑/↓</code>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default Search;