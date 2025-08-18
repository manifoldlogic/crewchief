import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { SplitPane } from './SplitPane';

/**
 * Demo component showcasing the new layout components
 * This component demonstrates:
 * - AppShell usage (should wrap this component)
 * - SplitPane with both horizontal and vertical splits
 * - Responsive design
 * - Layout persistence
 */
export const LayoutDemo: React.FC = () => {
  return (
    <div className="h-full w-full space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Layout Components Demo</h1>
        <p className="text-muted-foreground">
          This page demonstrates the new layout components including responsive design, 
          split panes, and keyboard navigation.
        </p>
      </div>

      {/* Split Pane Demo */}
      <Card>
        <CardHeader>
          <CardTitle>Split Pane Component</CardTitle>
          <CardDescription>
            Drag the divider to resize. Double-click to reset. Use keyboard arrows when focused.
            Settings are automatically saved to localStorage.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-96 border rounded-lg overflow-hidden">
            <SplitPane
              direction="horizontal"
              initialSplit={30}
              minSize={150}
              persistKey="demo-horizontal"
              onSplitChange={(split) => console.log('Horizontal split changed:', split)}
            >
              <div className="h-full bg-blue-50 dark:bg-blue-950 p-4">
                <h3 className="font-semibold mb-2">Left Panel</h3>
                <p className="text-sm text-muted-foreground">
                  This is the left panel. Try resizing by dragging the divider.
                </p>
                <div className="mt-4 space-y-2">
                  <div className="h-2 bg-blue-200 dark:bg-blue-800 rounded"></div>
                  <div className="h-2 bg-blue-200 dark:bg-blue-800 rounded w-3/4"></div>
                  <div className="h-2 bg-blue-200 dark:bg-blue-800 rounded w-1/2"></div>
                </div>
              </div>
              
              <div className="h-full">
                <SplitPane
                  direction="vertical"
                  initialSplit={60}
                  minSize={100}
                  persistKey="demo-vertical"
                  onSplitChange={(split) => console.log('Vertical split changed:', split)}
                >
                  <div className="h-full bg-green-50 dark:bg-green-950 p-4">
                    <h3 className="font-semibold mb-2">Top Right Panel</h3>
                    <p className="text-sm text-muted-foreground">
                      This demonstrates nested split panes. This is the top right section.
                    </p>
                    <div className="mt-4 grid grid-cols-3 gap-2">
                      <div className="h-16 bg-green-200 dark:bg-green-800 rounded"></div>
                      <div className="h-16 bg-green-200 dark:bg-green-800 rounded"></div>
                      <div className="h-16 bg-green-200 dark:bg-green-800 rounded"></div>
                    </div>
                  </div>
                  
                  <div className="h-full bg-purple-50 dark:bg-purple-950 p-4">
                    <h3 className="font-semibold mb-2">Bottom Right Panel</h3>
                    <p className="text-sm text-muted-foreground">
                      This is the bottom right section of the nested split pane.
                    </p>
                    <div className="mt-4">
                      <div className="flex space-x-2">
                        <div className="w-4 h-4 bg-purple-300 dark:bg-purple-700 rounded-full"></div>
                        <div className="w-4 h-4 bg-purple-300 dark:bg-purple-700 rounded-full"></div>
                        <div className="w-4 h-4 bg-purple-300 dark:bg-purple-700 rounded-full"></div>
                      </div>
                    </div>
                  </div>
                </SplitPane>
              </div>
            </SplitPane>
          </div>
        </CardContent>
      </Card>

      {/* Layout Features */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Responsive Design</CardTitle>
            <CardDescription>Mobile, tablet, and desktop support</CardDescription>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              <li>• Mobile: Overlay sidebar</li>
              <li>• Tablet: Collapsible sidebar</li>
              <li>• Desktop: Full sidebar with collapse</li>
              <li>• Touch-friendly interactions</li>
            </ul>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Keyboard Navigation</CardTitle>
            <CardDescription>Full keyboard support</CardDescription>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              <li>• <kbd className="px-1 py-0.5 text-xs bg-gray-100 dark:bg-gray-800 rounded">⌘/Ctrl + B</kbd> Toggle sidebar</li>
              <li>• <kbd className="px-1 py-0.5 text-xs bg-gray-100 dark:bg-gray-800 rounded">↑↓</kbd> Navigate menu</li>
              <li>• <kbd className="px-1 py-0.5 text-xs bg-gray-100 dark:bg-gray-800 rounded">Enter</kbd> Select item</li>
              <li>• <kbd className="px-1 py-0.5 text-xs bg-gray-100 dark:bg-gray-800 rounded">Esc</kbd> Close mobile menu</li>
            </ul>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Persistence</CardTitle>
            <CardDescription>Settings saved automatically</CardDescription>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              <li>• Sidebar state (collapsed/expanded)</li>
              <li>• Split pane positions</li>
              <li>• Theme preferences</li>
              <li>• Restored on page reload</li>
            </ul>
          </CardContent>
        </Card>
      </div>

      {/* Accessibility Features */}
      <Card>
        <CardHeader>
          <CardTitle>Accessibility Features</CardTitle>
          <CardDescription>Built with accessibility in mind</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 className="font-medium mb-2">ARIA Support</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li>• Proper landmark roles</li>
                <li>• Screen reader friendly</li>
                <li>• Descriptive labels</li>
                <li>• Live regions for updates</li>
              </ul>
            </div>
            <div>
              <h4 className="font-medium mb-2">Focus Management</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li>• Visible focus indicators</li>
                <li>• Logical tab order</li>
                <li>• Keyboard shortcuts</li>
                <li>• Focus trapping in modals</li>
              </ul>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};