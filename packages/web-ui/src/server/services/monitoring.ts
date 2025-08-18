/**
 * Monitoring Service
 * 
 * Service layer for system monitoring including metrics collection, health checks, and alerting.
 * Implements the Result pattern, caching, audit logging, and authorization.
 */

import { EventEmitter } from 'node:events';
import os from 'node:os';
import fs from 'node:fs/promises';
import { 
  BaseService, 
  Result, 
  success, 
  failure, 
  ServiceError, 
  ServiceConfig,
  CacheProvider,
  AuditLogger,
} from './base.js';
import { getDatabase } from '../../db/connection.js';

export interface MonitoringConfig extends ServiceConfig {
  metricsRetentionDays?: number;
  healthCheckInterval?: number;
  alertThresholds?: {
    cpu?: number;
    memory?: number;
    disk?: number;
    errorRate?: number;
  };
  alertChannels?: {
    email?: string[];
    webhook?: string[];
  };
}

export interface SystemMetrics {
  timestamp: string;
  cpu: {
    usage: number;
    cores: number;
    loadAverage: number[];
  };
  memory: {
    total: number;
    used: number;
    free: number;
    usage: number;
  };
  disk: {
    total: number;
    used: number;
    free: number;
    usage: number;
  };
  network?: {
    bytesReceived: number;
    bytesSent: number;
  };
  processes: {
    total: number;
    running: number;
  };
}

export interface ServiceHealth {
  service: string;
  healthy: boolean;
  lastCheck: string;
  responseTime: number;
  details?: Record<string, any>;
  error?: string;
}

export interface Alert {
  id: string;
  type: 'info' | 'warning' | 'error' | 'critical';
  title: string;
  message: string;
  source: string;
  severity: number;
  timestamp: string;
  acknowledged: boolean;
  acknowledgedBy?: string;
  acknowledgedAt?: string;
  resolved: boolean;
  resolvedBy?: string;
  resolvedAt?: string;
  metadata?: Record<string, any>;
}

export interface MetricsFilter {
  startTime?: string;
  endTime?: string;
  interval?: 'minute' | 'hour' | 'day';
  services?: string[];
}

export interface PerformanceMetrics {
  service: string;
  operation: string;
  count: number;
  averageResponseTime: number;
  minResponseTime: number;
  maxResponseTime: number;
  errorCount: number;
  errorRate: number;
  timestamp: string;
}

export class MonitoringService extends BaseService {
  private metricsRetentionDays: number;
  private healthCheckInterval: number;
  private alertThresholds: {
    cpu: number;
    memory: number;
    disk: number;
    errorRate: number;
  };
  private alertChannels: {
    email: string[];
    webhook: string[];
  };
  private eventEmitter = new EventEmitter();
  private healthCheckTimer?: NodeJS.Timeout;
  private metricsTimer?: NodeJS.Timeout;
  private lastMetrics?: SystemMetrics;
  private performanceTracking = new Map<string, { times: number[]; errors: number; total: number }>();

  constructor(
    config: MonitoringConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    super(config, cache, auditLogger);
    
    this.metricsRetentionDays = config.metricsRetentionDays || 30;
    this.healthCheckInterval = config.healthCheckInterval || 60000; // 1 minute
    this.alertThresholds = {
      cpu: 80,
      memory: 85,
      disk: 90,
      errorRate: 5,
      ...config.alertThresholds,
    };
    this.alertChannels = {
      email: [],
      webhook: [],
      ...config.alertChannels,
    };

    this.startMonitoring();
  }

  /**
   * Start monitoring timers
   */
  private startMonitoring(): void {
    // Health check timer
    this.healthCheckTimer = setInterval(() => {
      this.performHealthChecks();
    }, this.healthCheckInterval);

    // Metrics collection timer
    this.metricsTimer = setInterval(() => {
      this.collectSystemMetrics();
    }, 30000); // Every 30 seconds

    // Cleanup timer
    setInterval(() => {
      this.cleanupOldMetrics();
    }, 24 * 60 * 60 * 1000); // Daily
  }

  /**
   * Stop monitoring
   */
  stopMonitoring(): void {
    if (this.healthCheckTimer) {
      clearInterval(this.healthCheckTimer);
    }
    if (this.metricsTimer) {
      clearInterval(this.metricsTimer);
    }
  }

  /**
   * Collect system metrics
   */
  private async collectSystemMetrics(): Promise<void> {
    try {
      const cpuUsage = await this.getCpuUsage();
      const memoryInfo = this.getMemoryInfo();
      const diskInfo = await this.getDiskInfo();

      const metrics: SystemMetrics = {
        timestamp: new Date().toISOString(),
        cpu: {
          usage: cpuUsage,
          cores: os.cpus().length,
          loadAverage: os.loadavg(),
        },
        memory: memoryInfo,
        disk: diskInfo,
        processes: {
          total: 0, // Would need platform-specific implementation
          running: 0,
        },
      };

      this.lastMetrics = metrics;

      // Store metrics in database
      await this.storeMetrics(metrics);

      // Check thresholds and create alerts
      await this.checkThresholds(metrics);

      // Emit metrics event
      this.eventEmitter.emit('metrics:collected', metrics);
    } catch (error) {
      console.error('Failed to collect system metrics:', error);
    }
  }

  /**
   * Get CPU usage
   */
  private async getCpuUsage(): Promise<number> {
    const startUsage = process.cpuUsage();
    const startTime = process.hrtime.bigint();

    // Wait a small amount of time to measure
    await new Promise(resolve => setTimeout(resolve, 100));

    const endUsage = process.cpuUsage(startUsage);
    const endTime = process.hrtime.bigint();

    const totalTime = Number(endTime - startTime) / 1000000; // Convert to milliseconds
    const cpuTime = (endUsage.user + endUsage.system) / 1000; // Convert to milliseconds

    return Math.min(100, (cpuTime / totalTime) * 100);
  }

  /**
   * Get memory information
   */
  private getMemoryInfo(): SystemMetrics['memory'] {
    const totalMemory = os.totalmem();
    const freeMemory = os.freemem();
    const usedMemory = totalMemory - freeMemory;

    return {
      total: totalMemory,
      used: usedMemory,
      free: freeMemory,
      usage: (usedMemory / totalMemory) * 100,
    };
  }

  /**
   * Get disk information
   */
  private async getDiskInfo(): Promise<SystemMetrics['disk']> {
    try {
      const stats = await fs.statfs(process.cwd());
      const total = stats.blocks * stats.blksize;
      const free = stats.bavail * stats.blksize;
      const used = total - free;

      return {
        total,
        used,
        free,
        usage: (used / total) * 100,
      };
    } catch (error) {
      // Fallback values if statfs is not available
      return {
        total: 0,
        used: 0,
        free: 0,
        usage: 0,
      };
    }
  }

  /**
   * Store metrics in database
   */
  private async storeMetrics(metrics: SystemMetrics): Promise<void> {
    try {
      const db = getDatabase();
      await db.query(`
        INSERT INTO system_metrics (
          timestamp, cpu_usage, cpu_cores, load_average,
          memory_total, memory_used, memory_free, memory_usage,
          disk_total, disk_used, disk_free, disk_usage,
          processes_total, processes_running
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
      `, [
        metrics.timestamp,
        metrics.cpu.usage,
        metrics.cpu.cores,
        JSON.stringify(metrics.cpu.loadAverage),
        metrics.memory.total,
        metrics.memory.used,
        metrics.memory.free,
        metrics.memory.usage,
        metrics.disk.total,
        metrics.disk.used,
        metrics.disk.free,
        metrics.disk.usage,
        metrics.processes.total,
        metrics.processes.running,
      ]);
    } catch (error) {
      console.error('Failed to store metrics:', error);
    }
  }

  /**
   * Check thresholds and create alerts
   */
  private async checkThresholds(metrics: SystemMetrics): Promise<void> {
    const alerts: Omit<Alert, 'id' | 'acknowledged' | 'resolved'>[] = [];

    // CPU threshold
    if (metrics.cpu.usage > this.alertThresholds.cpu) {
      alerts.push({
        type: 'warning',
        title: 'High CPU Usage',
        message: `CPU usage is ${metrics.cpu.usage.toFixed(1)}% (threshold: ${this.alertThresholds.cpu}%)`,
        source: 'system_monitor',
        severity: 3,
        timestamp: metrics.timestamp,
        acknowledgedBy: undefined,
        acknowledgedAt: undefined,
        resolvedBy: undefined,
        resolvedAt: undefined,
        metadata: { cpu: metrics.cpu },
      });
    }

    // Memory threshold
    if (metrics.memory.usage > this.alertThresholds.memory) {
      alerts.push({
        type: 'warning',
        title: 'High Memory Usage',
        message: `Memory usage is ${metrics.memory.usage.toFixed(1)}% (threshold: ${this.alertThresholds.memory}%)`,
        source: 'system_monitor',
        severity: 3,
        timestamp: metrics.timestamp,
        acknowledgedBy: undefined,
        acknowledgedAt: undefined,
        resolvedBy: undefined,
        resolvedAt: undefined,
        metadata: { memory: metrics.memory },
      });
    }

    // Disk threshold
    if (metrics.disk.usage > this.alertThresholds.disk) {
      alerts.push({
        type: 'error',
        title: 'High Disk Usage',
        message: `Disk usage is ${metrics.disk.usage.toFixed(1)}% (threshold: ${this.alertThresholds.disk}%)`,
        source: 'system_monitor',
        severity: 4,
        timestamp: metrics.timestamp,
        acknowledgedBy: undefined,
        acknowledgedAt: undefined,
        resolvedBy: undefined,
        resolvedAt: undefined,
        metadata: { disk: metrics.disk },
      });
    }

    // Create alerts
    for (const alertData of alerts) {
      await this.createAlert(alertData);
    }
  }

  /**
   * Perform health checks on services
   */
  private async performHealthChecks(): Promise<void> {
    const services = ['database', 'redis', 'maproom', 'file_system'];
    const healthResults: ServiceHealth[] = [];

    for (const service of services) {
      const startTime = Date.now();
      try {
        let healthy = false;
        let details: any = {};

        switch (service) {
          case 'database':
            const db = getDatabase();
            await db.query('SELECT 1');
            healthy = true;
            break;
          case 'redis':
            healthy = this.cache.isAvailable();
            break;
          case 'maproom':
            // Would check maproom binary availability
            healthy = true;
            break;
          case 'file_system':
            await fs.access(process.cwd());
            healthy = true;
            break;
        }

        const responseTime = Date.now() - startTime;
        healthResults.push({
          service,
          healthy,
          lastCheck: new Date().toISOString(),
          responseTime,
          details,
        });
      } catch (error) {
        const responseTime = Date.now() - startTime;
        healthResults.push({
          service,
          healthy: false,
          lastCheck: new Date().toISOString(),
          responseTime,
          error: error.message,
        });

        // Create alert for service failure
        await this.createAlert({
          type: 'error',
          title: `Service Health Check Failed`,
          message: `Health check for ${service} failed: ${error.message}`,
          source: 'health_monitor',
          severity: 4,
          timestamp: new Date().toISOString(),
          acknowledged: false,
          resolved: false,
          metadata: { service, error: error.message, responseTime },
        });
      }
    }

    // Store health check results
    await this.storeHealthResults(healthResults);

    // Emit health check event
    this.eventEmitter.emit('health:checked', healthResults);
  }

  /**
   * Store health check results
   */
  private async storeHealthResults(results: ServiceHealth[]): Promise<void> {
    try {
      const db = getDatabase();
      for (const result of results) {
        await db.query(`
          INSERT INTO service_health (
            service, healthy, last_check, response_time, details, error
          ) VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          result.service,
          result.healthy,
          result.lastCheck,
          result.responseTime,
          JSON.stringify(result.details || {}),
          result.error,
        ]);
      }
    } catch (error) {
      console.error('Failed to store health results:', error);
    }
  }

  /**
   * Create an alert
   */
  private async createAlert(alertData: Omit<Alert, 'id' | 'acknowledged' | 'resolved'>): Promise<Alert> {
    const alert: Alert = {
      id: `alert_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
      acknowledged: false,
      resolved: false,
      ...alertData,
    };

    try {
      const db = getDatabase();
      await db.query(`
        INSERT INTO system_alerts (
          id, type, title, message, source, severity, timestamp,
          acknowledged, acknowledged_by, acknowledged_at,
          resolved, resolved_by, resolved_at, metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
      `, [
        alert.id,
        alert.type,
        alert.title,
        alert.message,
        alert.source,
        alert.severity,
        alert.timestamp,
        alert.acknowledged,
        alert.acknowledgedBy,
        alert.acknowledgedAt,
        alert.resolved,
        alert.resolvedBy,
        alert.resolvedAt,
        JSON.stringify(alert.metadata || {}),
      ]);

      // Emit alert event
      this.eventEmitter.emit('alert:created', alert);

      // Send alert notifications
      await this.sendAlertNotifications(alert);
    } catch (error) {
      console.error('Failed to create alert:', error);
    }

    return alert;
  }

  /**
   * Send alert notifications
   */
  private async sendAlertNotifications(alert: Alert): Promise<void> {
    // Email notifications
    for (const email of this.alertChannels.email) {
      try {
        // This would integrate with an email service
        console.log(`Would send email alert to ${email}:`, alert.title);
      } catch (error) {
        console.error(`Failed to send email alert to ${email}:`, error);
      }
    }

    // Webhook notifications
    for (const webhook of this.alertChannels.webhook) {
      try {
        // This would make HTTP requests to webhook URLs
        console.log(`Would send webhook alert to ${webhook}:`, alert.title);
      } catch (error) {
        console.error(`Failed to send webhook alert to ${webhook}:`, error);
      }
    }
  }

  /**
   * Get current system metrics
   */
  async getCurrentMetrics(userId?: string): Promise<Result<SystemMetrics>> {
    try {
      this.checkAuthorization(userId, 'read');

      if (!this.lastMetrics) {
        await this.collectSystemMetrics();
      }

      await this.auditLog('monitoring', 'get_current_metrics', true, {
        userId,
      });

      return success(this.lastMetrics!, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get current metrics: ${error.message}`,
          'METRICS_GET_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get historical metrics
   */
  async getHistoricalMetrics(
    filters: MetricsFilter,
    userId?: string,
  ): Promise<Result<SystemMetrics[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const db = getDatabase();
      let query = 'SELECT * FROM system_metrics WHERE 1=1';
      const params: any[] = [];
      let paramIndex = 1;

      if (filters.startTime) {
        query += ` AND timestamp >= $${paramIndex}`;
        params.push(filters.startTime);
        paramIndex++;
      }

      if (filters.endTime) {
        query += ` AND timestamp <= $${paramIndex}`;
        params.push(filters.endTime);
        paramIndex++;
      }

      query += ' ORDER BY timestamp DESC LIMIT 1000';

      const result = await db.query(query, params);

      const metrics = result.rows.map(row => ({
        timestamp: row.timestamp,
        cpu: {
          usage: row.cpu_usage,
          cores: row.cpu_cores,
          loadAverage: JSON.parse(row.load_average || '[]'),
        },
        memory: {
          total: row.memory_total,
          used: row.memory_used,
          free: row.memory_free,
          usage: row.memory_usage,
        },
        disk: {
          total: row.disk_total,
          used: row.disk_used,
          free: row.disk_free,
          usage: row.disk_usage,
        },
        processes: {
          total: row.processes_total,
          running: row.processes_running,
        },
      }));

      await this.auditLog('monitoring', 'get_historical_metrics', true, {
        userId,
        metadata: { filters, count: metrics.length },
      });

      return success(metrics, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get historical metrics: ${error.message}`,
          'HISTORICAL_METRICS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get service health status
   */
  async getServiceHealth(userId?: string): Promise<Result<ServiceHealth[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const db = getDatabase();
      const result = await db.query(`
        SELECT DISTINCT ON (service) *
        FROM service_health
        ORDER BY service, last_check DESC
      `);

      const healthResults = result.rows.map(row => ({
        service: row.service,
        healthy: row.healthy,
        lastCheck: row.last_check,
        responseTime: row.response_time,
        details: row.details || {},
        error: row.error,
      }));

      return success(healthResults, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get service health: ${error.message}`,
          'SERVICE_HEALTH_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get alerts
   */
  async getAlerts(
    filters: { acknowledged?: boolean; resolved?: boolean; type?: string[] } = {},
    userId?: string,
  ): Promise<Result<Alert[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const db = getDatabase();
      let query = 'SELECT * FROM system_alerts WHERE 1=1';
      const params: any[] = [];
      let paramIndex = 1;

      if (filters.acknowledged !== undefined) {
        query += ` AND acknowledged = $${paramIndex}`;
        params.push(filters.acknowledged);
        paramIndex++;
      }

      if (filters.resolved !== undefined) {
        query += ` AND resolved = $${paramIndex}`;
        params.push(filters.resolved);
        paramIndex++;
      }

      if (filters.type && filters.type.length > 0) {
        query += ` AND type = ANY($${paramIndex})`;
        params.push(filters.type);
        paramIndex++;
      }

      query += ' ORDER BY timestamp DESC LIMIT 100';

      const result = await db.query(query, params);

      const alerts = result.rows.map(row => ({
        id: row.id,
        type: row.type,
        title: row.title,
        message: row.message,
        source: row.source,
        severity: row.severity,
        timestamp: row.timestamp,
        acknowledged: row.acknowledged,
        acknowledgedBy: row.acknowledged_by,
        acknowledgedAt: row.acknowledged_at,
        resolved: row.resolved,
        resolvedBy: row.resolved_by,
        resolvedAt: row.resolved_at,
        metadata: row.metadata || {},
      }));

      return success(alerts, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get alerts: ${error.message}`,
          'ALERTS_GET_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Acknowledge an alert
   */
  async acknowledgeAlert(id: string, userId?: string): Promise<Result<void>> {
    try {
      this.checkAuthorization(userId, 'write');

      const db = getDatabase();
      const result = await db.query(`
        UPDATE system_alerts 
        SET acknowledged = true, acknowledged_by = $1, acknowledged_at = $2
        WHERE id = $3
      `, [userId, new Date().toISOString(), id]);

      if (result.rowCount === 0) {
        return failure(
          new ServiceError(
            `Alert '${id}' not found`,
            'ALERT_NOT_FOUND',
            404,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      await this.auditLog('monitoring', 'acknowledge_alert', true, {
        userId,
        resource: id,
      });

      return success(undefined, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to acknowledge alert: ${error.message}`,
          'ALERT_ACKNOWLEDGE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Track performance metrics
   */
  trackPerformance(service: string, operation: string, responseTime: number, error = false): void {
    const key = `${service}:${operation}`;
    const tracking = this.performanceTracking.get(key) || { times: [], errors: 0, total: 0 };
    
    tracking.times.push(responseTime);
    tracking.total++;
    if (error) tracking.errors++;

    // Keep only last 100 measurements
    if (tracking.times.length > 100) {
      tracking.times = tracking.times.slice(-100);
    }

    this.performanceTracking.set(key, tracking);
  }

  /**
   * Get performance metrics
   */
  async getPerformanceMetrics(userId?: string): Promise<Result<PerformanceMetrics[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const metrics: PerformanceMetrics[] = [];
      
      for (const [key, tracking] of this.performanceTracking.entries()) {
        const [service, operation] = key.split(':');
        const times = tracking.times;
        
        if (times.length === 0) continue;

        metrics.push({
          service,
          operation,
          count: tracking.total,
          averageResponseTime: times.reduce((a, b) => a + b, 0) / times.length,
          minResponseTime: Math.min(...times),
          maxResponseTime: Math.max(...times),
          errorCount: tracking.errors,
          errorRate: (tracking.errors / tracking.total) * 100,
          timestamp: new Date().toISOString(),
        });
      }

      return success(metrics, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get performance metrics: ${error.message}`,
          'PERFORMANCE_METRICS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Subscribe to monitoring events
   */
  subscribe(event: string, callback: (data: any) => void): void {
    this.eventEmitter.on(event, callback);
  }

  /**
   * Unsubscribe from monitoring events
   */
  unsubscribe(event: string, callback: (data: any) => void): void {
    this.eventEmitter.off(event, callback);
  }

  /**
   * Cleanup old metrics
   */
  private async cleanupOldMetrics(): Promise<void> {
    try {
      const cutoffDate = new Date();
      cutoffDate.setDate(cutoffDate.getDate() - this.metricsRetentionDays);

      const db = getDatabase();
      await db.query(`
        DELETE FROM system_metrics 
        WHERE timestamp < $1
      `, [cutoffDate.toISOString()]);

      await db.query(`
        DELETE FROM service_health 
        WHERE last_check < $1
      `, [cutoffDate.toISOString()]);
    } catch (error) {
      console.error('Metrics cleanup failed:', error);
    }
  }

  /**
   * Health check for the Monitoring service
   */
  async healthCheck(): Promise<{ healthy: boolean; details?: any }> {
    try {
      const isMonitoring = !!this.healthCheckTimer && !!this.metricsTimer;
      
      return {
        healthy: isMonitoring,
        details: {
          monitoring: isMonitoring,
          metricsRetentionDays: this.metricsRetentionDays,
          healthCheckInterval: this.healthCheckInterval,
          alertThresholds: this.alertThresholds,
          performanceTracking: this.performanceTracking.size,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    } catch (error) {
      return {
        healthy: false,
        details: {
          error: error.message,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    }
  }
}

// Export factory function for dependency injection
export function createMonitoringService(
  config?: MonitoringConfig,
  cache?: CacheProvider,
  auditLogger?: AuditLogger,
): MonitoringService {
  return new MonitoringService(config, cache, auditLogger);
}