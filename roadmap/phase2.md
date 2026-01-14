InboxCleanup Roadmap - Phase 2 (UI Refresh with shadcn-vue)

Goal
- Redo the app layout and filters list using shadcn-vue components and styles.

Scope
- Adopt shadcn-vue for layout primitives, forms, buttons, and list styling.
- Redesign the main layout: sidebar (filters) + main content (email list/detail).
- Improve filter list UX (grouping, toggles, quick actions).
- Keep Gmail-only IMAP logic unchanged (Phase 1 behavior preserved).

Deliverables
- shadcn-vue integrated into the Vue UI.
- New layout scaffold with sidebar + content split.
- Filters list rebuilt with shadcn-vue components (search, enable/disable, edit).
- Updated typography, spacing, and surface styles aligned with shadcn-vue.

Implementation Steps
1) Install and configure shadcn-vue (tailwind + component registry).
2) Replace layout structure in `src/App.vue` with shadcn-vue components.
3) Rebuild `src/components/FilterList.vue` using shadcn-vue list/form controls.
4) Align Email list/detail to new layout spacing and styling.
5) Visual QA across list, detail, and settings modal.

Out of Scope
- Backend changes or IMAP logic changes.
- Multi-provider support.
- New filtering logic beyond UI improvements.
