//! Frontend skills for PHANTOM's autonomous AI engineering system.
//!
//! Covers component libraries, responsive layout, design tokens, forms, data
//! tables, state management, routing, PWA, accessibility, animations,
//! micro-frontends, SEO, real-time UI, error boundaries, i18n, drag-and-drop,
//! virtual scrolling, offline-first, web components, and performance budgets.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all frontend skills into the provided registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(component_library());
    registry.register(responsive_layout());
    registry.register(design_system_tokens());
    registry.register(form_builder());
    registry.register(data_table_component());
    registry.register(state_management());
    registry.register(routing_architecture());
    registry.register(progressive_web_app());
    registry.register(accessibility_compliance());
    registry.register(animation_system());
    registry.register(micro_frontend());
    registry.register(seo_optimization());
    registry.register(real_time_ui());
    registry.register(error_boundary_system());
    registry.register(internationalization_ui());
    registry.register(drag_and_drop_system());
    registry.register(infinite_scroll_virtualization());
    registry.register(offline_first_architecture());
    registry.register(web_component_bridge());
    registry.register(performance_budget());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn component_library() -> Skill {
    Skill::new(
        "component_library",
        "Component Library",
        "Build design system components with variants, slots, accessibility attributes, \
         documentation, and Storybook integration.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a design system engineer. Generate component library implementations \
         that include:\n\
         - Component API: typed props with sensible defaults, compound component patterns \
           for complex composition (e.g., Menu.Item, Tabs.Panel)\n\
         - Variants: size (sm/md/lg), visual (primary/secondary/ghost/destructive), state \
           (default/hover/active/disabled/loading)\n\
         - Slot pattern: named slots or render props for customizable sections (header, \
           footer, icon, action)\n\
         - Accessibility: ARIA roles, keyboard interaction patterns (arrow keys, Enter, \
           Escape), focus management, screen reader announcements\n\
         - Documentation: JSDoc/TSDoc for every prop, usage examples, do/don't guidelines\n\
         - Storybook stories: default story, all variants, interactive controls, responsive \
           viewport testing, accessibility addon checks\n\
         - Styling: CSS-in-JS or Tailwind with design token references, theme-aware, \
           no hardcoded colors or spacing values\n\
         - Testing: unit tests for logic, interaction tests for user behavior, visual \
           regression snapshots",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn responsive_layout() -> Skill {
    Skill::new(
        "responsive_layout",
        "Responsive Layout",
        "Implement mobile-first responsive designs with breakpoints, container queries, \
         and fluid typography.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a responsive web design specialist. Generate layout implementations \
         that include:\n\
         - Mobile-first approach: base styles target smallest viewport, progressively \
           enhance with min-width media queries\n\
         - Breakpoint system: consistent breakpoints (sm: 640px, md: 768px, lg: 1024px, \
           xl: 1280px, 2xl: 1536px) aligned with design tokens\n\
         - Container queries: component-level responsiveness using @container, independent \
           of viewport width for reusable components\n\
         - Fluid typography: clamp() for font sizes that scale smoothly between min and \
           max viewport widths (e.g., clamp(1rem, 2.5vw, 2rem))\n\
         - Layout patterns: CSS Grid for page-level layout, Flexbox for component-level, \
           auto-fill/auto-fit for responsive grids without media queries\n\
         - Touch considerations: minimum 44x44px touch targets, adequate spacing between \
           interactive elements, swipe gesture support\n\
         - Content reflow: stack side-by-side layouts vertically on narrow screens, hide \
           non-essential elements, collapsible navigation\n\
         - Testing: viewport screenshots at each breakpoint, real-device testing checklist",
    )
    .with_quality_threshold(0.8)
}

fn design_system_tokens() -> Skill {
    Skill::new(
        "design_system_tokens",
        "Design System Tokens",
        "Define design tokens for colors, spacing, typography, and shadows with theme \
         support and dark mode.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Config,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a design token architect. Generate design token systems that include:\n\
         - Color tokens: semantic naming (color.bg.primary, color.text.secondary, \
           color.border.error) mapped to primitive palette values\n\
         - Spacing scale: consistent spacing tokens (space.1 = 4px, space.2 = 8px, etc.) \
           used for margins, padding, gaps\n\
         - Typography tokens: font families, font sizes (text.xs through text.4xl), \
           line heights, font weights, letter spacing\n\
         - Shadow tokens: elevation levels (shadow.sm, shadow.md, shadow.lg) for depth \
           hierarchy\n\
         - Border radius tokens: consistent rounding (radius.sm, radius.md, radius.full)\n\
         - Theme support: light and dark theme token sets with semantic mappings, system \
           preference detection (prefers-color-scheme)\n\
         - Token format: CSS custom properties as output, optionally source from JSON/YAML \
           for multi-platform (web, iOS, Android) via Style Dictionary\n\
         - Dark mode: invert luminance relationships, reduce saturation slightly, ensure \
           WCAG AA contrast ratios in both themes",
    )
    .with_quality_threshold(0.8)
}

fn form_builder() -> Skill {
    Skill::new(
        "form_builder",
        "Form Builder",
        "Build complex forms with validation, multi-step wizards, conditional fields, \
         file upload, and autosave.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a form engineering specialist. Generate form implementations that \
         include:\n\
         - Validation: schema-based validation (Zod/Yup), field-level and form-level \
           validators, real-time validation on blur, debounced on change\n\
         - Multi-step wizards: step indicator, forward/back navigation, per-step validation, \
           state persistence across steps, progress saving\n\
         - Conditional fields: show/hide fields based on other field values, dependent \
           field validation, cascading resets\n\
         - File upload: drag-and-drop zone, file type and size validation, upload progress, \
           preview for images, chunked upload for large files\n\
         - Autosave: debounced save to server or localStorage, dirty state indicator, \
           conflict detection on submit, recovery from saved draft\n\
         - Accessibility: proper label associations, error announcements via aria-live, \
           fieldset/legend for groups, tab order management\n\
         - Performance: uncontrolled inputs where possible (react-hook-form pattern), \
           avoid re-rendering entire form on single field change\n\
         - Server integration: optimistic submission, server-side validation error mapping \
           to field-level errors, CSRF protection",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn data_table_component() -> Skill {
    Skill::new(
        "data_table_component",
        "Data Table Component",
        "Build data tables with sorting, filtering, pagination, column resizing, \
         virtual scrolling, and export functionality.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a data table component engineer. Generate data table implementations \
         that include:\n\
         - Sorting: multi-column sort with visual indicators, server-side sort delegation \
           for large datasets, stable sort algorithm\n\
         - Filtering: per-column filters (text, select, date range, numeric range), global \
           search across all columns, filter composition (AND/OR)\n\
         - Pagination: page-based and cursor-based navigation, configurable page sizes, \
           total count display, URL sync for shareable state\n\
         - Column resizing: drag-to-resize handles, minimum column widths, persisted \
           column widths to localStorage\n\
         - Virtual scrolling: render only visible rows (react-window/tanstack-virtual), \
           handle variable row heights, maintain scroll position on data update\n\
         - Row selection: single and multi-select with checkbox column, select-all across \
           pages, bulk action toolbar\n\
         - Export: CSV/Excel export with current filters applied, column visibility respected\n\
         - Accessibility: proper table semantics, aria-sort on headers, keyboard navigation \
           between cells, screen reader row/column announcements",
    )
    .with_quality_threshold(0.85)
}

fn state_management() -> Skill {
    Skill::new(
        "state_management",
        "State Management",
        "Implement global state with Redux, Zustand, or Jotai patterns including \
         selectors, middleware, persistence, and devtools.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a frontend state management architect. Generate state management \
         implementations that include:\n\
         - Store design: flat normalized state shape, separate UI state from domain state, \
           co-locate server cache state (React Query/SWR) separately from client state\n\
         - Selectors: memoized derived state (reselect/zustand selectors), avoid unnecessary \
           re-renders by selecting minimal required state slices\n\
         - Middleware: logging middleware for debugging, async action middleware (thunks/sagas), \
           validation middleware for state invariants\n\
         - Persistence: selective persistence to localStorage/sessionStorage, migration \
           strategy for schema changes, encryption for sensitive state\n\
         - Devtools: Redux DevTools integration for time-travel debugging, action logging, \
           state snapshot export/import\n\
         - Type safety: fully typed store, actions, and selectors with TypeScript, \
           discriminated union action types\n\
         - Patterns: optimistic updates with rollback, undo/redo via action history, \
           computed state without duplication\n\
         - Testing: store unit tests with initial state setup, action dispatch assertions, \
           selector output verification",
    )
    .with_quality_threshold(0.8)
}

fn routing_architecture() -> Skill {
    Skill::new(
        "routing_architecture",
        "Routing Architecture",
        "Design client-side routing with code splitting, route guards, nested layouts, \
         breadcrumbs, and prefetching.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a frontend routing architect. Generate routing implementations that \
         include:\n\
         - Route-based code splitting: React.lazy / dynamic import() for each route, \
           loading fallback components per route\n\
         - Route guards: authentication guards (redirect to login), authorization guards \
           (role-based access), data-loading guards (fetch before render)\n\
         - Nested layouts: shared layout components (sidebar, header) that persist across \
           child route navigation without re-mounting\n\
         - Breadcrumbs: auto-generated from route hierarchy, customizable labels per route, \
           structured data (JSON-LD BreadcrumbList)\n\
         - Prefetching: prefetch next-likely routes on link hover or viewport intersection, \
           preload both code and data\n\
         - URL state: sync filter/sort/pagination state to URL search params for shareable \
           and bookmarkable views\n\
         - Scroll restoration: restore scroll position on back navigation, scroll to top \
           on forward navigation\n\
         - Error handling: per-route error boundaries, 404 catch-all route, redirect rules \
           for renamed or removed routes",
    )
    .with_quality_threshold(0.8)
}

fn progressive_web_app() -> Skill {
    Skill::new(
        "progressive_web_app",
        "Progressive Web App",
        "Implement PWA with service worker, offline support, push notifications, and \
         install prompt handling.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a Progressive Web App engineer. Generate PWA implementations that \
         include:\n\
         - Service worker: Workbox-based with precaching for app shell, runtime caching \
           strategies (CacheFirst for assets, NetworkFirst for API, StaleWhileRevalidate \
           for images)\n\
         - Offline support: offline fallback page, cached API responses for offline reads, \
           background sync queue for offline writes\n\
         - Push notifications: VAPID key setup, subscription management, notification \
           payload handling, permission request UX (delay until user context warrants it)\n\
         - Install prompt: listen for beforeinstallprompt, show custom install banner at \
           contextual moment, track install analytics\n\
         - Web app manifest: name, icons (512x512 maskable + purpose), theme_color, \
           background_color, display: standalone, shortcuts, screenshots\n\
         - Update flow: detect new service worker, prompt user to refresh, skip waiting \
           strategy with user consent\n\
         - Cache management: version-based cache busting, size-limited caches with LRU \
           eviction, cache cleanup on activation\n\
         - Testing: Lighthouse PWA audit checklist, offline simulation testing, cross-browser \
           service worker behavior verification",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn accessibility_compliance() -> Skill {
    Skill::new(
        "accessibility_compliance",
        "Accessibility Compliance",
        "Ensure WCAG 2.1 AA compliance with ARIA patterns, keyboard navigation, screen \
         reader support, and color contrast verification.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend, AgentRole::Qa],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are an accessibility compliance specialist. Generate accessibility \
         implementations that include:\n\
         - ARIA patterns: correct role, aria-label, aria-describedby, aria-expanded, \
           aria-live regions for dynamic content announcements\n\
         - Keyboard navigation: all interactive elements focusable, logical tab order, \
           arrow key navigation within composite widgets (menus, tabs, listboxes)\n\
         - Screen reader support: meaningful alt text, heading hierarchy (h1-h6 in order), \
           landmark regions (main, nav, aside), visually-hidden descriptive text\n\
         - Color contrast: minimum 4.5:1 for normal text, 3:1 for large text (WCAG AA), \
           do not convey information by color alone\n\
         - Focus management: visible focus indicators (outline, ring), trap focus in modals, \
           return focus to trigger on modal close\n\
         - Semantic HTML: prefer native elements (button, a, input) over div+role, use \
           fieldset/legend for form groups, proper table markup\n\
         - Reduced motion: respect prefers-reduced-motion, disable animations, provide \
           static alternatives for animated content\n\
         - Testing: axe-core automated checks in CI, manual screen reader testing checklist \
           (NVDA, VoiceOver), keyboard-only navigation audit",
    )
    .with_quality_threshold(0.9)
}

fn animation_system() -> Skill {
    Skill::new(
        "animation_system",
        "Animation System",
        "Build animation libraries with transitions, gestures, scroll-driven animations, \
         and reduced motion support.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a frontend animation engineer. Generate animation system implementations \
         that include:\n\
         - Transition system: enter/exit transitions for mount/unmount, shared layout \
           animations (Framer Motion layoutId), spring physics for natural motion\n\
         - Gesture animations: drag with inertia and constraints, pinch-to-zoom, swipe \
           actions with velocity-based completion, long press\n\
         - Scroll-driven animations: progress-based animations tied to scroll position \
           (CSS scroll-timeline or Intersection Observer), parallax effects\n\
         - Reduced motion: wrap all animations in prefers-reduced-motion check, provide \
           instant transitions as fallback, respect OS setting dynamically\n\
         - Performance: use transform and opacity exclusively (compositor-only properties), \
           will-change hints sparingly, avoid layout thrashing\n\
         - Orchestration: stagger children animations, sequence chained animations, \
           interruptible animations that blend from current state\n\
         - Reusable primitives: fade, slide, scale, collapse utilities composable via props\n\
         - Accessibility: pause animations when page is not visible (Page Visibility API), \
           provide skip-animation controls for long sequences",
    )
    .with_quality_threshold(0.8)
}

fn micro_frontend() -> Skill {
    Skill::new(
        "micro_frontend",
        "Micro-Frontend",
        "Architect module federation and micro-frontend systems with shared dependencies, \
         routing, and inter-app communication.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a micro-frontend architecture specialist. Generate micro-frontend \
         designs that include:\n\
         - Module federation: Webpack Module Federation or Vite federation plugin \
           configuration, remote entry points, shared module versioning\n\
         - Shared dependencies: singleton shared libraries (React, design system), version \
           negotiation strategy, fallback to bundled version on mismatch\n\
         - Routing: shell-level route registration, lazy-loaded remote routes, cross-app \
           navigation without full page reload\n\
         - Communication: custom events for loose coupling, shared state bus for tight \
           integration, typed contract definitions between apps\n\
         - Independent deployment: separate CI/CD pipelines per micro-frontend, version \
           manifests, rollback per-app without affecting others\n\
         - Styling isolation: CSS modules or Shadow DOM scoping, shared design tokens, \
           prevent style leakage between apps\n\
         - Error isolation: error boundary per micro-frontend, graceful fallback when \
           remote fails to load, health check before remote import\n\
         - Testing: contract tests between shell and remotes, integration test suite for \
           composed application, performance budget per micro-frontend",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn seo_optimization() -> Skill {
    Skill::new(
        "seo_optimization",
        "SEO Optimization",
        "Implement meta tags, structured data (JSON-LD), sitemaps, robots.txt, canonical \
         URLs, and Open Graph tags.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are an SEO engineering specialist. Generate SEO implementations that \
         include:\n\
         - Meta tags: title (50-60 chars), description (150-160 chars), viewport, charset, \
           robots (index/noindex, follow/nofollow) per page\n\
         - Structured data: JSON-LD for Organization, Product, Article, BreadcrumbList, \
           FAQPage, HowTo schemas as appropriate for content type\n\
         - Sitemap: XML sitemap generation with lastmod, changefreq, priority, image \
           sitemap extension, sitemap index for large sites\n\
         - Robots.txt: allow/disallow rules, sitemap reference, crawl-delay if needed, \
           block admin/API paths\n\
         - Canonical URLs: self-referencing canonicals on every page, canonical across \
           duplicate/paginated content, hreflang for multilingual\n\
         - Open Graph: og:title, og:description, og:image (1200x630), og:type, og:url \
           for social sharing previews\n\
         - Twitter Cards: twitter:card (summary_large_image), twitter:title, twitter:description, \
           twitter:image\n\
         - Technical SEO: clean URL structure, internal linking, 301 redirects for moved \
           content, prerender for JavaScript-heavy pages",
    )
    .with_quality_threshold(0.8)
}

fn real_time_ui() -> Skill {
    Skill::new(
        "real_time_ui",
        "Real-Time UI",
        "Build real-time UIs with WebSocket integration, optimistic updates, conflict \
         resolution, and presence indicators.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a real-time UI engineer. Generate real-time frontend implementations \
         that include:\n\
         - WebSocket management: connection lifecycle (connect, reconnect with exponential \
           backoff, heartbeat/ping-pong, graceful close)\n\
         - Message protocol: typed message envelopes with action, payload, and correlation ID, \
           binary encoding (MessagePack/CBOR) for bandwidth efficiency\n\
         - Optimistic updates: apply mutations to local state immediately, reconcile with \
           server confirmation, rollback on rejection with user notification\n\
         - Conflict resolution: last-write-wins for simple fields, operational transform or \
           CRDT for collaborative text editing, merge strategies for concurrent edits\n\
         - Presence indicators: who is online, who is viewing/editing this resource, \
           cursor positions for collaborative contexts, idle detection\n\
         - Subscription management: subscribe to specific resources/channels, unsubscribe \
           on unmount, multiplexed subscriptions over single connection\n\
         - Offline handling: queue messages while disconnected, replay on reconnect, \
           reconcile missed server events via catch-up query\n\
         - Performance: throttle high-frequency updates (cursor positions: 50ms), batch \
           DOM updates, virtual rendering for large real-time lists",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn error_boundary_system() -> Skill {
    Skill::new(
        "error_boundary_system",
        "Error Boundary System",
        "Implement error boundaries with fallback UI, error reporting integration, \
         recovery strategies, and retry mechanisms.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a frontend error handling engineer. Generate error boundary \
         implementations that include:\n\
         - Granular boundaries: page-level, section-level, and component-level error \
           boundaries with appropriate fallback granularity\n\
         - Fallback UI: user-friendly error messages, illustration/icon, action buttons \
           (retry, go home, contact support), avoid technical jargon\n\
         - Error reporting: capture error + component stack trace, send to Sentry/Bugsnag \
           with user context, deduplicate repeated errors\n\
         - Recovery strategies: retry the failed render, reset component state, navigate \
           to safe route, offer manual refresh\n\
         - Retry mechanism: exponential backoff retry for transient errors (network, API), \
           max retry count, user-initiated retry button\n\
         - Async error handling: catch rejected promises in event handlers and effects, \
           global unhandledrejection listener as safety net\n\
         - Development experience: detailed error overlay in dev mode, component stack \
           trace with source maps, hot-reload recovery\n\
         - Testing: simulate errors in tests to verify boundary behavior, verify fallback \
           renders, verify error reporting calls",
    )
    .with_quality_threshold(0.85)
}

fn internationalization_ui() -> Skill {
    Skill::new(
        "internationalization_ui",
        "Internationalization UI",
        "Implement i18n with locale switching, RTL layout support, number/date formatting, \
         and pluralization rules.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an internationalization engineering specialist. Generate i18n \
         implementations that include:\n\
         - Translation management: key-based message lookups (react-intl, next-intl, \
           i18next), namespace separation per feature, fallback locale chain\n\
         - Locale switching: URL-based locale (path prefix or subdomain), persist preference, \
           detect from Accept-Language header, allow manual override\n\
         - RTL layout: CSS logical properties (margin-inline-start, padding-block-end), \
           dir='rtl' attribute, bidirectional text handling, mirrored icons\n\
         - Number formatting: Intl.NumberFormat for currency, percentages, compact notation, \
           locale-aware decimal/thousands separators\n\
         - Date formatting: Intl.DateTimeFormat for dates/times, relative time (Intl.\
           RelativeTimeFormat), timezone-aware display\n\
         - Pluralization: ICU MessageFormat syntax for plurals (zero/one/two/few/many/other), \
           gender-aware messages, ordinal support\n\
         - Translation workflow: extraction of translatable strings, CI check for missing \
           translations, machine translation fallback for development\n\
         - Performance: lazy-load locale bundles per language, only ship active locale, \
           preload on locale switch hover",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn drag_and_drop_system() -> Skill {
    Skill::new(
        "drag_and_drop_system",
        "Drag and Drop System",
        "Implement drag-and-drop with sortable lists, kanban boards, file drop zones, \
         and touch device support.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a drag-and-drop interaction engineer. Generate DnD implementations \
         that include:\n\
         - Sortable lists: reorder items within a list with animated displacement, drop \
           indicators, keyboard-based reorder (Alt+Arrow)\n\
         - Kanban boards: drag cards between columns, column reordering, drag handle for \
           precise control, auto-scroll near edges\n\
         - File drop zones: visual drop target highlighting on dragenter, file type validation, \
           multiple file support, directory drop\n\
         - Touch support: long-press to initiate drag on touch devices, touch-move tracking, \
           scroll-while-dragging, cancel on edge swipe\n\
         - Drag preview: custom drag overlay that follows pointer, snapshot of dragged element, \
           opacity/scale feedback on source element\n\
         - Accessibility: aria-grabbed, aria-dropeffect, keyboard drag mode (Space to grab, \
           arrows to move, Space to drop, Escape to cancel)\n\
         - Collision detection: rectangle intersection, closest-center, or custom algorithms \
           for nested drop targets\n\
         - State management: optimistic reorder on drop, persist new order to server, \
           rollback on API failure, undo support",
    )
    .with_quality_threshold(0.8)
}

fn infinite_scroll_virtualization() -> Skill {
    Skill::new(
        "infinite_scroll_virtualization",
        "Infinite Scroll Virtualization",
        "Implement virtual scrolling with infinite loading, bidirectional scroll, and \
         dynamic row heights.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a virtual scrolling specialist. Generate virtual scroll implementations \
         that include:\n\
         - Windowed rendering: only render items in viewport plus overscan buffer (typically \
           5-10 items), unmount items scrolled out of view\n\
         - Infinite loading: detect scroll near bottom (Intersection Observer on sentinel \
           element), trigger next page fetch, loading indicator\n\
         - Bidirectional scroll: load older items when scrolling up (chat interfaces), \
           maintain scroll position when prepending items\n\
         - Dynamic row heights: measure items after render, cache measured heights, estimate \
           unmeasured items, recalculate on resize\n\
         - Scroll position restoration: save and restore position on navigation, jump-to-item \
           by index or ID\n\
         - Grid virtualization: 2D virtualization for large grids, column and row windowing, \
           frozen rows/columns\n\
         - Performance: requestAnimationFrame for scroll handler, avoid forced reflows, \
           passive scroll event listeners\n\
         - Libraries: tanstack-virtual for framework-agnostic, react-window/react-virtuoso \
           for React, configuration and customization examples",
    )
    .with_quality_threshold(0.8)
}

fn offline_first_architecture() -> Skill {
    Skill::new(
        "offline_first_architecture",
        "Offline-First Architecture",
        "Design offline-first apps with IndexedDB, sync queues, conflict resolution, \
         and optimistic UI patterns.",
        SkillCategory::Frontend,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Frontend, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an offline-first application architect. Generate offline-first designs \
         that include:\n\
         - Local storage: IndexedDB via Dexie.js or idb for structured data, object stores \
           per entity type, indexed fields for efficient queries\n\
         - Sync queue: persist pending mutations in IndexedDB, replay on reconnect in FIFO \
           order, handle partial sync (some succeed, some fail)\n\
         - Conflict resolution: vector clocks or hybrid logical clocks for ordering, \
           last-write-wins for simple fields, custom merge for complex types, user-facing \
           conflict resolution UI for unresolvable conflicts\n\
         - Optimistic UI: apply mutations locally immediately, display pending state indicator, \
           reconcile with server response, rollback on permanent failure\n\
         - Network detection: navigator.onLine + periodic fetch-based connectivity check, \
           distinguish offline from server-down\n\
         - Data freshness: track last-sync timestamp per entity, show stale data indicators, \
           background sync on reconnect, pull-based delta sync\n\
         - Storage management: quota estimation (navigator.storage.estimate), eviction \
           strategy for low storage, critical vs purgeable data classification\n\
         - Migration: IndexedDB schema versioning, upgrade handlers for structural changes, \
           data migration between versions",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn web_component_bridge() -> Skill {
    Skill::new(
        "web_component_bridge",
        "Web Component Bridge",
        "Build framework-agnostic Web Components with Shadow DOM, slots, custom events, \
         and framework interop.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a Web Components engineer. Generate Web Component implementations \
         that include:\n\
         - Custom element definition: extend HTMLElement, define in customElements registry, \
           observed attributes with attributeChangedCallback\n\
         - Shadow DOM: encapsulated styling via shadow root, :host selector for component \
           shell, ::part() for selective external styling\n\
         - Slots: named slots for content projection (<slot name='header'>), default slot \
           content, slotchange event handling\n\
         - Custom events: dispatch CustomEvent with detail payload, bubble and composed \
           flags for shadow DOM traversal, typed event interfaces\n\
         - Lifecycle: connectedCallback for setup, disconnectedCallback for cleanup, \
           adoptedCallback for document transfer\n\
         - Framework interop: React wrapper (ref forwarding, event mapping), Vue integration \
           (v-model support), Angular element bridge\n\
         - Styling: CSS custom properties as theming API, constructable stylesheets for \
           shared styles, adoptedStyleSheets\n\
         - Testing: test in isolation (no framework), test attribute reflection, test event \
           dispatch, test slot projection, test in consuming frameworks",
    )
    .with_quality_threshold(0.8)
}

fn performance_budget() -> Skill {
    Skill::new(
        "performance_budget",
        "Performance Budget",
        "Enforce performance budgets with CI integration, regression detection, and \
         optimization suggestions.",
        SkillCategory::Frontend,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend, AgentRole::Qa],
        OutputFormat::Config,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a frontend performance budget engineer. Generate performance budget \
         configurations that include:\n\
         - Budget definitions: JavaScript bundle size (<200KB gzipped per route), CSS \
           (<50KB), images (<500KB per page), total transfer (<1MB), LCP (<2.5s), \
           INP (<200ms), CLS (<0.1)\n\
         - CI integration: Lighthouse CI with budget assertions, bundlesize or size-limit \
           GitHub checks, fail PR on budget violation\n\
         - Regression detection: compare against baseline (main branch), alert on >5% \
           increase in any metric, track trends over time\n\
         - Per-route budgets: different budgets for landing page (strictest), dashboard \
           (moderate), admin (relaxed), with documented rationale\n\
         - Third-party accounting: separate budget for first-party vs third-party resources, \
           flag new third-party additions for review\n\
         - Optimization suggestions: automated recommendations when budget is exceeded \
           (tree-shake unused, lazy-load heavy component, compress images)\n\
         - Dashboard: historical trend visualization, budget utilization percentage, \
           top contributors to each metric\n\
         - Alerting: notify team channel when budget approaches threshold (>80% consumed), \
           block merge when exceeded without exception approval",
    )
    .with_quality_threshold(0.85)
}
