# FileSystem Service

A comprehensive, secure file system service for the CrewChief Web UI with built-in security features, streaming support, and file watching capabilities.

## Features

### Security Features
- **Path Traversal Prevention**: Prevents access outside the configured root directory
- **File Size Limits**: Configurable maximum file sizes to prevent resource exhaustion
- **Symbolic Link Protection**: Secure handling of symbolic links with optional following
- **Path Sanitization**: Removes dangerous characters and validates file paths
- **Permission Validation**: Checks file system permissions before operations
- **Concurrency Limiting**: Prevents resource exhaustion with configurable operation limits

### File Operations
- **Reading**: Text files, binary files, streaming reads, chunked reads with progress
- **Writing**: Text files, binary files, streaming writes, atomic writes with rollback
- **Directory Operations**: Create, list (with .gitignore support), remove, recursive operations
- **File Metadata**: Size, timestamps, permissions, MIME types, file stats

### Advanced Features
- **File Watching**: Real-time file system monitoring with debouncing
- **GitIgnore Support**: Respects .gitignore files when listing directories
- **Atomic Operations**: Write-to-temp-then-rename for data integrity
- **Streaming**: Efficient handling of large files without memory issues
- **Progress Tracking**: Progress callbacks for long-running operations

## Quick Start

```typescript
import { createProjectFileSystemService } from './filesystem/index.js';

// Create a service for your project
const fs = createProjectFileSystemService('./my-project');

// Basic file operations
await fs.writeFile('config.json', JSON.stringify({ setting: 'value' }));
const content = await fs.readFile('config.json');

// Directory operations
await fs.createDirectory('src/components');
const files = await fs.listDirectory('src', true); // respects .gitignore

// Clean up when done
await fs.cleanup();
```

## API Reference

### FileSystemService

The main service class providing all file system operations.

#### Constructor Options

```typescript
interface FileSystemOptions {
  rootDirectory: string;           // Root directory for all operations
  maxFileSize?: number;           // Maximum file size in bytes (default: 100MB)
  followSymlinks?: boolean;       // Whether to follow symbolic links (default: false)
  tempDirectory?: string;         // Directory for temporary files
  watchDebounceMs?: number;       // File watch debounce delay (default: 300ms)
  maxConcurrentOps?: number;      // Max concurrent operations (default: 10)
}
```

#### File Reading

```typescript
// Read text file
const content = await fs.readFile('file.txt', 'utf8');

// Read binary file
const buffer = await fs.readFileBuffer('image.png');

// Stream reading
const stream = await fs.createReadStream('large-file.txt', {
  start: 0,
  end: 1024,
  bufferSize: 64 * 1024
});

// Chunked reading with progress
const chunks = await fs.readFileChunked('file.txt', 1024, (progress) => {
  console.log(`Progress: ${progress.percentage}%`);
});
```

#### File Writing

```typescript
// Write text file
await fs.writeFile('output.txt', 'Hello, World!');

// Write binary file
await fs.writeFileBuffer('data.bin', buffer);

// Stream writing
const writeStream = await fs.createWriteStream('output.txt');
writeStream.write('data');
writeStream.end();

// Atomic write (safe for critical files)
await fs.writeFileAtomic('critical.json', data, {
  backup: true,
  fsync: true
});
```

#### Directory Operations

```typescript
// Create directory
await fs.createDirectory('path/to/dir');

// List directory (respects .gitignore)
const entries = await fs.listDirectory('src', true);

// Remove directory
await fs.removeDirectory('temp', true); // recursive
```

#### File Metadata

```typescript
// Get file info
const metadata = await fs.getFileMetadata('file.txt');
console.log(metadata.size, metadata.mimeType, metadata.mtime);

// Check existence
const exists = await fs.exists('file.txt');

// Check permissions
const canRead = await fs.checkPermissions('file.txt', fs.constants.R_OK);
```

#### File Watching

```typescript
// Start watching
const watcher = fs.getWatcher();
watcher.on('change', (event) => {
  console.log(`${event.type}: ${event.path}`);
});

await fs.watchPath('./src', {
  recursive: true,
  debounceMs: 300,
  ignoreInitial: true
});

// Stop watching
await fs.unwatchPath('./src');
```

### Factory Functions

#### createProjectFileSystemService(projectRoot, maxFileSize?)
Creates a service configured for typical project needs.

```typescript
const fs = createProjectFileSystemService('./my-project', 50 * 1024 * 1024); // 50MB limit
```

#### createSecureFileSystemService(rootDirectory, maxFileSize?)
Creates a service with stricter security settings.

```typescript
const fs = createSecureFileSystemService('./secure-area', 10 * 1024 * 1024); // 10MB limit
```

## Security Considerations

### Path Traversal Prevention
The service automatically validates all paths to prevent directory traversal attacks:

```typescript
// These will throw SecurityError
await fs.readFile('../../../etc/passwd');
await fs.readFile('/etc/passwd');
await fs.readFile('..\\..\\windows\\system32\\config\\sam');
```

### File Size Limits
Configure appropriate file size limits to prevent resource exhaustion:

```typescript
const fs = createProjectFileSystemService('./uploads', 5 * 1024 * 1024); // 5MB limit

// This will throw FileSizeError if file is too large
await fs.writeFile('huge.txt', 'x'.repeat(10 * 1024 * 1024));
```

### Symbolic Link Security
By default, symbolic links are not followed for security:

```typescript
const fs = new FileSystemService({
  rootDirectory: './project',
  followSymlinks: false // default
});

// This will throw SecurityError if target.txt is a symlink
await fs.readFile('target.txt');
```

## Error Handling

The service provides specific error types for different failure scenarios:

```typescript
import { 
  SecurityError, 
  FileSizeError, 
  PermissionError, 
  FileSystemError 
} from './filesystem/index.js';

try {
  await fs.readFile('file.txt');
} catch (error) {
  if (error instanceof SecurityError) {
    console.error('Security violation:', error.message);
  } else if (error instanceof FileSizeError) {
    console.error('File too large:', error.size, 'max:', error.maxSize);
  } else if (error instanceof PermissionError) {
    console.error('Permission denied:', error.path);
  } else if (error instanceof FileSystemError) {
    console.error('File system error:', error.code, error.message);
  }
}
```

## Integration with CrewChief

The filesystem service is designed to integrate seamlessly with CrewChief workflows:

```typescript
// In a CrewChief agent
const fs = createProjectFileSystemService(process.cwd());

// Watch for changes during development
const watcher = fs.getWatcher();
watcher.on('change', (event) => {
  if (event.path.endsWith('.ts') && event.type === 'change') {
    // Trigger TypeScript compilation
    runTypeScriptBuild();
  }
});

await fs.watchPath('./src', { recursive: true });
```

## Performance Considerations

### Streaming for Large Files
Use streaming operations for large files to avoid memory issues:

```typescript
// Bad: loads entire file into memory
const content = await fs.readFile('huge-file.txt');

// Good: streams the file
const stream = await fs.createReadStream('huge-file.txt');
```

### Concurrency Control
The service automatically limits concurrent operations to prevent resource exhaustion:

```typescript
const fs = new FileSystemService({
  rootDirectory: './project',
  maxConcurrentOps: 5 // Only 5 operations at once
});
```

### File Watching Debouncing
File watching includes debouncing to reduce CPU usage:

```typescript
await fs.watchPath('./src', {
  debounceMs: 300 // Wait 300ms before emitting events
});
```

## Best Practices

1. **Always call cleanup()** when done with the service
2. **Use atomic writes** for critical files
3. **Enable .gitignore support** when listing directories
4. **Set appropriate file size limits** for your use case
5. **Use streaming** for large files
6. **Handle errors appropriately** with specific error types
7. **Configure concurrency limits** based on your system resources

## Testing

Run the test suite to verify security and functionality:

```bash
pnpm test src/server/filesystem/filesystem.test.ts
```

The tests cover:
- Security features (path traversal, file size limits, etc.)
- All file operations (read, write, stream, atomic)
- Directory operations
- File watching
- Error handling
- Concurrency control

## Examples

See `example.ts` for comprehensive examples of all features including:
- Basic file operations
- Streaming operations
- Atomic operations
- File watching
- Security validation
- CrewChief integration
- Error handling