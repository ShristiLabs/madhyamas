# Web UI Improvements TODO

## High Priority

### 1. Resizable Panels
- [x] Create ResizablePanel component with drag handle
- [x] Make traffic list/detail panels resizable in TrafficView.tsx
- [x] Make tools sidebar resizable
- [x] Add localStorage persistence for panel sizes

### 2. Traffic List Improvements
- [ ] Add virtual scrolling for performance
- [x] Make columns resizable (using flex layout)
- [ ] Add column show/hide context menu
- [x] Add click-to-sort on columns
- [x] Improve method/status color styling

### 3. Tools Sidebar Redesign
- [x] Convert to vertical tab layout with icons
- [x] Group tabs into categories
- [x] Add collapse/expand mode
- [x] Add keyboard navigation

### 4. Global Improvements
- [x] Add keyboard shortcuts system
- [x] Add keyboard shortcut help modal (?)
- [ ] Add toast notification system (already exists via useToast)
- [ ] Add session state persistence

## Medium Priority

### 5. Traffic Detail Improvements
- [ ] Add side-by-side request/response view option
- [x] Add body search/highlight
- [x] Add prettify/minify toggle for JSON
- [x] Add export options (HTTPie, wget, fetch)
- [ ] Add timing breakdown diagram

### 6. Toolbar Improvements
- [ ] Add advanced filter builder
- [ ] Add filter presets
- [x] Add quick filters (errors, slow, API)
- [ ] Add regex support for URL patterns

### 7. Tool Panels Improvements
- [x] Add search/filter to each panel
- [x] Add bulk actions (select all, enable/disable)
- [x] Add import/export for rules
- [x] Add rule templates

### 8. Mocks Panel
- [ ] Add inline editing
- [x] Add rule templates (404, 500, CORS)
- [x] Add hit history display
- [ ] Add URL pattern validation

### 9. Breakpoints Panel
- [ ] Add conditional breakpoints
- [x] Add hit count display
- [ ] Add temporary "once" breakpoints

### 10. Rewrites Panel
- [ ] Add visual rule builder
- [ ] Add rule preview/test
- [ ] Add rule ordering with drag-drop

## Lower Priority

### 11. Throttle Panel
- [ ] Add custom profile builder
- [ ] Add live bandwidth visualization
- [ ] Add per-host throttling

### 12. Replay Panel
- [ ] Add request diff view
- [x] Add saved collections
- [ ] Add environment variables
- [ ] Add scheduled replay

### 13. gRPC Panel
- [ ] Add protocol buffer decoding
- [ ] Add method list from reflection
- [ ] Add call initiation

### 14. Scripts Panel
- [ ] Add Monaco editor integration
- [ ] Add script templates
- [ ] Add debugging console
- [ ] Add test functionality

### 15. Plugins Panel
- [ ] Add marketplace view
- [ ] Add plugin configuration UI
- [ ] Add execution logs

### 16. Accessibility
- [ ] Add focus indicators
- [x] Add ARIA labels (partial)
- [ ] Add high contrast theme
- [ ] Add font size preferences

### 17. Performance
- [ ] Add React.memo to list items
- [ ] Debounce filter inputs
- [ ] Cancel pending requests on unmount
- [ ] Lazy load panel content

### 18. Mobile/Responsive
- [ ] Add responsive breakpoints
- [ ] Convert sidebars to drawers on mobile
- [ ] Add touch gestures

## Final Steps
- [x] Run ESLint and fix issues
- [x] Run TypeScript build check
- [x] Build production bundle
- [x] Rebuild Docker image
- [ ] Test Docker container
