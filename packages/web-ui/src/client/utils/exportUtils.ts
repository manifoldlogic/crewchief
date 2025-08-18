import { SearchResult } from '../components/search/SearchResults';

export interface ExportOptions {
  includeContext?: boolean;
  includeMetadata?: boolean;
  filename?: string;
  timestamp?: boolean;
}

/**
 * Export search results to JSON format
 */
export const exportToJSON = async (
  results: SearchResult[], 
  options: ExportOptions = {}
): Promise<void> => {
  const {
    includeContext = true,
    includeMetadata = true,
    filename = 'search-results',
    timestamp = true,
  } = options;

  try {
    const exportData = {
      ...(includeMetadata && {
        metadata: {
          exportDate: new Date().toISOString(),
          totalResults: results.length,
          exportOptions: options,
        },
      }),
      results: results.map(result => ({
        id: result.id,
        file_path: result.file_path,
        line_start: result.line_start,
        line_end: result.line_end,
        content: result.content,
        relevance_score: result.relevance_score,
        ...(result.language && { language: result.language }),
        ...(result.chunk_type && { chunk_type: result.chunk_type }),
        ...(includeContext && result.context && { context: result.context }),
      })),
    };

    const jsonString = JSON.stringify(exportData, null, 2);
    const blob = new Blob([jsonString], { type: 'application/json' });
    
    const finalFilename = timestamp 
      ? `${filename}-${new Date().toISOString().split('T')[0]}.json`
      : `${filename}.json`;
      
    await downloadBlob(blob, finalFilename);
  } catch (error) {
    console.error('Failed to export to JSON:', error);
    throw new Error('Export to JSON failed');
  }
};

/**
 * Export search results to CSV format
 */
export const exportToCSV = async (
  results: SearchResult[], 
  options: ExportOptions = {}
): Promise<void> => {
  const {
    includeContext = false, // CSV typically doesn't handle multiline well
    filename = 'search-results',
    timestamp = true,
  } = options;

  try {
    const headers = [
      'File Path',
      'Line Start',
      'Line End',
      'Relevance Score',
      'Language',
      'Chunk Type',
      'Content',
      ...(includeContext ? ['Context Before', 'Context After'] : []),
    ];

    const csvRows = [
      headers.join(','),
      ...results.map(result => {
        const row = [
          escapeCSVField(result.file_path),
          result.line_start.toString(),
          result.line_end.toString(),
          result.relevance_score.toFixed(3),
          escapeCSVField(result.language || ''),
          escapeCSVField(result.chunk_type || ''),
          escapeCSVField(result.content.replace(/\n/g, ' ')), // Replace newlines with spaces
          ...(includeContext ? [
            escapeCSVField(result.context?.before?.replace(/\n/g, ' ') || ''),
            escapeCSVField(result.context?.after?.replace(/\n/g, ' ') || ''),
          ] : []),
        ];
        return row.join(',');
      }),
    ];

    const csvString = csvRows.join('\n');
    const blob = new Blob([csvString], { type: 'text/csv;charset=utf-8;' });
    
    const finalFilename = timestamp 
      ? `${filename}-${new Date().toISOString().split('T')[0]}.csv`
      : `${filename}.csv`;
      
    await downloadBlob(blob, finalFilename);
  } catch (error) {
    console.error('Failed to export to CSV:', error);
    throw new Error('Export to CSV failed');
  }
};

/**
 * Copy search results to clipboard
 */
export const copyToClipboard = async (
  results: SearchResult[], 
  format: 'text' | 'json' | 'markdown' = 'text'
): Promise<void> => {
  try {
    let content: string;

    switch (format) {
      case 'json':
        content = JSON.stringify(results, null, 2);
        break;
        
      case 'markdown':
        content = generateMarkdownContent(results);
        break;
        
      case 'text':
      default:
        content = generateTextContent(results);
        break;
    }

    await navigator.clipboard.writeText(content);
  } catch (error) {
    console.error('Failed to copy to clipboard:', error);
    throw new Error('Copy to clipboard failed');
  }
};

/**
 * Generate markdown formatted content
 */
const generateMarkdownContent = (results: SearchResult[]): string => {
  const sections = results.map((result, index) => {
    const header = `## Result ${index + 1}: ${result.file_path}`;
    const metadata = [
      `- **Lines:** ${result.line_start}-${result.line_end}`,
      `- **Relevance:** ${(result.relevance_score * 100).toFixed(1)}%`,
      ...(result.language ? [`- **Language:** ${result.language}`] : []),
      ...(result.chunk_type ? [`- **Type:** ${result.chunk_type}`] : []),
    ].join('\n');
    
    const codeBlock = `\`\`\`${result.language?.toLowerCase() || ''}\n${result.content}\n\`\`\``;
    
    return [header, metadata, codeBlock].join('\n\n');
  });

  const header = `# Search Results (${results.length} items)\n\nGenerated on ${new Date().toLocaleString()}\n`;
  
  return [header, ...sections].join('\n\n---\n\n');
};

/**
 * Generate plain text content
 */
const generateTextContent = (results: SearchResult[]): string => {
  const sections = results.map((result, index) => {
    const header = `Result ${index + 1}: ${result.file_path}`;
    const separator = '='.repeat(header.length);
    const metadata = [
      `Lines: ${result.line_start}-${result.line_end}`,
      `Relevance: ${(result.relevance_score * 100).toFixed(1)}%`,
      ...(result.language ? [`Language: ${result.language}`] : []),
      ...(result.chunk_type ? [`Type: ${result.chunk_type}`] : []),
    ].join(' | ');
    
    return [header, separator, metadata, '', result.content].join('\n');
  });

  const header = `Search Results (${results.length} items)\nGenerated on ${new Date().toLocaleString()}\n`;
  const headerSeparator = '='.repeat(50);
  
  return [header, headerSeparator, '', ...sections].join('\n\n');
};

/**
 * Escape CSV field content
 */
const escapeCSVField = (field: string): string => {
  if (field.includes(',') || field.includes('"') || field.includes('\n')) {
    return `"${field.replace(/"/g, '""')}"`;
  }
  return field;
};

/**
 * Download blob as file
 */
const downloadBlob = async (blob: Blob, filename: string): Promise<void> => {
  // Create a temporary URL for the blob
  const url = URL.createObjectURL(blob);
  
  try {
    // Create a temporary link element and trigger download
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    
    // Append to body, click, and remove
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  } finally {
    // Clean up the URL
    URL.revokeObjectURL(url);
  }
};

/**
 * Get file size in human readable format
 */
export const getFileSizeString = (bytes: number): string => {
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  if (bytes === 0) return '0 Bytes';
  
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
};

/**
 * Validate export data before processing
 */
export const validateExportData = (results: SearchResult[]): boolean => {
  if (!Array.isArray(results)) {
    throw new Error('Results must be an array');
  }
  
  if (results.length === 0) {
    throw new Error('No results to export');
  }
  
  // Validate each result has required fields
  for (const result of results) {
    if (!result.id || !result.file_path || !result.content) {
      throw new Error('Invalid result data: missing required fields');
    }
  }
  
  return true;
};