import React, { useState, useMemo } from 'react';
import { Highlight, themes } from 'prism-react-renderer';
import { 
  Copy, 
  ExternalLink, 
  ChevronDown, 
  ChevronUp, 
  FileText,
  Maximize2,
  X
} from 'lucide-react';
import { Button } from '../ui/button';
import { cn } from '../../lib/utils';

export interface CodePreviewProps {
  content: string;
  filePath: string;
  language?: string;
  lineStart?: number;
  lineEnd?: number;
  highlightLines?: number[];
  searchTerms?: string[];
  onOpenFile?: (filePath: string, lineNumber?: number) => void;
  className?: string;
  expanded?: boolean;
  onToggleExpanded?: () => void;
  showLineNumbers?: boolean;
  maxHeight?: number;
  title?: string;
}

// Language mapping for Prism
const getLanguageFromExtension = (filePath: string, language?: string): string => {
  if (language) {
    // Map common language names to Prism language identifiers
    const languageMap: Record<string, string> = {
      'TypeScript': 'typescript',
      'JavaScript': 'javascript',
      'Python': 'python',
      'Rust': 'rust',
      'Go': 'go',
      'Java': 'java',
      'C++': 'cpp',
      'C#': 'csharp',
      'Ruby': 'ruby',
      'PHP': 'php',
      'Swift': 'swift',
      'Kotlin': 'kotlin',
      'Scala': 'scala',
      'Shell': 'bash',
      'SQL': 'sql',
      'HTML': 'html',
      'CSS': 'css',
      'YAML': 'yaml',
      'JSON': 'json',
      'Markdown': 'markdown',
    };
    
    return languageMap[language] || language.toLowerCase();
  }

  // Fallback to file extension
  const extension = filePath.split('.').pop()?.toLowerCase();
  const extensionMap: Record<string, string> = {
    'ts': 'typescript',
    'tsx': 'tsx',
    'js': 'javascript',
    'jsx': 'jsx',
    'py': 'python',
    'rs': 'rust',
    'go': 'go',
    'java': 'java',
    'cpp': 'cpp',
    'c': 'c',
    'cs': 'csharp',
    'rb': 'ruby',
    'php': 'php',
    'swift': 'swift',
    'kt': 'kotlin',
    'scala': 'scala',
    'sh': 'bash',
    'bash': 'bash',
    'sql': 'sql',
    'html': 'html',
    'css': 'css',
    'scss': 'scss',
    'yml': 'yaml',
    'yaml': 'yaml',
    'json': 'json',
    'md': 'markdown',
    'toml': 'toml',
    'xml': 'xml',
  };

  return extensionMap[extension || ''] || 'text';
};

// Highlight search terms in code
const highlightSearchTerms = (content: string, searchTerms: string[] = []): string => {
  if (!searchTerms.length) return content;

  let highlighted = content;
  searchTerms.forEach(term => {
    if (term.trim()) {
      const regex = new RegExp(`(${term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
      highlighted = highlighted.replace(regex, '===$1===');
    }
  });

  return highlighted;
};

export const CodePreview: React.FC<CodePreviewProps> = ({
  content,
  filePath,
  language,
  lineStart = 1,
  lineEnd,
  highlightLines = [],
  searchTerms = [],
  onOpenFile,
  className,
  expanded = false,
  onToggleExpanded,
  showLineNumbers = true,
  maxHeight = 400,
  title,
}) => {
  const [isFullscreen, setIsFullscreen] = useState(false);
  
  const prismLanguage = useMemo(
    () => getLanguageFromExtension(filePath, language),
    [filePath, language]
  );

  const lines = useMemo(() => content.split('\n'), [content]);
  const actualLineEnd = lineEnd || lineStart + lines.length - 1;

  const handleCopyContent = async () => {
    try {
      await navigator.clipboard.writeText(content);
    } catch (error) {
      console.warn('Failed to copy to clipboard:', error);
    }
  };

  const handleOpenFile = () => {
    onOpenFile?.(filePath, lineStart);
  };

  const handleToggleFullscreen = () => {
    setIsFullscreen(!isFullscreen);
  };

  const containerClassName = cn(
    "rounded-lg border bg-card overflow-hidden",
    isFullscreen && "fixed inset-4 z-50 shadow-2xl",
    className
  );

  const codeContainerStyle = isFullscreen 
    ? { maxHeight: 'calc(100vh - 120px)' }
    : { maxHeight: expanded ? 'none' : maxHeight };

  return (
    <>
      {isFullscreen && (
        <div className="fixed inset-0 bg-black/50 z-40" onClick={handleToggleFullscreen} />
      )}
      
      <div className={containerClassName}>
        {/* Header */}
        <div className="flex items-center justify-between p-3 border-b bg-muted/50">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <FileText className="h-4 w-4 text-muted-foreground flex-shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium truncate" title={filePath}>
                {title || filePath}
              </p>
              <p className="text-xs text-muted-foreground">
                {lineStart !== actualLineEnd ? (
                  `Lines ${lineStart}-${actualLineEnd}`
                ) : (
                  `Line ${lineStart}`
                )}
                {language && (
                  <>
                    {' • '}
                    <span className="font-mono">{language}</span>
                  </>
                )}
              </p>
            </div>
          </div>
          
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyContent}
              title="Copy code"
            >
              <Copy className="h-4 w-4" />
            </Button>
            
            {onOpenFile && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleOpenFile}
                title="Open file"
              >
                <ExternalLink className="h-4 w-4" />
              </Button>
            )}
            
            <Button
              variant="ghost"
              size="sm"
              onClick={handleToggleFullscreen}
              title="Toggle fullscreen"
            >
              <Maximize2 className="h-4 w-4" />
            </Button>
            
            {onToggleExpanded && !isFullscreen && (
              <Button
                variant="ghost"
                size="sm"
                onClick={onToggleExpanded}
                title={expanded ? "Collapse" : "Expand"}
              >
                {expanded ? (
                  <ChevronUp className="h-4 w-4" />
                ) : (
                  <ChevronDown className="h-4 w-4" />
                )}
              </Button>
            )}
            
            {isFullscreen && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleToggleFullscreen}
                title="Close fullscreen"
              >
                <X className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>

        {/* Code Content */}
        <div 
          className="overflow-auto scrollbar-thin scrollbar-thumb-muted-foreground/20"
          style={codeContainerStyle}
        >
          <Highlight
            theme={themes.github}
            code={content}
            language={prismLanguage}
          >
            {({ className, style, tokens, getLineProps, getTokenProps }) => (
              <pre 
                className={cn(className, "text-sm p-4 m-0")} 
                style={style}
              >
                {tokens.map((line, lineIndex) => {
                  const lineNumber = lineStart + lineIndex;
                  const isHighlighted = highlightLines.includes(lineNumber);
                  
                  return (
                    <div
                      key={lineIndex}
                      className={cn(
                        "flex",
                        isHighlighted && "bg-yellow-100 dark:bg-yellow-900/20"
                      )}
                      {...getLineProps({ line, key: lineIndex })}
                    >
                      {showLineNumbers && (
                        <span className="select-none text-muted-foreground text-right pr-4 w-12 flex-shrink-0">
                          {lineNumber}
                        </span>
                      )}
                      <span className="flex-1">
                        {line.map((token, key) => {
                          const tokenValue = token.content;
                          const highlighted = highlightSearchTerms(tokenValue, searchTerms);
                          
                          if (highlighted !== tokenValue && highlighted.includes('===')) {
                            // Split by highlight markers and render highlighted parts
                            const parts = highlighted.split(/===(.*?)===/g);
                            return (
                              <span key={key} {...getTokenProps({ token: { ...token, content: '' }, key })}>
                                {parts.map((part, partIndex) => 
                                  partIndex % 2 === 1 ? (
                                    <mark key={partIndex} className="bg-yellow-200 dark:bg-yellow-800">
                                      {part}
                                    </mark>
                                  ) : (
                                    part
                                  )
                                )}
                              </span>
                            );
                          }
                          
                          return <span key={key} {...getTokenProps({ token, key })} />;
                        })}
                      </span>
                    </div>
                  );
                })}
              </pre>
            )}
          </Highlight>
        </div>

        {/* Footer (only show when not expanded and content is long) */}
        {!expanded && !isFullscreen && lines.length > 15 && onToggleExpanded && (
          <div className="p-2 border-t bg-muted/50 text-center">
            <Button variant="ghost" size="sm" onClick={onToggleExpanded}>
              <ChevronDown className="h-4 w-4 mr-1" />
              Show more ({lines.length - 15} more lines)
            </Button>
          </div>
        )}
      </div>
    </>
  );
};