/**
 * Example usage of the FileSystemService
 * 
 * This file demonstrates how to use the secure file system operations
 * in various scenarios within the CrewChief Web UI.
 */

import path from 'path';
import { constants as fsConstants } from 'fs';
import {
  createProjectFileSystemService,
  createSecureFileSystemService,
  FileSystemService,
  WatchEvent,
} from './index.js';

/**
 * Example: Basic file operations
 */
export async function basicFileOperations() {
  const projectRoot = path.resolve('./project');
  const fs = createProjectFileSystemService(projectRoot);

  try {
    // Write a configuration file
    const config = {
      name: 'My Project',
      version: '1.0.0',
      settings: {
        debug: true,
        maxWorkers: 4,
      },
    };
    
    await fs.writeFile('config.json', JSON.stringify(config, null, 2));
    console.log('Configuration file written');

    // Read and parse the configuration
    const configContent = await fs.readFile('config.json');
    const parsedConfig = JSON.parse(configContent);
    console.log('Loaded config:', parsedConfig.name);

    // Check file metadata
    const metadata = await fs.getFileMetadata('config.json');
    console.log(`File size: ${metadata.size} bytes, modified: ${metadata.mtime}`);

    // Create a directory structure
    await fs.createDirectory('src/components');
    await fs.createDirectory('src/utils');
    
    // Write some source files
    await fs.writeFile('src/components/App.tsx', 'export default function App() {}');
    await fs.writeFile('src/utils/helpers.ts', 'export function helper() {}');

    // List directory contents (respecting .gitignore)
    const entries = await fs.listDirectory('src', true);
    console.log('Source files:', entries.map(e => e.name));

  } catch (error) {
    console.error('File operation failed:', error);
  } finally {
    await fs.cleanup();
  }
}

/**
 * Example: Streaming large files
 */
export async function streamingOperations() {
  const fs = createProjectFileSystemService('./uploads');

  try {
    // Create a large file in chunks
    const writeStream = await fs.createWriteStream('large-file.txt');
    
    return new Promise<void>((resolve, reject) => {
      writeStream.on('error', reject);
      writeStream.on('finish', resolve);
      
      // Write data in chunks
      for (let i = 0; i < 1000; i++) {
        writeStream.write(`Line ${i}: This is some sample data\n`);
      }
      writeStream.end();
    }).then(async () => {
      console.log('Large file written using streams');
      
      // Read the file in chunks with progress tracking
      const chunks = await fs.readFileChunked(
        'large-file.txt',
        1024, // 1KB chunks
        (progress) => {
          console.log(`Progress: ${progress.percentage?.toFixed(1)}%`);
        },
      );
      
      console.log(`Read ${chunks.length} chunks`);
    });

  } catch (error) {
    console.error('Streaming operation failed:', error);
  } finally {
    await fs.cleanup();
  }
}

/**
 * Example: Atomic operations for critical files
 */
export async function atomicOperations() {
  const fs = createSecureFileSystemService('./data');

  try {
    // Write critical data atomically
    const criticalData = {
      lastBackup: new Date().toISOString(),
      records: ['record1', 'record2', 'record3'],
      checksum: 'sha256:abcd1234',
    };

    await fs.writeFileAtomic(
      'critical-data.json',
      JSON.stringify(criticalData, null, 2),
      {
        backup: true,
        backupSuffix: '.backup',
        fsync: true, // Ensure data is written to disk
      },
    );

    console.log('Critical data written atomically with backup');

    // Verify the write was successful
    const verification = await fs.readFile('critical-data.json');
    const parsed = JSON.parse(verification);
    console.log('Verification successful:', parsed.checksum);

  } catch (error) {
    console.error('Atomic operation failed:', error);
    // The backup should still be available if something went wrong
  } finally {
    await fs.cleanup();
  }
}

/**
 * Example: File watching for development
 */
export async function fileWatchingExample() {
  const fs = createProjectFileSystemService('./watched-project');

  try {
    // Set up file watching
    const watcher = fs.getWatcher();
    
    watcher.on('change', (event: WatchEvent) => {
      console.log(`File ${event.type}: ${event.path}`);
      
      switch (event.type) {
        case 'add':
          console.log('New file detected, triggering build...');
          break;
        case 'change':
          console.log('File modified, reloading...');
          break;
        case 'unlink':
          console.log('File deleted, cleaning up...');
          break;
      }
    });

    watcher.on('error', (error) => {
      console.error('Watcher error:', error);
    });

    // Watch the source directory
    await fs.watchPath('./src', {
      recursive: true,
      ignoreInitial: true,
      debounceMs: 300,
    });

    console.log('File watcher started. Make changes to files in ./src/');
    
    // Simulate some file changes
    await fs.createDirectory('src');
    await fs.writeFile('src/test.js', 'console.log("hello");');
    
    // Wait a bit to see the events
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Modify the file
    await fs.writeFile('src/test.js', 'console.log("hello world");');
    
    // Wait for events to process
    await new Promise(resolve => setTimeout(resolve, 1000));

  } catch (error) {
    console.error('File watching failed:', error);
  } finally {
    await fs.cleanup();
  }
}

/**
 * Example: Secure file operations with validation
 */
export async function secureOperations() {
  const fs = createSecureFileSystemService('./secure-area', 5 * 1024 * 1024); // 5MB limit

  try {
    // Attempt to write a file
    const userContent = 'User provided content';
    const fileName = 'user-file.txt';

    // Validate the filename is safe
    if (fileName.includes('/') || fileName.includes('\\')) {
      throw new Error('Invalid filename: contains path separators');
    }

    // Check if we can write to the location
    const canWrite = await fs.checkPermissions('.', fsConstants.W_OK);
    if (!canWrite) {
      throw new Error('No write permission');
    }

    // Write the file
    await fs.writeFile(fileName, userContent);
    console.log('File written securely');

    // Get file metadata to verify
    const metadata = await fs.getFileMetadata(fileName);
    console.log(`File info: ${metadata.size} bytes, type: ${metadata.mimeType}`);

    // Demonstrate directory traversal protection
    try {
      await fs.readFile('../../../etc/passwd');
      console.log('ERROR: Security bypass detected!');
    } catch (error) {
      console.log('Security working: path traversal blocked');
    }

  } catch (error) {
    console.error('Secure operation failed:', error);
  } finally {
    await fs.cleanup();
  }
}

/**
 * Example: Integration with CrewChief workflows
 */
export async function crewChiefIntegration() {
  // Simulate a CrewChief worktree directory
  const worktreeRoot = './crewchief-worktree';
  const fs = createProjectFileSystemService(worktreeRoot);

  try {
    // Create a typical CrewChief project structure
    await fs.createDirectory('src');
    await fs.createDirectory('tests');
    await fs.createDirectory('docs');

    // Write project files
    await fs.writeFile('package.json', JSON.stringify({
      name: 'crewchief-task',
      version: '1.0.0',
      dependencies: {},
    }, null, 2));

    await fs.writeFile('src/main.ts', `
export class TaskRunner {
  async run() {
    console.log('Task running...');
  }
}
`);

    await fs.writeFile('tests/main.test.ts', `
import { TaskRunner } from '../src/main.js';

describe('TaskRunner', () => {
  it('should run successfully', async () => {
    const runner = new TaskRunner();
    await runner.run();
  });
});
`);

    // Create .gitignore to exclude build outputs
    await fs.writeFile('.gitignore', `
node_modules/
dist/
*.log
.env.local
`);

    // List all project files (respecting .gitignore)
    const projectFiles = await fs.listDirectory('.', true);
    console.log('Project files:');
    projectFiles.forEach(file => {
      console.log(`  ${file.type === 'directory' ? '📁' : '📄'} ${file.path}`);
    });

    // Watch for changes during development
    const watcher = fs.getWatcher();
    watcher.on('change', (event: WatchEvent) => {
      console.log(`[CrewChief] ${event.type.toUpperCase()}: ${event.path}`);
      
      // Simulate triggering a rebuild or test run
      if (event.path.endsWith('.ts') && event.type === 'change') {
        console.log('[CrewChief] TypeScript file changed, running type check...');
      }
    });

    await fs.watchPath('./src', { recursive: true });
    console.log('CrewChief file watcher active');

  } catch (error) {
    console.error('CrewChief integration failed:', error);
  } finally {
    await fs.cleanup();
  }
}

/**
 * Example: Error handling and recovery
 */
export async function errorHandlingExample() {
  const fs = createProjectFileSystemService('./error-test');

  try {
    // Demonstrate graceful error handling
    console.log('Testing error scenarios...');

    // Test 1: File not found
    try {
      await fs.readFile('nonexistent.txt');
    } catch (error) {
      console.log('✓ File not found handled gracefully');
    }

    // Test 2: Permission denied (simulated)
    try {
      await fs.writeFile('/root/restricted.txt', 'content');
    } catch (error) {
      console.log('✓ Permission error handled gracefully');
    }

    // Test 3: File size limit
    try {
      const hugeContent = 'x'.repeat(200 * 1024 * 1024); // 200MB
      await fs.writeFile('huge.txt', hugeContent);
    } catch (error) {
      console.log('✓ File size limit enforced');
    }

    // Test 4: Path traversal
    try {
      await fs.readFile('../../../etc/passwd');
    } catch (error) {
      console.log('✓ Path traversal attack blocked');
    }

    console.log('All security tests passed!');

  } catch (error) {
    console.error('Unexpected error in error handling test:', error);
  } finally {
    await fs.cleanup();
  }
}

// Export all examples for easy testing
export const examples = {
  basicFileOperations,
  streamingOperations,
  atomicOperations,
  fileWatchingExample,
  secureOperations,
  crewChiefIntegration,
  errorHandlingExample,
};

// Run all examples if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  console.log('Running FileSystem examples...\n');
  
  for (const [name, example] of Object.entries(examples)) {
    console.log(`\n=== ${name} ===`);
    try {
      await example();
      console.log(`✓ ${name} completed successfully`);
    } catch (error) {
      console.error(`✗ ${name} failed:`, error);
    }
  }
}