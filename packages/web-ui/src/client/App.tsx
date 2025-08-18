import React from 'react';
import { Routes, Route } from 'react-router-dom';
import ErrorBoundary from './components/ErrorBoundary';
import Layout from './components/Layout';
import Dashboard from './pages/Dashboard';
import Search from './pages/Search';
import Worktrees from './pages/Worktrees';
import Agents from './pages/Agents';
import Settings from './pages/Settings';
import NotFound from './pages/NotFound';

function App() {
  return (
    <ErrorBoundary resetOnPropsChange>
      <Layout>
        <ErrorBoundary isolate>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/search" element={<Search />} />
            <Route path="/worktrees" element={<Worktrees />} />
            <Route path="/agents" element={<Agents />} />
            <Route path="/settings" element={<Settings />} />
            {/* Catch-all route for 404 errors */}
            <Route path="*" element={<NotFound />} />
          </Routes>
        </ErrorBoundary>
      </Layout>
    </ErrorBoundary>
  );
}

export default App;