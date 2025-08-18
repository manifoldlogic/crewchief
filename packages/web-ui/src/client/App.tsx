import React from 'react';
import { Routes, Route } from 'react-router-dom';
import ErrorBoundary from './components/ErrorBoundary';
import { AppShell } from './components/layout/index';
import Dashboard from './pages/Dashboard';
import Search from './pages/Search';
import Worktrees from './pages/Worktrees';
import Agents from './pages/Agents';
import Settings from './pages/Settings';
import LayoutDemo from './pages/LayoutDemo';
import ProgressDemo from './pages/ProgressDemo';
import NotFound from './pages/NotFound';

function App() {
  return (
    <ErrorBoundary resetOnPropsChange>
      <AppShell showFooter={false}>
        <ErrorBoundary isolate>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/search" element={<Search />} />
            <Route path="/worktrees" element={<Worktrees />} />
            <Route path="/agents" element={<Agents />} />
            <Route path="/agents/active" element={<Agents />} />
            <Route path="/agents/logs" element={<Agents />} />
            <Route path="/agents/performance" element={<Agents />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/layout-demo" element={<LayoutDemo />} />
            <Route path="/progress-demo" element={<ProgressDemo />} />
            {/* Catch-all route for 404 errors */}
            <Route path="*" element={<NotFound />} />
          </Routes>
        </ErrorBoundary>
      </AppShell>
    </ErrorBoundary>
  );
}

export default App;