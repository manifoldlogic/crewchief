# UX Plan for CrewChief Web UI

## Design Philosophy

### Core Principles

1. **Developer-First Design**
   - Keyboard shortcuts for everything
   - Dense information display
   - Dark mode by default
   - Monospace fonts for data

2. **Progressive Complexity**
   - Simple defaults, powerful options
   - Collapsible advanced sections
   - Contextual help tooltips
   - Learn-as-you-go interface

3. **Real-Time Feedback**
   - Live status updates
   - Progress indicators
   - Instant search results
   - WebSocket notifications

4. **Efficiency Over Beauty**
   - Function before form
   - Minimal animations
   - High information density
   - Quick actions prominent

## Visual Design System

### Color Palette

**Dark Theme (Default)**
```css
--background: #0a0a0a
--surface: #141414
--surface-elevated: #1f1f1f
--border: #2a2a2a
--text-primary: #e5e5e5
--text-secondary: #999999
--accent: #3b82f6 /* blue */
--success: #10b981 /* green */
--warning: #f59e0b /* amber */
--error: #ef4444 /* red */
--info: #06b6d4 /* cyan */
```

**Light Theme**
```css
--background: #ffffff
--surface: #f9f9f9
--surface-elevated: #ffffff
--border: #e5e5e5
--text-primary: #171717
--text-secondary: #666666
--accent: #2563eb
--success: #059669
--warning: #d97706
--error: #dc2626
--info: #0891b2
```

### Typography

```css
--font-mono: 'JetBrains Mono', 'Cascadia Code', monospace
--font-sans: 'Inter', system-ui, sans-serif
--font-code: 'Fira Code', 'Consolas', monospace

--text-xs: 11px
--text-sm: 12px
--text-base: 14px
--text-lg: 16px
--text-xl: 20px
```

### Spacing System
```css
--space-1: 4px
--space-2: 8px
--space-3: 12px
--space-4: 16px
--space-6: 24px
--space-8: 32px
```

## Component Library

### Layout Components

**AppShell**
- Fixed header with global search
- Collapsible sidebar navigation
- Main content area with breadcrumbs
- Optional right panel for details
- Fixed footer with status indicators

**SplitPane**
- Resizable panels
- Horizontal/vertical orientation
- Collapse to tab on mobile
- Remember size preferences

**CommandPalette**
- `Cmd+K` activation
- Fuzzy search
- Recent commands
- Keyboard navigation

### Data Display

**DataTable**
- Sortable columns
- Inline filtering
- Row selection
- Virtualized scrolling
- Export capabilities

**CodeViewer**
- Syntax highlighting
- Line numbers
- Search within file
- Diff view support
- Copy button

**LogViewer**
- Auto-scroll
- Filter by level
- Search/grep
- Timestamp toggle
- ANSI color support

### Input Components

**SearchBar**
- Instant results dropdown
- Search history
- Advanced filters toggle
- Save search option
- Keyboard shortcuts display

**CommandInput**
- Auto-completion
- Command history
- Syntax validation
- Parameter hints
- Multi-line support

### Feedback Components

**StatusBadge**
- Color-coded states
- Animated for active states
- Tooltip with details
- Click for actions

**ProgressBar**
- Determinate/indeterminate
- Stacked for multiple operations
- Cancel button
- Time remaining estimate

**Toast**
- Auto-dismiss
- Action buttons
- Stack multiple
- Persist important ones

## Key User Flows

### 1. Quick Search Flow
```
User Intent: Find and open a specific function

1. Press Cmd+K → Command palette opens
2. Type function name → Instant results appear
3. Arrow keys to select → Preview shows inline
4. Enter to open → File opens in Monaco editor
5. Escape to close → Return to previous view

Time: < 3 seconds
```

### 2. Worktree Creation Flow
```
User Intent: Create worktree for new feature

1. Click "+" in worktree panel → Modal opens
2. Type branch name → Suggestions appear
3. Select base branch → Shows commit info
4. Toggle "Copy .env" → Checkbox feedback
5. Click "Create" → Progress shows
6. Auto-navigate → Opens new worktree

Time: < 10 seconds
```

### 3. Agent Spawning Flow
```
User Intent: Start AI agent for task

1. Click "Spawn Agent" → Drawer slides in
2. Select agent type → Shows capabilities
3. Describe task → Character count shown
4. Choose worktree → Dropdown with status
5. Click "Spawn" → Agent card appears
6. Real-time updates → Status changes live

Time: < 15 seconds
```

### 4. Search and Index Flow
```
User Intent: Search after code changes

1. Make code changes → File watcher triggers
2. Notification appears → "Files changed"
3. Click "Re-index" → Progress bar shows
4. Search automatically → Results update
5. Filter by recent → Toggle filter
6. Open result → Split view with diff

Time: < 5 seconds after save
```

## Interaction Patterns

### Keyboard Shortcuts

**Global**
- `Cmd+K` - Command palette
- `Cmd+/` - Global search
- `Cmd+B` - Toggle sidebar
- `Cmd+\` - Toggle terminal
- `Escape` - Close modal/drawer

**Navigation**
- `G then D` - Go to dashboard
- `G then S` - Go to search
- `G then W` - Go to worktrees
- `G then A` - Go to agents
- `G then B` - Go to branches

**Actions**
- `N` - New (context-aware)
- `E` - Edit selected
- `D` - Delete selected
- `R` - Refresh current view
- `?` - Show help

### Mouse Interactions

**Click Behaviors**
- Single click - Select
- Double click - Open/expand
- Right click - Context menu
- Middle click - Open in new tab

**Hover States**
- Show tooltips after 500ms
- Highlight related items
- Preview on hover (delay 1s)
- Show keyboard shortcut

**Drag and Drop**
- Files between worktrees
- Reorder dashboard widgets
- Agent task assignment
- Branch merge operations

## Responsive Design

### Breakpoints
```css
--mobile: 640px
--tablet: 768px
--laptop: 1024px
--desktop: 1280px
--wide: 1536px
```

### Mobile Adaptations
- Stack panels vertically
- Bottom navigation bar
- Swipe gestures
- Simplified tables
- Touch-friendly targets (44px min)

### Desktop Optimizations
- Multi-column layouts
- Keyboard-first navigation
- Dense information display
- Multiple panels open
- Hover interactions

## Accessibility

### WCAG 2.1 AA Compliance
- Color contrast ratios > 4.5:1
- Focus indicators visible
- Screen reader labels
- Semantic HTML
- ARIA landmarks

### Keyboard Navigation
- Tab order logical
- Skip links provided
- No keyboard traps
- All actions accessible
- Shortcut customization

### Screen Reader Support
- Live regions for updates
- Descriptive labels
- Status announcements
- Table headers associated
- Form validation clear

## Animation and Motion

### Principles
- Purpose over decoration
- Respect prefers-reduced-motion
- Duration < 300ms for UI
- Ease-out for enter
- Ease-in for exit

### Standard Animations
```css
--transition-fast: 150ms ease-out
--transition-base: 200ms ease-out
--transition-slow: 300ms ease-out
```

### Use Cases
- Page transitions - slide
- Modal open - fade + scale
- Drawer - slide from edge
- Tooltips - fade only
- Loading - skeleton pulse

## Error Handling

### Error States
- Inline validation messages
- Non-blocking warnings
- Clear error descriptions
- Suggested fixes
- Retry mechanisms

### Empty States
- Helpful illustrations
- Clear call-to-action
- Quick start guides
- Import options
- Documentation links

### Loading States
- Skeleton screens
- Progress indicators
- Estimated time
- Cancel options
- Partial results

## Performance Goals

### Metrics
- First Contentful Paint < 1s
- Time to Interactive < 2s
- Search results < 100ms
- Navigation < 200ms
- Animations @ 60fps

### Optimization Strategies
- Virtual scrolling for lists
- Lazy loading for images
- Code splitting by route
- Web workers for search
- IndexedDB for caching

## Testing and Validation

### Usability Testing
- Task completion rates
- Time to complete tasks
- Error frequency
- User satisfaction scores
- Feature discovery rate

### A/B Testing
- Search result layouts
- Navigation patterns
- Onboarding flows
- Color schemes
- Information density

### Analytics Tracking
- Feature usage
- Search patterns
- Error rates
- Performance metrics
- User paths