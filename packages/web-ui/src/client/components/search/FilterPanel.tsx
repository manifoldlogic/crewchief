import React, { useState, useCallback } from 'react';
import { Filter, X, Calendar, FileType, Code, Folder } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { cn } from '../../lib/utils';

export interface DateRange {
  start?: string;
  end?: string;
}

export interface SearchFilters {
  languages: string[];
  fileTypes: string[];
  paths: string[];
  dateRange: DateRange;
  relevanceThreshold: number;
  maxResults: number;
}

export interface FilterPanelProps {
  filters: SearchFilters;
  onChange: (filters: SearchFilters) => void;
  onReset: () => void;
  className?: string;
  collapsed?: boolean;
  onToggleCollapse?: () => void;
}

const COMMON_LANGUAGES = [
  'TypeScript',
  'JavaScript',
  'Python',
  'Rust',
  'Go',
  'Java',
  'C++',
  'C#',
  'Ruby',
  'PHP',
  'Swift',
  'Kotlin',
  'Scala',
  'Shell',
  'SQL',
  'HTML',
  'CSS',
  'YAML',
  'JSON',
  'Markdown',
];

const COMMON_FILE_TYPES = [
  '.ts',
  '.tsx',
  '.js',
  '.jsx',
  '.py',
  '.rs',
  '.go',
  '.java',
  '.cpp',
  '.c',
  '.cs',
  '.rb',
  '.php',
  '.swift',
  '.kt',
  '.scala',
  '.sh',
  '.sql',
  '.html',
  '.css',
  '.yml',
  '.yaml',
  '.json',
  '.md',
  '.toml',
  '.xml',
];

export const FilterPanel: React.FC<FilterPanelProps> = ({
  filters,
  onChange,
  onReset,
  className,
  collapsed = false,
  onToggleCollapse,
}) => {
  const [customLanguage, setCustomLanguage] = useState('');
  const [customFileType, setCustomFileType] = useState('');
  const [customPath, setCustomPath] = useState('');

  // Handle language filter changes
  const handleLanguageToggle = useCallback((language: string) => {
    const newLanguages = filters.languages.includes(language)
      ? filters.languages.filter(l => l !== language)
      : [...filters.languages, language];
    
    onChange({ ...filters, languages: newLanguages });
  }, [filters, onChange]);

  // Handle file type filter changes
  const handleFileTypeToggle = useCallback((fileType: string) => {
    const newFileTypes = filters.fileTypes.includes(fileType)
      ? filters.fileTypes.filter(ft => ft !== fileType)
      : [...filters.fileTypes, fileType];
    
    onChange({ ...filters, fileTypes: newFileTypes });
  }, [filters, onChange]);

  // Handle path filter changes
  const handlePathAdd = useCallback((path: string) => {
    if (path.trim() && !filters.paths.includes(path.trim())) {
      onChange({ 
        ...filters, 
        paths: [...filters.paths, path.trim()]
      });
    }
  }, [filters, onChange]);

  const handlePathRemove = useCallback((path: string) => {
    onChange({ 
      ...filters, 
      paths: filters.paths.filter(p => p !== path)
    });
  }, [filters, onChange]);

  // Handle date range changes
  const handleDateRangeChange = useCallback((field: keyof DateRange, value: string) => {
    onChange({
      ...filters,
      dateRange: { ...filters.dateRange, [field]: value || undefined }
    });
  }, [filters, onChange]);

  // Handle relevance threshold change
  const handleRelevanceThresholdChange = useCallback((value: number) => {
    onChange({ ...filters, relevanceThreshold: value });
  }, [filters, onChange]);

  // Handle max results change
  const handleMaxResultsChange = useCallback((value: number) => {
    onChange({ ...filters, maxResults: value });
  }, [filters, onChange]);

  // Add custom language
  const handleAddCustomLanguage = () => {
    if (customLanguage.trim() && !filters.languages.includes(customLanguage.trim())) {
      handleLanguageToggle(customLanguage.trim());
      setCustomLanguage('');
    }
  };

  // Add custom file type
  const handleAddCustomFileType = () => {
    if (customFileType.trim() && !filters.fileTypes.includes(customFileType.trim())) {
      handleFileTypeToggle(customFileType.trim());
      setCustomFileType('');
    }
  };

  // Add custom path
  const handleAddCustomPath = () => {
    if (customPath.trim()) {
      handlePathAdd(customPath.trim());
      setCustomPath('');
    }
  };

  // Check if any filters are active
  const hasActiveFilters = 
    filters.languages.length > 0 || 
    filters.fileTypes.length > 0 || 
    filters.paths.length > 0 ||
    filters.dateRange.start || 
    filters.dateRange.end ||
    filters.relevanceThreshold > 0 ||
    filters.maxResults !== 100;

  if (collapsed) {
    return (
      <div className={cn("flex items-center gap-2", className)}>
        <Button
          variant="outline"
          size="sm"
          onClick={onToggleCollapse}
          className="flex items-center gap-2"
        >
          <Filter className="h-4 w-4" />
          Filters
          {hasActiveFilters && (
            <span className="rounded-full bg-primary px-1.5 py-0.5 text-xs text-primary-foreground">
              {filters.languages.length + filters.fileTypes.length + filters.paths.length + 
               (filters.dateRange.start || filters.dateRange.end ? 1 : 0)}
            </span>
          )}
        </Button>
        {hasActiveFilters && (
          <Button variant="ghost" size="sm" onClick={onReset}>
            Clear
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className={cn("space-y-6 rounded-lg border bg-card p-4", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4" />
          <h3 className="font-medium">Filters</h3>
        </div>
        <div className="flex items-center gap-2">
          {hasActiveFilters && (
            <Button variant="ghost" size="sm" onClick={onReset}>
              Reset
            </Button>
          )}
          {onToggleCollapse && (
            <Button variant="ghost" size="sm" onClick={onToggleCollapse}>
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Languages */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Code className="h-4 w-4 text-muted-foreground" />
          <h4 className="text-sm font-medium">Languages</h4>
        </div>
        <div className="flex flex-wrap gap-2">
          {COMMON_LANGUAGES.map(language => (
            <Button
              key={language}
              variant={filters.languages.includes(language) ? "default" : "outline"}
              size="sm"
              className="h-7 text-xs"
              onClick={() => handleLanguageToggle(language)}
            >
              {language}
            </Button>
          ))}
        </div>
        {filters.languages.filter(lang => !COMMON_LANGUAGES.includes(lang)).map(language => (
          <Button
            key={language}
            variant="default"
            size="sm"
            className="h-7 text-xs"
            onClick={() => handleLanguageToggle(language)}
          >
            {language}
            <X className="ml-1 h-3 w-3" />
          </Button>
        ))}
        <div className="flex gap-2">
          <Input
            placeholder="Add custom language..."
            value={customLanguage}
            onChange={(e) => setCustomLanguage(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleAddCustomLanguage()}
            className="h-8 text-xs"
          />
          <Button size="sm" onClick={handleAddCustomLanguage} disabled={!customLanguage.trim()}>
            Add
          </Button>
        </div>
      </div>

      {/* File Types */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <FileType className="h-4 w-4 text-muted-foreground" />
          <h4 className="text-sm font-medium">File Types</h4>
        </div>
        <div className="flex flex-wrap gap-2">
          {COMMON_FILE_TYPES.map(fileType => (
            <Button
              key={fileType}
              variant={filters.fileTypes.includes(fileType) ? "default" : "outline"}
              size="sm"
              className="h-7 text-xs"
              onClick={() => handleFileTypeToggle(fileType)}
            >
              {fileType}
            </Button>
          ))}
        </div>
        {filters.fileTypes.filter(ft => !COMMON_FILE_TYPES.includes(ft)).map(fileType => (
          <Button
            key={fileType}
            variant="default"
            size="sm"
            className="h-7 text-xs"
            onClick={() => handleFileTypeToggle(fileType)}
          >
            {fileType}
            <X className="ml-1 h-3 w-3" />
          </Button>
        ))}
        <div className="flex gap-2">
          <Input
            placeholder="Add file extension..."
            value={customFileType}
            onChange={(e) => setCustomFileType(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleAddCustomFileType()}
            className="h-8 text-xs"
          />
          <Button size="sm" onClick={handleAddCustomFileType} disabled={!customFileType.trim()}>
            Add
          </Button>
        </div>
      </div>

      {/* Paths */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Folder className="h-4 w-4 text-muted-foreground" />
          <h4 className="text-sm font-medium">Paths</h4>
        </div>
        {filters.paths.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {filters.paths.map(path => (
              <Button
                key={path}
                variant="default"
                size="sm"
                className="h-7 text-xs"
                onClick={() => handlePathRemove(path)}
              >
                {path}
                <X className="ml-1 h-3 w-3" />
              </Button>
            ))}
          </div>
        )}
        <div className="flex gap-2">
          <Input
            placeholder="Add path filter... (e.g., src/, packages/)"
            value={customPath}
            onChange={(e) => setCustomPath(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleAddCustomPath()}
            className="h-8 text-xs"
          />
          <Button size="sm" onClick={handleAddCustomPath} disabled={!customPath.trim()}>
            Add
          </Button>
        </div>
      </div>

      {/* Date Range */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Calendar className="h-4 w-4 text-muted-foreground" />
          <h4 className="text-sm font-medium">Date Range</h4>
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className="text-xs text-muted-foreground">From</label>
            <Input
              type="date"
              value={filters.dateRange.start || ''}
              onChange={(e) => handleDateRangeChange('start', e.target.value)}
              className="h-8 text-xs"
            />
          </div>
          <div>
            <label className="text-xs text-muted-foreground">To</label>
            <Input
              type="date"
              value={filters.dateRange.end || ''}
              onChange={(e) => handleDateRangeChange('end', e.target.value)}
              className="h-8 text-xs"
            />
          </div>
        </div>
      </div>

      {/* Relevance Threshold */}
      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium">Relevance Threshold</label>
          <p className="text-xs text-muted-foreground">
            Minimum relevance score (0.0 - 1.0)
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Input
            type="range"
            min="0"
            max="1"
            step="0.1"
            value={filters.relevanceThreshold}
            onChange={(e) => handleRelevanceThresholdChange(parseFloat(e.target.value))}
            className="flex-1"
          />
          <span className="text-xs font-mono w-10 text-right">
            {filters.relevanceThreshold.toFixed(1)}
          </span>
        </div>
      </div>

      {/* Max Results */}
      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium">Max Results</label>
          <p className="text-xs text-muted-foreground">
            Maximum number of results to display
          </p>
        </div>
        <div className="flex gap-2">
          {[25, 50, 100, 200, 500].map(num => (
            <Button
              key={num}
              variant={filters.maxResults === num ? "default" : "outline"}
              size="sm"
              className="h-7 text-xs"
              onClick={() => handleMaxResultsChange(num)}
            >
              {num}
            </Button>
          ))}
        </div>
      </div>
    </div>
  );
};