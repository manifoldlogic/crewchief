/**
 * Services Index
 * 
 * Central export point for all services with factory functions for dependency injection.
 */

export * from './base.js';
export * from './maproom.js';
export * from './worktree.js';
export * from './agent.js';
export * from './config.js';
export * from './monitoring.js';

import {
  BaseService,
  ServiceConfig,
  CacheProvider,
  AuditLogger,
  RedisCache,
  MemoryCache,
  DatabaseAuditLogger,
  ConsoleAuditLogger,
} from './base.js';
import {
  MaproomService,
  MaproomConfig,
  createMaproomService,
} from './maproom.js';
import {
  WorktreeService,
  WorktreeConfig,
  createWorktreeService,
} from './worktree.js';
import {
  AgentService,
  AgentConfig,
  createAgentService,
} from './agent.js';
import {
  ConfigService,
  ConfigServiceConfig,
  createConfigService,
} from './config.js';
import {
  MonitoringService,
  MonitoringConfig,
  createMonitoringService,
} from './monitoring.js';

/**
 * Service container for dependency injection
 */
export interface ServiceContainer {
  cache: CacheProvider;
  auditLogger: AuditLogger;
  maproom: MaproomService;
  worktree: WorktreeService;
  agent: AgentService;
  config: ConfigService;
  monitoring: MonitoringService;
}

/**
 * Configuration for all services
 */
export interface ServicesConfig {
  global?: ServiceConfig;
  maproom?: MaproomConfig;
  worktree?: WorktreeConfig;
  agent?: AgentConfig;
  config?: ConfigServiceConfig;
  monitoring?: MonitoringConfig;
}

/**
 * Create service container with all services initialized
 */
export function createServiceContainer(config: ServicesConfig = {}): ServiceContainer {
  // Initialize shared dependencies
  const cache: CacheProvider = config.global?.redis?.enabled !== false 
    ? new RedisCache(config.global?.redis || {})
    : new MemoryCache();

  const auditLogger: AuditLogger = config.global?.audit?.enabled !== false
    ? new DatabaseAuditLogger()
    : new ConsoleAuditLogger();

  // Initialize services with shared dependencies
  const maproom = createMaproomService(config.maproom, cache, auditLogger);
  const worktree = createWorktreeService(config.worktree, cache, auditLogger);
  const agent = createAgentService(config.agent, cache, auditLogger);
  const configService = createConfigService(config.config, cache, auditLogger);
  const monitoring = createMonitoringService(config.monitoring, cache, auditLogger);

  return {
    cache,
    auditLogger,
    maproom,
    worktree,
    agent,
    config: configService,
    monitoring,
  };
}

/**
 * Service registry for runtime access
 */
let serviceContainer: ServiceContainer | null = null;

/**
 * Initialize global service container
 */
export function initializeServices(config: ServicesConfig = {}): ServiceContainer {
  if (serviceContainer) {
    throw new Error('Services already initialized');
  }
  
  serviceContainer = createServiceContainer(config);
  return serviceContainer;
}

/**
 * Get initialized service container
 */
export function getServices(): ServiceContainer {
  if (!serviceContainer) {
    throw new Error('Services not initialized. Call initializeServices() first.');
  }
  
  return serviceContainer;
}

/**
 * Cleanup services and connections
 */
export async function cleanupServices(): Promise<void> {
  if (!serviceContainer) return;

  try {
    // Stop monitoring
    serviceContainer.monitoring.stopMonitoring();

    // Clear caches
    await serviceContainer.cache.clear();

    // Additional cleanup as needed
    serviceContainer = null;
  } catch (error) {
    console.error('Error during service cleanup:', error);
  }
}

/**
 * Health check for all services
 */
export async function checkServicesHealth(): Promise<Record<string, { healthy: boolean; details?: any }>> {
  if (!serviceContainer) {
    return { services: { healthy: false, details: { error: 'Services not initialized' } } };
  }

  const results: Record<string, { healthy: boolean; details?: any }> = {};

  try {
    results.maproom = await serviceContainer.maproom.healthCheck();
  } catch (error) {
    results.maproom = { healthy: false, details: { error: error.message } };
  }

  try {
    results.worktree = await serviceContainer.worktree.healthCheck();
  } catch (error) {
    results.worktree = { healthy: false, details: { error: error.message } };
  }

  try {
    results.agent = await serviceContainer.agent.healthCheck();
  } catch (error) {
    results.agent = { healthy: false, details: { error: error.message } };
  }

  try {
    results.config = await serviceContainer.config.healthCheck();
  } catch (error) {
    results.config = { healthy: false, details: { error: error.message } };
  }

  try {
    results.monitoring = await serviceContainer.monitoring.healthCheck();
  } catch (error) {
    results.monitoring = { healthy: false, details: { error: error.message } };
  }

  // Cache health check
  results.cache = {
    healthy: serviceContainer.cache.isAvailable(),
    details: { type: serviceContainer.cache.constructor.name },
  };

  return results;
}