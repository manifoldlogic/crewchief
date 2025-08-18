import React, { useState } from 'react';
import { Button } from '../ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';
import { 
  ProgressBar, 
  StatusBadge, 
  ActivityIndicator, 
  useProgressToast,
  ToastContainer,
} from './index';

/**
 * Accessibility Test Component
 * 
 * Tests and demonstrates the accessibility features of progress components.
 * Use this component with screen readers to verify ARIA compliance.
 */
export const AccessibilityTest: React.FC = () => {
  const [progress, setProgress] = useState(50);
  const { toasts, showToast, dismissToast } = useProgressToast();

  const announceProgress = () => {
    // Use aria-live regions for announcements
    const announcement = `Progress updated to ${progress} percent`;
    
    // Create a temporary live region for announcements
    const liveRegion = document.createElement('div');
    liveRegion.setAttribute('aria-live', 'polite');
    liveRegion.setAttribute('aria-atomic', 'true');
    liveRegion.className = 'sr-only';
    liveRegion.textContent = announcement;
    
    document.body.appendChild(liveRegion);
    
    // Remove after announcement
    setTimeout(() => {
      document.body.removeChild(liveRegion);
    }, 1000);
  };

  const testKeyboardNavigation = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      setProgress(prev => Math.min(prev + 10, 100));
      announceProgress();
    }
  };

  const showAccessibleToast = () => {
    showToast({
      title: 'Accessible Notification',
      description: 'This toast notification includes proper ARIA attributes and can be read by screen readers.',
      type: 'info',
      duration: 0, // Persistent for testing
      actions: [
        {
          label: 'Acknowledge',
          onClick: () => {
            console.log('Toast acknowledged');
          },
        },
      ],
    });
  };

  return (
    <div className="space-y-6 p-6">
      {/* Screen reader instructions */}
      <div className="sr-only">
        <h1>Accessibility Test Page for Progress Indicators</h1>
        <p>
          This page tests the accessibility features of progress indicators. 
          All components should be properly announced by screen readers.
          Use Tab to navigate between interactive elements.
          Press Enter or Space on the progress increment button to test live updates.
        </p>
      </div>

      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Accessibility Test</h1>
        <div className="space-x-2">
          <Button 
            onClick={() => {
              setProgress(prev => Math.min(prev + 10, 100));
              announceProgress();
            }}
            onKeyDown={testKeyboardNavigation}
            aria-describedby="progress-help"
          >
            Increment Progress (+10%)
          </Button>
          <Button onClick={showAccessibleToast} variant="outline">
            Show Accessible Toast
          </Button>
        </div>
      </div>

      {/* Hidden help text */}
      <div id="progress-help" className="sr-only">
        Use this button to increase the progress bar by 10 percent. Progress updates will be announced.
      </div>

      {/* Live region for dynamic announcements */}
      <div aria-live="polite" aria-atomic="true" className="sr-only" id="announcements" />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Progress Bars with Accessibility */}
        <Card>
          <CardHeader>
            <CardTitle>Progress Bars</CardTitle>
            <p className="text-sm text-gray-600">
              All progress bars include proper ARIA attributes including role, valuenow, valuemin, and valuemax.
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <label htmlFor="main-progress" className="block text-sm font-medium mb-2">
                Main Progress Task
              </label>
              <ProgressBar
                value={progress}
                label="File Processing"
                showEta
                showPercentage
                startTime={new Date(Date.now() - 30000)}
                aria-label="File processing progress"
                aria-describedby="progress-description"
              />
              <div id="progress-description" className="text-xs text-gray-500 mt-1">
                Current progress: {progress}%. Estimated completion time is shown above.
              </div>
            </div>

            <div>
              <label htmlFor="indeterminate-progress" className="block text-sm font-medium mb-2">
                Indeterminate Progress
              </label>
              <ProgressBar
                value={0}
                indeterminate
                label="Loading Data"
                aria-label="Loading data, progress unknown"
              />
            </div>

            <div>
              <label htmlFor="step-progress" className="block text-sm font-medium mb-2">
                Stepped Progress
              </label>
              <ProgressBar
                value={(3 / 5) * 100}
                label="Setup Wizard"
                steps={{ current: 3, total: 5 }}
                aria-label="Setup wizard, step 3 of 5"
              />
            </div>
          </CardContent>
        </Card>

        {/* Status Badges with Accessibility */}
        <Card>
          <CardHeader>
            <CardTitle>Status Badges</CardTitle>
            <p className="text-sm text-gray-600">
              Status badges include descriptive labels and proper role attributes.
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <h4 className="font-medium mb-2">Agent Status</h4>
              <div className="flex flex-wrap gap-2" role="group" aria-label="Agent status indicators">
                <StatusBadge 
                  status="running" 
                  label="Agent Alpha"
                  aria-label="Agent Alpha is currently running"
                />
                <StatusBadge 
                  status="busy" 
                  label="Agent Beta"
                  aria-label="Agent Beta is busy processing tasks"
                />
                <StatusBadge 
                  status="error" 
                  label="Agent Gamma"
                  aria-label="Agent Gamma has encountered an error"
                />
                <StatusBadge 
                  status="offline" 
                  label="Agent Delta"
                  aria-label="Agent Delta is offline"
                />
              </div>
            </div>

            <div>
              <h4 className="font-medium mb-2">Connection Status</h4>
              <div className="space-y-2">
                <StatusBadge 
                  status="online" 
                  label="WebSocket Connection"
                  showLastUpdate 
                  lastUpdate={new Date()}
                  aria-label="WebSocket connection is online, last updated just now"
                />
                <StatusBadge 
                  status="connecting" 
                  label="Database Connection"
                  pulse
                  aria-label="Database connection is currently being established"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Activity Indicators with Accessibility */}
        <Card>
          <CardHeader>
            <CardTitle>Activity Indicators</CardTitle>
            <p className="text-sm text-gray-600">
              Activity indicators describe ongoing operations with clear status information.
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            <div role="group" aria-label="Current system activities">
              <ActivityIndicator
                type="indexing"
                status="active"
                label="Indexing Repository"
                description="Processing TypeScript files"
                progress={progress}
                variant="detailed"
                eta="2 minutes"
                showDuration
                startTime={new Date(Date.now() - 120000)}
                metadata={{
                  itemsProcessed: Math.floor(progress * 2.5),
                  totalItems: 250,
                  speed: "15 files/sec",
                }}
              />

              <div className="mt-4">
                <ActivityIndicator
                  type="syncing"
                  status="active"
                  label="Synchronizing Changes"
                  variant="compact"
                />
              </div>

              <div className="mt-2">
                <ActivityIndicator
                  type="analyzing"
                  status="completed"
                  label="Code Analysis"
                  variant="compact"
                />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Keyboard Navigation Test */}
        <Card>
          <CardHeader>
            <CardTitle>Keyboard Navigation</CardTitle>
            <p className="text-sm text-gray-600">
              Test keyboard navigation and focus management.
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <p className="text-sm mb-2">
                Use Tab to navigate through these interactive elements:
              </p>
              <div className="space-y-2">
                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={() => setProgress(0)}
                  aria-describedby="reset-help"
                >
                  Reset Progress
                </Button>
                <div id="reset-help" className="sr-only">
                  Resets the progress bar to 0 percent
                </div>

                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={() => setProgress(100)}
                  aria-describedby="complete-help"
                >
                  Complete Progress
                </Button>
                <div id="complete-help" className="sr-only">
                  Sets the progress bar to 100 percent complete
                </div>

                <StatusBadge 
                  status="online" 
                  onClick={() => {
                    console.log('Status badge clicked');
                    announceProgress();
                  }}
                  aria-label="Clickable status badge, currently online"
                />
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Screen Reader Testing Instructions */}
      <Card>
        <CardHeader>
          <CardTitle>Screen Reader Testing Guide</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3 text-sm">
            <h4 className="font-medium">Testing with NVDA/JAWS/VoiceOver:</h4>
            <ol className="list-decimal list-inside space-y-1">
              <li>Navigate to this page with a screen reader active</li>
              <li>Use Tab to move through interactive elements</li>
              <li>Verify that progress bars announce their current value</li>
              <li>Test that status changes are announced in live regions</li>
              <li>Confirm that all buttons have descriptive labels</li>
              <li>Check that grouped elements are properly announced</li>
              <li>Verify that keyboard shortcuts work as expected</li>
            </ol>
            
            <h4 className="font-medium mt-4">Expected Announcements:</h4>
            <ul className="list-disc list-inside space-y-1">
              <li>Progress bars should announce "Progress bar, X percent"</li>
              <li>Status badges should read their full status and label</li>
              <li>Activity indicators should describe the current operation</li>
              <li>Buttons should announce their action and any help text</li>
              <li>Changes should be announced in live regions</li>
            </ul>
          </div>
        </CardContent>
      </Card>

      {/* Toast Container */}
      <ToastContainer
        toasts={toasts}
        onDismissToast={dismissToast}
        position="top-right"
        maxToasts={3}
      />
    </div>
  );
};

export default AccessibilityTest;