import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { promises as fs, constants as fsConstants } from 'fs';
import path from 'path';
import os from 'os';
import {
  FileSystemService,
  createFileSystemService,
  SecurityError,
  FileSizeError,
  FileSystemError,
} from './index.js';

describe('FileSystemService', () => {
  let tempDir: string;
  let service: FileSystemService;

  beforeEach(async () => {
    // Create a temporary directory for testing
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'fs-test-'));
    service = createFileSystemService({
      rootDirectory: tempDir,
      maxFileSize: 1024 * 1024, // 1MB for testing
      maxConcurrentOps: 5,
    });
  });

  afterEach(async () => {
    // Clean up
    await service.cleanup();
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Security', () => {
    it('should prevent path traversal attacks', async () => {
      const maliciousPaths = [
        '../../../etc/passwd',
        '..\\..\\..\\windows\\system32\\config\\sam',
        '/etc/passwd',
        'C:\\Windows\\System32\\config\\SAM',
        '....//....//etc/passwd',
        '..%2F..%2F..%2Fetc%2Fpasswd',
      ];

      for (const maliciousPath of maliciousPaths) {
        await expect(service.readFile(maliciousPath)).rejects.toThrow();
      }
    });

    it('should reject files with null bytes', async () => {
      await expect(service.readFile('test\x00file.txt')).rejects.toThrow();
    });

    it('should prevent access outside root directory', async () => {
      const outsidePath = path.join(path.dirname(tempDir), 'outside.txt');
      await fs.writeFile(outsidePath, 'outside content');
      
      try {
        await expect(service.readFile('../outside.txt')).rejects.toThrow(SecurityError);
      } finally {
        await fs.unlink(outsidePath);
      }
    });

    it('should enforce file size limits', async () => {
      const largePath = path.join(tempDir, 'large.txt');
      const largeContent = 'x'.repeat(2 * 1024 * 1024); // 2MB, exceeds 1MB limit
      
      await expect(service.writeFile('large.txt', largeContent)).rejects.toThrow(FileSizeError);
    });

    it('should handle symbolic links securely', async () => {
      const targetPath = path.join(tempDir, 'target.txt');
      const linkPath = path.join(tempDir, 'link.txt');
      
      await fs.writeFile(targetPath, 'target content');
      await fs.symlink(targetPath, linkPath);
      
      // Should reject symlinks by default
      await expect(service.readFile('link.txt')).rejects.toThrow(SecurityError);
    });

    it('should validate filenames', async () => {
      const dangerousNames = [
        'CON',
        'PRN',
        'AUX',
        'NUL',
        'file<name.txt',
        'file>name.txt',
        'file|name.txt',
        'file?name.txt',
        'file*name.txt',
      ];

      for (const name of dangerousNames) {
        await expect(service.writeFile(name, 'content')).rejects.toThrow(SecurityError);
      }
    });
  });

  describe('File Operations', () => {
    it('should read and write text files', async () => {
      const content = 'Hello, World!';
      await service.writeFile('test.txt', content);
      
      const readContent = await service.readFile('test.txt');
      expect(readContent).toBe(content);
    });

    it('should read and write binary files', async () => {
      const buffer = Buffer.from([1, 2, 3, 4, 5]);
      await service.writeFileBuffer('test.bin', buffer);
      
      const readBuffer = await service.readFileBuffer('test.bin');
      expect(readBuffer).toEqual(buffer);
    });

    it('should handle atomic writes', async () => {
      const originalContent = 'original content';
      const newContent = 'new content';
      
      await service.writeFile('atomic.txt', originalContent);
      
      // Simulate write failure by mocking filesystem
      const originalRename = fs.rename;
      let callCount = 0;
      
      vi.spyOn(fs, 'rename').mockImplementation(async (oldPath, newPath) => {
        callCount++;
        if (callCount === 1) {
          throw new Error('Simulated failure');
        }
        return originalRename(oldPath as any, newPath as any);
      });

      try {
        await expect(service.writeFileAtomic('atomic.txt', newContent)).rejects.toThrow();
        
        // Original content should be preserved
        const content = await service.readFile('atomic.txt');
        expect(content).toBe(originalContent);
      } finally {
        vi.restoreAllMocks();
      }
    });

    it('should handle streaming reads', async () => {
      const content = 'streaming test content';
      await service.writeFile('stream.txt', content);
      
      const stream = await service.createReadStream('stream.txt');
      
      return new Promise<void>((resolve, reject) => {
        let data = '';
        stream.on('data', (chunk) => {
          data += chunk.toString();
        });
        stream.on('end', () => {
          expect(data).toBe(content);
          resolve();
        });
        stream.on('error', reject);
      });
    });

    it('should handle chunked reads with progress', async () => {
      const content = 'x'.repeat(1000);
      await service.writeFile('chunked.txt', content);
      
      const progressEvents: any[] = [];
      const chunks = await service.readFileChunked(
        'chunked.txt',
        100,
        (progress) => progressEvents.push(progress),
      );
      
      const reconstructed = Buffer.concat(chunks).toString();
      expect(reconstructed).toBe(content);
      expect(progressEvents.length).toBeGreaterThan(0);
      expect(progressEvents[progressEvents.length - 1].percentage).toBe(100);
    });
  });

  describe('Directory Operations', () => {
    it('should create and list directories', async () => {
      await service.createDirectory('testdir');
      await service.writeFile('testdir/file1.txt', 'content1');
      await service.writeFile('testdir/file2.txt', 'content2');
      
      const entries = await service.listDirectory('testdir', false);
      expect(entries).toHaveLength(2);
      
      const fileNames = entries.map(e => e.name).sort();
      expect(fileNames).toEqual(['file1.txt', 'file2.txt']);
    });

    it('should respect .gitignore when listing directories', async () => {
      await service.createDirectory('gitdir');
      await service.writeFile('gitdir/.gitignore', 'ignored.txt\n*.tmp');
      await service.writeFile('gitdir/included.txt', 'included');
      await service.writeFile('gitdir/ignored.txt', 'ignored');
      await service.writeFile('gitdir/test.tmp', 'temp');
      
      const entries = await service.listDirectory('gitdir', true);
      const fileNames = entries.map(e => e.name);
      
      expect(fileNames).toContain('included.txt');
      expect(fileNames).not.toContain('ignored.txt');
      expect(fileNames).not.toContain('test.tmp');
    });

    it('should copy files and directories', async () => {
      await service.writeFile('source.txt', 'source content');
      await service.copy('source.txt', 'dest.txt');
      
      const content = await service.readFile('dest.txt');
      expect(content).toBe('source content');
    });

    it('should copy directories recursively', async () => {
      await service.createDirectory('sourcedir');
      await service.writeFile('sourcedir/file.txt', 'file content');
      await service.createDirectory('sourcedir/subdir');
      await service.writeFile('sourcedir/subdir/subfile.txt', 'sub content');
      
      await service.copy('sourcedir', 'destdir', { recursive: true });
      
      expect(await service.exists('destdir/file.txt')).toBe(true);
      expect(await service.exists('destdir/subdir/subfile.txt')).toBe(true);
      
      const content = await service.readFile('destdir/subdir/subfile.txt');
      expect(content).toBe('sub content');
    });
  });

  describe('File Metadata', () => {
    it('should get file metadata', async () => {
      const content = 'metadata test';
      await service.writeFile('metadata.txt', content);
      
      const metadata = await service.getFileMetadata('metadata.txt');
      
      expect(metadata.path).toBe('metadata.txt');
      expect(metadata.size).toBe(content.length);
      expect(metadata.isFile).toBe(true);
      expect(metadata.isDirectory).toBe(false);
      expect(metadata.mimeType).toBe('text/plain');
    });

    it('should check file existence', async () => {
      await service.writeFile('exists.txt', 'content');
      
      expect(await service.exists('exists.txt')).toBe(true);
      expect(await service.exists('nonexistent.txt')).toBe(false);
    });

    it('should check file permissions', async () => {
      await service.writeFile('perms.txt', 'content');
      
      expect(await service.checkPermissions('perms.txt', fsConstants.R_OK)).toBe(true);
      expect(await service.checkPermissions('perms.txt', fsConstants.W_OK)).toBe(true);
      expect(await service.checkPermissions('nonexistent.txt')).toBe(false);
    });
  });

  describe('File Watching', () => {
    it('should watch file changes', async () => {
      const events: any[] = [];
      
      service.getWatcher().on('change', (event) => {
        events.push(event);
      });
      
      await service.watchPath(tempDir);
      
      // Wait a bit for watcher to initialize
      await new Promise(resolve => setTimeout(resolve, 100));
      
      await service.writeFile('watched.txt', 'initial content');
      
      // Wait for debouncing
      await new Promise(resolve => setTimeout(resolve, 500));
      
      expect(events.length).toBeGreaterThan(0);
      const addEvent = events.find(e => e.type === 'add' && e.path.endsWith('watched.txt'));
      expect(addEvent).toBeDefined();
    });
  });

  describe('Error Handling', () => {
    it('should handle non-existent files gracefully', async () => {
      await expect(service.readFile('nonexistent.txt')).rejects.toThrow();
    });

    it('should handle permission errors', async () => {
      // This test might not work on all systems, so we'll mock it
      vi.spyOn(fs, 'access').mockRejectedValueOnce(
        Object.assign(new Error('Permission denied'), { code: 'EACCES' }),
      );
      
      await expect(service.readFile('restricted.txt')).rejects.toThrow();
    });
  });

  describe('Service Management', () => {
    it('should provide service statistics', () => {
      const stats = service.getStats();
      
      expect(stats.rootDirectory).toBe(tempDir);
      expect(stats.maxFileSize).toBe(1024 * 1024);
      expect(typeof stats.activeOperations).toBe('number');
      expect(typeof stats.maxConcurrentOps).toBe('number');
    });

    it('should cleanup resources', async () => {
      await service.watchPath(tempDir);
      expect(service.getWatcher().isActive()).toBe(true);
      
      await service.cleanup();
      expect(service.getWatcher().isActive()).toBe(false);
    });
  });

  describe('Concurrency Control', () => {
    it('should provide concurrency statistics', () => {
      const stats = service.getStats();
      expect(stats.maxConcurrentOps).toBe(5);
      expect(typeof stats.activeOperations).toBe('number');
    });
    
    // Note: Actual concurrency limiting is tested in integration
    // The operation limiter ensures proper resource management
  });
});