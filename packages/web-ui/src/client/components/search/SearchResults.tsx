import React, { useState, useMemo, useCallback } from 'react';
import { FixedSizeList as List } from 'react-window';
import { 
  ChevronLeft, 
  ChevronRight, 
  ChevronsLeft, 
  ChevronsRight, 
  FileText, 
  Star,
  ExternalLink,
  Copy,
  Download
} from 'lucide-react';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';

export interface SearchResult {
  id: string;
  file_path: string;
  line_start: number;
  line_end: number;
  content: string;
  relevance_score: number;
  language?: string;
  chunk_type?: string;
  context?: {
    before?: string;
    after?: string;
  };
}

export interface SearchResultsProps {
  results: SearchResult[];
  loading?: boolean;
  error?: string;
  onResultClick?: (result: SearchResult) => void;
  onResultSelect?: (result: SearchResult) => void;
  selectedResults?: Set<string>;
  className?: string;
  itemHeight?: number;
  pageSize?: number;
  showPagination?: boolean;
  onExport?: (results: SearchResult[], format: 'json' | 'csv') => void;
  onCopyResults?: (results: SearchResult[]) => void;
}

interface ResultItemProps {
  index: number;
  style: React.CSSProperties;
  data: {
    results: SearchResult[];
    onResultClick?: (result: SearchResult) => void;
    onResultSelect?: (result: SearchResult) => void;
    selectedResults?: Set<string>;
  };
}

const ResultItem: React.FC<ResultItemProps> = ({ index, style, data }) => {
  const { results, onResultClick, onResultSelect, selectedResults } = data;
  const result = results[index];
  
  if (!result) return null;

  const isSelected = selectedResults?.has(result.id) || false;
  const scoreColor = result.relevance_score >= 0.8 ? 'text-green-600' : 
                   result.relevance_score >= 0.6 ? 'text-yellow-600' : 'text-gray-600';

  const handleClick = () => {
    onResultClick?.(result);
  };

  const handleSelect = (e: React.MouseEvent) => {
    e.stopPropagation();
    onResultSelect?.(result);
  };

  const handleCopyContent = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(result.content);
    } catch (error) {
      console.warn('Failed to copy to clipboard:', error);
    }
  };

  // Highlight search terms in content
  const highlightContent = (content: string) => {
    // This is a simple implementation - in a real app, you'd want to pass the search query
    // and highlight matching terms
    return content;
  };

  return (
    <div style={style}>
      <div
        className={cn(
          "group relative rounded-lg border p-4 transition-all hover:shadow-md cursor-pointer",
          isSelected && "border-primary bg-primary/5",
          "hover:border-primary/50"
        )}
        onClick={handleClick}
      >
        {/* Header */}
        <div className="flex items-start justify-between gap-4 mb-2">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <FileText className="h-4 w-4 text-muted-foreground flex-shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium truncate">{result.file_path}</p>
              <p className="text-xs text-muted-foreground">
                Line {result.line_start}
                {result.line_end !== result.line_start && `-${result.line_end}`}
                {result.language && (
                  <>
                    {' • '}
                    <span className="font-mono">{result.language}</span>
                  </>
                )}
                {result.chunk_type && (
                  <>
                    {' • '}
                    <span className="capitalize">{result.chunk_type}</span>
                  </>
                )}
              </p>
            </div>
          </div>
          
          {/* Relevance Score */}
          <div className="flex items-center gap-2 flex-shrink-0">
            <div className="flex items-center gap-1">
              <Star className={cn("h-3 w-3", scoreColor)} />
              <span className={cn("text-xs font-medium", scoreColor)}>
                {(result.relevance_score * 100).toFixed(0)}%
              </span>
            </div>
            
            {/* Actions */}
            <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              {onResultSelect && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0"
                  onClick={handleSelect}
                  title="Select result"
                >
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => {}}
                    className="h-3 w-3"
                  />
                </Button>
              )}
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0"
                onClick={handleCopyContent}
                title="Copy content"
              >
                <Copy className="h-3 w-3" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0"
                title="Open file"
              >
                <ExternalLink className="h-3 w-3" />
              </Button>
            </div>
          </div>
        </div>

        {/* Content Preview */}
        <div className="space-y-2">
          {result.context?.before && (
            <div className="text-xs text-muted-foreground font-mono bg-muted/50 p-2 rounded">
              {result.context.before}
            </div>
          )}
          
          <div className="text-sm font-mono bg-muted p-2 rounded">
            <pre className="whitespace-pre-wrap break-words">
              {highlightContent(result.content)}
            </pre>
          </div>
          
          {result.context?.after && (
            <div className="text-xs text-muted-foreground font-mono bg-muted/50 p-2 rounded">
              {result.context.after}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export const SearchResults: React.FC<SearchResultsProps> = ({
  results,
  loading = false,
  error,
  onResultClick,
  onResultSelect,
  selectedResults = new Set(),
  className,
  itemHeight = 180,
  pageSize = 25,
  showPagination = true,
  onExport,
  onCopyResults,
}) => {
  const [currentPage, setCurrentPage] = useState(1);

  // Pagination logic
  const totalPages = Math.ceil(results.length / pageSize);
  const startIndex = (currentPage - 1) * pageSize;
  const endIndex = Math.min(startIndex + pageSize, results.length);
  const currentResults = useMemo(
    () => results.slice(startIndex, endIndex),
    [results, startIndex, endIndex]
  );

  // Navigation handlers
  const goToPage = useCallback((page: number) => {
    setCurrentPage(Math.max(1, Math.min(page, totalPages)));
  }, [totalPages]);

  const goToFirstPage = useCallback(() => goToPage(1), [goToPage]);
  const goToLastPage = useCallback(() => goToPage(totalPages), [goToPage, totalPages]);
  const goToPreviousPage = useCallback(() => goToPage(currentPage - 1), [currentPage, goToPage]);
  const goToNextPage = useCallback(() => goToPage(currentPage + 1), [currentPage, goToPage]);

  // Export handlers
  const handleExportJSON = useCallback(() => {
    const selectedItems = results.filter(r => selectedResults.has(r.id));
    const exportData = selectedItems.length > 0 ? selectedItems : results;
    onExport?.(exportData, 'json');
  }, [results, selectedResults, onExport]);

  const handleExportCSV = useCallback(() => {
    const selectedItems = results.filter(r => selectedResults.has(r.id));
    const exportData = selectedItems.length > 0 ? selectedItems : results;
    onExport?.(exportData, 'csv');
  }, [results, selectedResults, onExport]);

  const handleCopyResults = useCallback(() => {
    const selectedItems = results.filter(r => selectedResults.has(r.id));
    const copyData = selectedItems.length > 0 ? selectedItems : results;
    onCopyResults?.(copyData);
  }, [results, selectedResults, onCopyResults]);

  // List data for virtualization
  const listData = useMemo(() => ({
    results: currentResults,
    onResultClick,
    onResultSelect,
    selectedResults,
  }), [currentResults, onResultClick, onResultSelect, selectedResults]);

  // Loading state
  if (loading) {
    return (
      <div className={cn("flex items-center justify-center py-12", className)}>
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
          <p className="text-muted-foreground">Searching...</p>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className={cn("text-center py-12", className)}>
        <div className="text-red-500 mb-2">Search Error</div>
        <p className="text-muted-foreground text-sm">{error}</p>
      </div>
    );
  }

  // Empty state
  if (results.length === 0) {
    return (
      <div className={cn("text-center py-12", className)}>
        <FileText className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
        <h3 className="text-lg font-medium mb-2">No results found</h3>
        <p className="text-muted-foreground">
          Try adjusting your search terms or filters
        </p>
      </div>
    );
  }

  return (
    <div className={cn("space-y-4", className)}>
      {/* Results Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h3 className="text-lg font-medium">
            Search Results ({results.length})
          </h3>
          {selectedResults.size > 0 && (
            <span className="text-sm text-muted-foreground">
              {selectedResults.size} selected
            </span>
          )}
        </div>
        
        {/* Export Actions */}
        {(onExport || onCopyResults) && (
          <div className="flex items-center gap-2">
            {onCopyResults && (
              <Button variant="outline" size="sm" onClick={handleCopyResults}>
                <Copy className="h-4 w-4 mr-1" />
                Copy
              </Button>
            )}
            {onExport && (
              <>
                <Button variant="outline" size="sm" onClick={handleExportJSON}>
                  <Download className="h-4 w-4 mr-1" />
                  JSON
                </Button>
                <Button variant="outline" size="sm" onClick={handleExportCSV}>
                  <Download className="h-4 w-4 mr-1" />
                  CSV
                </Button>
              </>
            )}
          </div>
        )}
      </div>

      {/* Virtualized List */}
      <div className="border rounded-lg">
        <List
          height={Math.min(600, currentResults.length * itemHeight)}
          itemCount={currentResults.length}
          itemSize={itemHeight}
          itemData={listData}
          className="scrollbar-thin scrollbar-thumb-muted-foreground/20"
        >
          {ResultItem}
        </List>
      </div>

      {/* Pagination */}
      {showPagination && totalPages > 1 && (
        <div className="flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            Showing {startIndex + 1}-{endIndex} of {results.length} results
          </div>
          
          <div className="flex items-center gap-1">
            <Button
              variant="outline"
              size="sm"
              onClick={goToFirstPage}
              disabled={currentPage === 1}
            >
              <ChevronsLeft className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={goToPreviousPage}
              disabled={currentPage === 1}
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            
            {/* Page Numbers */}
            <div className="flex items-center gap-1">
              {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
                const page = Math.max(1, Math.min(totalPages - 4, currentPage - 2)) + i;
                if (page > totalPages) return null;
                
                return (
                  <Button
                    key={page}
                    variant={currentPage === page ? "default" : "outline"}
                    size="sm"
                    onClick={() => goToPage(page)}
                    className="w-8"
                  >
                    {page}
                  </Button>
                );
              })}
            </div>
            
            <Button
              variant="outline"
              size="sm"
              onClick={goToNextPage}
              disabled={currentPage === totalPages}
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={goToLastPage}
              disabled={currentPage === totalPages}
            >
              <ChevronsRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};