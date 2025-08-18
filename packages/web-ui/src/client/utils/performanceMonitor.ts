/**
 * Performance monitoring utilities for the dashboard
 * Ensures compliance with acceptance criteria:
 * - Dashboard loads in < 2 seconds
 * - Quick actions execute in < 500ms
 * - Real-time updates are smooth
 */

interface PerformanceMetric {
  name: string;
  startTime: number;
  endTime?: number;
  duration?: number;
  threshold: number;
  passed?: boolean;
}

class PerformanceMonitor {
  private metrics: Map<string, PerformanceMetric> = new Map();
  private observers: PerformanceObserver[] = [];

  constructor() {
    this.setupPerformanceObservers();
  }

  private setupPerformanceObservers() {
    // Monitor navigation timing
    if ('PerformanceObserver' in window) {
      const navigationObserver = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          if (entry.entryType === 'navigation') {
            const navEntry = entry as PerformanceNavigationTiming;
            this.recordMetric('page-load', navEntry.loadEventStart, navEntry.loadEventEnd, 2000);
            this.recordMetric('dom-content-loaded', navEntry.domContentLoadedEventStart, navEntry.domContentLoadedEventEnd, 1000);
          }
        }
      });

      try {
        navigationObserver.observe({ entryTypes: ['navigation'] });
        this.observers.push(navigationObserver);
      } catch (error) {
        console.warn('Navigation timing observer not supported:', error);
      }

      // Monitor resource loading
      const resourceObserver = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          if (entry.entryType === 'resource') {
            const resourceEntry = entry as PerformanceResourceTiming;
            
            // Monitor API calls
            if (resourceEntry.name.includes('/api/')) {
              this.recordMetric(
                `api-call-${resourceEntry.name.split('/').pop()}`,
                resourceEntry.startTime,
                resourceEntry.responseEnd,
                1000 // 1 second threshold for API calls
              );
            }
          }
        }
      });

      try {
        resourceObserver.observe({ entryTypes: ['resource'] });
        this.observers.push(resourceObserver);
      } catch (error) {
        console.warn('Resource timing observer not supported:', error);
      }

      // Monitor paint timing
      const paintObserver = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          if (entry.entryType === 'paint') {
            this.recordMetric(
              entry.name,
              0,
              entry.startTime,
              entry.name === 'first-contentful-paint' ? 1500 : 2500
            );
          }
        }
      });

      try {
        paintObserver.observe({ entryTypes: ['paint'] });
        this.observers.push(paintObserver);
      } catch (error) {
        console.warn('Paint timing observer not supported:', error);
      }
    }
  }

  startTiming(name: string, threshold: number = 1000): void {
    this.metrics.set(name, {
      name,
      startTime: performance.now(),
      threshold,
    });
  }

  endTiming(name: string): PerformanceMetric | null {
    const metric = this.metrics.get(name);
    if (!metric) {
      console.warn(`No timing started for: ${name}`);
      return null;
    }

    const endTime = performance.now();
    const duration = endTime - metric.startTime;
    const passed = duration <= metric.threshold;

    const completedMetric: PerformanceMetric = {
      ...metric,
      endTime,
      duration,
      passed,
    };

    this.metrics.set(name, completedMetric);

    if (!passed) {
      console.warn(
        `Performance threshold exceeded for ${name}: ${duration.toFixed(2)}ms > ${metric.threshold}ms`
      );
    }

    return completedMetric;
  }

  private recordMetric(name: string, startTime: number, endTime: number, threshold: number): void {
    const duration = endTime - startTime;
    const passed = duration <= threshold;

    this.metrics.set(name, {
      name,
      startTime,
      endTime,
      duration,
      threshold,
      passed,
    });

    if (!passed) {
      console.warn(
        `Performance threshold exceeded for ${name}: ${duration.toFixed(2)}ms > ${threshold}ms`
      );
    }
  }

  measureDashboardLoad(): Promise<PerformanceMetric> {
    return new Promise((resolve) => {
      this.startTiming('dashboard-load', 2000);

      // Wait for dashboard to be fully loaded
      const checkDashboardReady = () => {
        const dashboardElement = document.querySelector('[data-testid="dashboard-container"]') || 
                               document.querySelector('.dashboard') ||
                               document.querySelector('main');
        
        if (dashboardElement) {
          const metric = this.endTiming('dashboard-load');
          if (metric) {
            resolve(metric);
          }
        } else {
          requestAnimationFrame(checkDashboardReady);
        }
      };

      requestAnimationFrame(checkDashboardReady);
    });
  }

  measureQuickAction(actionName: string): {
    start: () => void;
    end: () => PerformanceMetric | null;
  } {
    const timerName = `quick-action-${actionName}`;
    
    return {
      start: () => this.startTiming(timerName, 500),
      end: () => this.endTiming(timerName),
    };
  }

  measureWebSocketLatency(): Promise<number> {
    return new Promise((resolve, reject) => {
      const startTime = performance.now();
      const timeout = setTimeout(() => {
        reject(new Error('WebSocket ping timeout'));
      }, 5000);

      // This would be integrated with the actual WebSocket client
      // For now, we'll simulate it
      setTimeout(() => {
        clearTimeout(timeout);
        const latency = performance.now() - startTime;
        resolve(latency);
      }, Math.random() * 100 + 50); // Simulate 50-150ms latency
    });
  }

  getMetrics(): PerformanceMetric[] {
    return Array.from(this.metrics.values());
  }

  getFailedMetrics(): PerformanceMetric[] {
    return this.getMetrics().filter(metric => metric.passed === false);
  }

  generateReport(): {
    totalMetrics: number;
    passedMetrics: number;
    failedMetrics: number;
    overallScore: number;
    details: PerformanceMetric[];
    recommendations: string[];
  } {
    const metrics = this.getMetrics();
    const failed = this.getFailedMetrics();
    const passed = metrics.filter(m => m.passed === true);

    const recommendations: string[] = [];

    if (failed.some(m => m.name.includes('dashboard-load'))) {
      recommendations.push('Consider code splitting to reduce initial bundle size');
      recommendations.push('Implement lazy loading for non-critical components');
    }

    if (failed.some(m => m.name.includes('api-call'))) {
      recommendations.push('Optimize API response times or implement caching');
      recommendations.push('Consider using data prefetching strategies');
    }

    if (failed.some(m => m.name.includes('quick-action'))) {
      recommendations.push('Optimize quick action handlers');
      recommendations.push('Consider using optimistic UI updates');
    }

    return {
      totalMetrics: metrics.length,
      passedMetrics: passed.length,
      failedMetrics: failed.length,
      overallScore: metrics.length > 0 ? (passed.length / metrics.length) * 100 : 0,
      details: metrics,
      recommendations,
    };
  }

  cleanup(): void {
    this.observers.forEach(observer => observer.disconnect());
    this.observers = [];
    this.metrics.clear();
  }
}

// Singleton instance
export const performanceMonitor = new PerformanceMonitor();

// Utility functions for measuring common dashboard operations
export const measureDashboardLoad = () => performanceMonitor.measureDashboardLoad();
export const measureQuickAction = (actionName: string) => performanceMonitor.measureQuickAction(actionName);
export const measureWebSocketLatency = () => performanceMonitor.measureWebSocketLatency();

// Hook for React components
export function usePerformanceMonitoring() {
  const startTiming = (name: string, threshold?: number) => {
    performanceMonitor.startTiming(name, threshold);
  };

  const endTiming = (name: string) => {
    return performanceMonitor.endTiming(name);
  };

  const getReport = () => {
    return performanceMonitor.generateReport();
  };

  return {
    startTiming,
    endTiming,
    getReport,
    measureQuickAction,
    measureWebSocketLatency,
  };
}

// Development helper to log performance metrics
if (process.env.NODE_ENV === 'development') {
  // Log metrics every 30 seconds in development
  setInterval(() => {
    const report = performanceMonitor.generateReport();
    if (report.totalMetrics > 0) {
      console.group('Dashboard Performance Report');
      console.log(`Overall Score: ${report.overallScore.toFixed(1)}%`);
      console.log(`Passed: ${report.passedMetrics}/${report.totalMetrics}`);
      
      if (report.failedMetrics > 0) {
        console.warn('Failed Metrics:', report.details.filter(m => !m.passed));
        console.log('Recommendations:', report.recommendations);
      }
      
      console.groupEnd();
    }
  }, 30000);
}