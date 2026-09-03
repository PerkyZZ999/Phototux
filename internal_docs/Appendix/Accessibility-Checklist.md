# Accessibility Checklist

## Purpose

Executable checklist for PhotoTux accessibility conformance against [29 — Accessibility](../29-Accessibility.md), with cross-links to shell, command, theme, and UX specs. Use during feature design, implementation review, and release evidence. Normative keywords follow [Requirement Keywords](Requirement-Keywords.md). PhotoTux MUST expose role, name, state, availability, and action semantics for actionable UI; keyboard access MUST cover primary menu operations except inherently continuous gestures that have parameterized command equivalents ([00](../00-Introduction.md)).

## How to Use

- Mark each item **Pass**, **Fail**, **N/A**, or **Deferred** with owner and evidence link.
- Deferred items require Decision Register rationale and user-visible limitation notes.
- Toolkit widgets do not grant a free pass; compare AT-SPI (or host) tree to semantic oracle.
- Vendor-neutral: no proprietary screen-reader workflow requirements.

## A. Semantic Tree and Identity

| ID | Requirement | Evidence |
| --- | --- | --- |
| A1 | Every actionable control has a semantic node with stable ID derived from owner+role, not widget pointer or coordinates | tree dump |
| A2 | Application → window → workspace → regions hierarchy matches reading order, not construction order | hierarchy fixture |
| A3 | Decorative wrappers/icons excluded unless they convey state | review — layers panel clipping marker carries `Accessible.ignored: !clips_to_below`; `visible: false` alone left it in the tree as a zero-sized node named "Clipped to layer below" on layers that were not clipped (2026-09-03) |
| A4 | Virtualized lists expose collection size/position and stable active descendant | layer list — the `ListView` is `Accessible.List`, each row `Accessible.ListItem`, and every row's description ends "N of M". Verified over AT-SPI (2026-09-03) |
| A5 | Hidden/collapsed panels expose expandable representative, not ghost actionable subtrees | panel tests |
| A6 | Critical status remains reachable when visual overflow collapses | status region test |
| A7 | Extension UI contributions provide names/roles/states or are rejected | plugin harness |

## B. Names, Descriptions, Values

| ID | Requirement | Evidence |
| --- | --- | --- |
| B1 | Name source order: visible label → descriptor → type+sanitized user name → generic fallback | name oracle |
| B2 | Placeholder/hint is not the accessible name | form tests |
| B3 | Tooltip is not sole name | icon button tests |
| B4 | Ambiguous repeats include context (“Visibility, Layer 3”) | layer tree — eye buttons read "Hide Layer 1" / "Layer 2 — hidden with its group"; row descriptions carry kind, visibility, nesting, mask and clipping (2026-09-03) |
| B5 | Descriptions carry consequence/disabled reason/units/shortcut only when useful | spot check |
| B6 | Values expose range, units, text form, mixed/indeterminate, editability | slider/spin |
| B7 | Private paths/hidden metadata not exposed merely for diagnostics | privacy review |
| B8 | Continuous pointer coordinates do not rename canvas at high frequency | event policy test |

## C. Roles and States

| ID | Requirement | Evidence |
| --- | --- | --- |
| C1 | Roles drawn from documented families (menus, toolbars, trees, tabs, dialogs, canvas summary, etc.) | role map |
| C2 | Custom concepts use closest standard role + attributes; no mislabeling for phrasing | review |
| C3 | States include enabled, focused, selected, expanded, checked/pressed, busy, invalid, required, read-only, modal, multiselectable, indeterminate as applicable | state matrix |
| C4 | Selected ≠ focused ≠ active edit target; all independently exposed | layer rows expose `selected` from the object-selection set and `focused` from the single active layer, separately. AT-SPI over a four-row stack: the active group reads `focused, selectable, selected`, the rest `selectable` only (2026-09-03) |
| C5 | Busy/progress exposed for ops >250 ms | job UI |

## D. Focus

| ID | Requirement | Evidence |
| --- | --- | --- |
| D1 | Exactly one keyboard focus locus per active window | focus audit |
| D2 | Focus indicator visible at 200% scale and high-contrast theme | screenshot + AT |
| D3 | Hover never moves focus | pointer test |
| D4 | Dialog open focuses safe first required/invalid/content control | dialog suite |
| D5 | Dialog close restores invoker or documented fallback | dialog suite |
| D6 | Context menu close restores invoking object | context menu |
| D7 | Deleted focused object falls back sibling→parent→tree policy | layer delete |
| D8 | Async completion never steals focus | export complete |
| D9 | Error summary focus moves only on explicit submit/activation | form validation |
| D10 | Disabled/decorative nodes are not focus landings | tab order |

## E. Keyboard and Shortcuts

| ID | Requirement | Evidence |
| --- | --- | --- |
| E1 | Every named action reachable via menu or command search | action coverage |
| E2 | Region navigation among menus, tools, canvas, panels, status | keyboard map |
| E3 | Layer tree: arrows, expand/collapse, Home/End, type-ahead; Activate sets edit target separately | tree tests |
| E4 | Tabs, tool groups, menus, dialogs follow composite patterns in 29 | suite |
| E5 | Canvas has named zoom/pan/rotation and numeric spatial alternatives | canvas a11y |
| E6 | Shortcut resolver yields to text input and IME | text tool |
| E7 | Sticky/slow/bounce keys and host transforms respected | host note |
| E8 | Ordered shortcut sequences offer adjustable/no-timeout mode | shortcut UI |
| E9 | Unmodified printable tool shortcuts can be disabled | prefs |
| E10 | No simultaneous multi-nonmodifier chord required for core workflows | shortcut policy |
| E11 | Keyboard focus movement alone does not mutate pixels/reorder/toggle visibility/selection without explicit command | invariant test |

## F. Selection, Context, Active Edit Target

| ID | Requirement | Evidence |
| --- | --- | --- |
| F1 | Layer items expose level, parent, child count, visibility, lock, type, mask/effect relations | tree attrs |
| F2 | Multi-selection announces count and anchor | selection |
| F3 | Switching mask vs layer pixels is explicit action + announcement | edit target |
| F4 | Context menu uses focused object; preserves selection per [07](../07-Context-Menus.md) | context tests |
| F5 | Destructive actions announce exact selected/context scope | delete/merge |
| F6 | Context menu is not the sole route to actions | completeness |

## G. Canvas Accessibility

| ID | Requirement | Evidence |
| --- | --- | --- |
| G1 | Canvas not an unlabeled bitmap; baseline summary includes doc/view identity, size, color/profile, zoom/pan/rotation, tool, targets, selection emptiness/bounds, renderer status | canvas node |
| G2 | Structured canvas explorer provides landmarks without pixel dump | explorer |
| G3 | Pixel inspector on request/coarse interval; privacy respected | inspector |
| G4 | Color values include space/profile/channel/alpha | sample |
| G5 | Numeric alternatives for move/transform/guides/crop/selection/zoom/pan | commands |
| G6 | No false claim that keyboard recreates arbitrary freehand gestures | UX copy |
| G7 | Brush sample floods suppressed | event test |

## H. Forms, Dialogs, Tasks

| ID | Requirement | Evidence |
| --- | --- | --- |
| H1 | Fields: label, description, required, value, units, range, invalid, error relation | form suite |
| H2 | Sliders include numeric edit/action | slider |
| H3 | Dialog: title, purpose, modal scope, target, actions, default, destructive disclosure | dialog |
| H4 | Destructive action never initial focus/default | destructive |
| H5 | File chooser host/portal; parent relation + return focus retained | portal |
| H6 | Error summary count + links; advanced groups announce invalid child count | validation |
| H7 | Typing validation rate-limited; does not break IME composition | IME |
| H8 | Tasks expose name, phase, value/total/indeterminate, cancellability | progress |
| H9 | Cancel remains keyboard/AT action; “finishing” explains noninterruptible bounded commit | cancel |

## I. Menus and Command Presentation

| ID | Requirement | Evidence |
| --- | --- | --- |
| I1 | Menu items expose label, disabled reason, shortcut, submenu, destructive description, extension provenance | menu dump |
| I2 | Structure stable while open; safe state updates without reordering | live menu |
| I3 | Disabled items may remain focusable to expose reason under host convention | disabled |
| I4 | Toolbar icon-only controls have names | toolbar |
| I5 | Command search is keyboard accessible and uses stable action IDs | search |

## J. Themes, Contrast, Scaling, Motion

| ID | Requirement | Evidence |
| --- | --- | --- |
| J1 | Contrast meets [25 — Themes](../25-Themes.md) for text, icons, focus, selection, invalid | theme audit |
| J2 | UI usable at 200% scaling; no clipped essential controls | scale |
| J3 | Color is not the only severity/state channel | status |
| J4 | Reduced-motion preference honored; essential state not motion-only | motion |
| J5 | High-contrast mode keeps focus and selection distinguishable | HC theme |

## K. Announcements and Events

| ID | Requirement | Evidence |
| --- | --- | --- |
| K1 | Events derived from committed semantic projections | event source |
| K2 | Coalesce by node/property/revision; order preserved for focus/removal | coalescer |
| K3 | Assertive only for immediate failure/decision; polite otherwise | live regions |
| K4 | No announce-every-frame/tile/brush-sample | flood test |
| K5 | Commit announced as committed, not “image finished” | copy |
| K6 | Device loss / recovery / invariant status announced appropriately | lifecycle |

## L. Host Adapter (Linux AT-SPI)

| ID | Requirement | Evidence |
| --- | --- | --- |
| L1 | Adapter maps roles/states/actions/relations; core owns semantic identity | bridge tests |
| L2 | AT actions revalidated (tree generation, node generation) then routed to actions/commands | action path |
| L3 | AT clients cannot bypass capabilities or domain validation | security |
| L4 | AT-SPI absence does not block editing; reconnect publishes full tree new generation | fault inject |
| L5 | Coordinates converted correctly under scaling | component bounds |

## M. Privacy

| ID | Requirement | Evidence |
| --- | --- | --- |
| M1 | Accessibility tree excludes secrets, absolute paths, hidden private metadata by default | privacy |
| M2 | Diagnostic a11y dumps are local and user-initiated when retaining content | diagnostics |

## N. Concurrency and Performance

| ID | Requirement | Evidence |
| --- | --- | --- |
| N1 | Projection applies deltas with backpressure; UI not blocked on AT | threading |
| N2 | Virtualization maintains semantic continuity under scroll | virt test |
| N3 | Accessibility work respects interactive reservations ([30](../30-Performance.md)) | perf note |

## O. Persistence and Preferences

| ID | Requirement | Evidence |
| --- | --- | --- |
| O1 | A11y-related prefs (announcements density, shortcut disable, timeouts) migrate safely | prefs |
| O2 | Workspace restore restores semantic focus fallbacks, not widget pointers | restore |

## P. Subsystem Sign-Off Matrix

| Feature area | Must pass checklist sections | Primary docs |
| --- | --- | --- |
| New panel | A–F, I, K, N | 05, 29 |
| New dialog | B, D, H, J, K | 26, 29 |
| New tool | E, F, G, I | 06, 14, 29 |
| New command/action | E, I, K | 08, 09, 29 |
| Canvas/view change | G, E, K | 03, 17, 29 |
| Theme change | J, D | 25, 29 |
| Plugin UI/filter | A7, I, H, L2 | 23, 29 |
| Import/export task | H8–H9, K | 22, 29 |
| Lifecycle/recovery | D, K, L4 | 02, 29 |

## Q. Release Evidence Bundle

A release claiming accessibility readiness SHOULD include:

1. semantic oracle vs AT-SPI comparison for primary windows;
2. keyboard coverage report for action registry;
3. focus indicator screenshots at 100%/200% and high contrast;
4. canvas explorer walkthrough recording or scripted AT script;
5. flood-policy test results for brush and navigation;
6. fault injection: AT disconnect, device loss announcement, dialog focus restore;
7. known gaps list with Decision Register links.

## R. Explicit Non-Claims

- PhotoTux does not require cloud a11y services.
- PhotoTux does not claim pixel-perfect recreation of freehand painting by keyboard alone.
- PhotoTux does not depend on a single commercial screen reader.
- Missing AT bus is degraded assistive integration, not document corruption.

## Cross References

- [29 — Accessibility](../29-Accessibility.md)
- [01 — Information Architecture](../01-Information-Architecture.md)
- [07 — Context Menus](../07-Context-Menus.md)
- [08 — Command System](../08-Command-System.md)
- [09 — Shortcut System](../09-Shortcut-System.md)
- [25 — Themes](../25-Themes.md)
- [26 — Dialogs](../26-Dialogs.md)
- [28 — UX Guidelines](../28-UX-Guidelines.md)
- [31 — Testing](../31-Testing.md)
- [Interactive Stability Checklist](Interactive-Stability-Checklist.md) — GUI smoke includes a11y subset
- [Event Catalog](Event-Catalog.md)
- [Performance Budget Ledger](Performance-Budget-Ledger.md)
