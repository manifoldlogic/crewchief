
import { lazy, Suspense } from 'react'
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom'

const Dashboard = lazy(() => import('./pages/Dashboard'))
const Search = lazy(() => import('./pages/Search'))
const Worktrees = lazy(() => import('./pages/Worktrees'))
// Add other lazy routes

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/" element={<Suspense fallback={<div>Loading...</div>}><Dashboard /></Suspense>} />
        <Route path="/search" element={<Suspense fallback={<div>Loading...</div>}><Search /></Suspense>} />
        <Route path="/worktrees" element={<Suspense fallback={<div>Loading...</div>}><Worktrees /></Suspense>} />
        // Add other routes
      </Routes>
    </Router>
  )
}

export default App
