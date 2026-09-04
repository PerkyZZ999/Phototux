# 01 — Information Architecture

## Overview

PhotoTux information architecture translates a complex raster-editing system into a stable user mental model. Experienced raster-editor users should recognize documents, canvases, layers, masks, selections, tools, properties, history, resources, and export without relying on another vendor’s labels or menu geography. Engineers should be able to map every visible concept to an owner, semantic action, accessibility representation, persistence policy, and extension boundary.

This specification governs application hierarchy, workspace organization, navigation, action placement, universal pointer grammar, selection/focus/context distinctions, context-menu completeness, progressive disclosure, naming, discoverability, information scent, accessibility semantics, and extensibility. It deliberately does not select a UI toolkit or prescribe pixel styling. Normative keywords follow [Requirement Keywords](Appendix/Requirement-Keywords.md).

The central rule is:

> Users manipulate document objects through semantic actions presented in context. Views reveal state; they do not become state owners.

## Navigator Thumbnail

The Navigator draws the composite, not a flat rectangle. Its whole purpose is to
say *where in the image* the viewport is, and without the picture it was telling
the user where they were relative to nothing.

The path is: read the composite back from the GPU, downsample it in
`phototux_engine::thumbnail` (a box filter, so a thumbnail does not flicker as
the stride lands on different pixels), encode a PNG through `phototux_io`, and
hand QML a `data:` URL. That URL is the only route from Rust-held pixels into a
QML `Image` here — the canvas is a native wgpu item, and a `QQuickImageProvider`
would mean hand-written C++, which the crate boundaries keep out of everything
but `phototux_canvas`. Base64 is written out rather than depended on: twenty
lines, a fixed standard, held to the RFC 4648 vectors.

Two guards, both required. The rebuild happens only when the composite
generation has changed **and** at least 600 ms have passed: the generation alone
would rebuild on every dab of a stroke, and the clock alone would rebuild a
document nobody is editing. It runs from the engine poll tick, not from the
composite, because a full readback is thirty-odd megabytes at 4K and does not
belong on the path DR-017 budgets.

Two things worth knowing if this is touched again. The readiness test asks
`phototux_canvas::has_document()` rather than the host's own `has_document`
field: that field is synced *after* the composite a new document triggers, so on
the first pass it still says there is nothing to draw. And the QML side tests
`AppSession.navigatorThumbnail.length > 0` rather than `source != ""` —
assigning to a `url` property normalises the value, so comparing the result
against the empty string says nothing useful.

## Resizable Dock Panels

The seam between two stacked dock panels is draggable, and the height is
remembered. Every panel's height used to be a constant in the shell, so
Properties was permanently capped at a fraction of the dock and its longer
groups could only be scrolled.

The grip sits on the top edge of a panel header and resizes the panel *above*
it, which is the seam a user aims at — the line they can see between the two.
It is a hairline until approached. Hidden on the topmost group, and on a
neighbour that has no height of its own to drag: Swatches sizes to its content
and Layers takes whatever is left.

Committed heights live in `DockTopology::panel_heights`, keyed by panel id and
absent for a panel the user has never dragged — absent means "the shell
decides", and writing a default for every panel would freeze a layout decision
the shell should still be free to change. Undocking a panel forgets its height,
and "Reset Workspace" clears them all by replacing the topology.

What the shell decides comes from `PanelDescriptor::default_height`, a
`PanelHeight` of `Fixed(px)`, `FractionOfDock(f)` or `Flexible`. Three cases
rather than a number with sentinels, because a panel must be exactly one and
the difference is real: Navigator is a fixed strip, Properties takes a share of
the dock clamped to leave the stack below room, and Swatches and Layers have no
height of their own to give. It lived in a `switch` in `Main.qml` keyed on the
same five ids the engine already declares — the fourth such switch, beside one
for the glyph, one for whether the panel resizes, and one for whether it has a
body. A panel added to the descriptors but missed in one of them got a
`dots-three` icon and no height, with nothing to say so.

The glyph moved the same way, to `PanelDescriptor::icon_key`. That also brings
panels under `every_icon_key_is_packaged_into_the_qrc`, so a stem the qrc does
not carry fails the build rather than shipping a blank button on the auto-hide
strip. `panelHasBody` deliberately stays in `Main.qml`: it asks which panels
that file declares a body for, which is not something the engine can know.

The serde tagging on `PanelHeight` is a wire contract, since `panelDefaultHeight`
branches on `kind` and reads `value`. Renaming a variant would leave every
branch falling through to "not resizable" — every panel silently unresizable —
while the Rust round-trip stayed correct, so
`a_panel_height_serialises_as_the_shell_reads_it` pins the strings.

Two things the implementation has to get right, both found by getting them
wrong:

- A resized body must pin `Layout.minimumHeight` to its preferred height. Once
  a panel is dragged taller the stack's preferred sizes exceed the dock, and a
  `GridLayout` resolves that by compressing whatever has no minimum — so the
  panel springs straight back. The Layers body fills, so it is what yields.
- A binding that resolves a height by *calling a helper* which reads a `var`
  property does not reliably re-evaluate when that property is reassigned. The
  drag reported the right numbers the whole way while the panel stayed put. An
  `int` revision, bumped on every change and read by each binding, is the
  dependency that cannot be missed — the same `var _ = …` idiom the tool shelf
  uses.

The commit is computed from the pointer position at *release*, not from the last
preview: motion events are not guaranteed, and a resize that depends on having
seen them commits the height it started with.

## Messages

Transient messages are **toasts**, stacked bottom-centre over the canvas. They
are not written into the status bar.

The two are different kinds of thing and were sharing one string. The status bar
carries the document summary — size, zoom, active layer, tool — which is *state*:
true continuously, and refreshed from six places. A message is an *event*: true
once. Putting both in `status_text` meant the next summary refresh silently
erased whatever the user had not read, so a message was only seen by someone
already looking at the footer. `nothing_writes_a_message_into_the_status_bar`
keeps them apart.

That guard reads the host as text, and for a while it read it a line at a time —
which `rustfmt` defeats. A long assignment is wrapped after the `=`, leaving the
message on the next line where the scan could not see it, and two writers sat in
plain sight of a passing test: the colour-profile conversion's warning that it
had rewritten layer data, and the "Unreadable selection op" that names a registry
wiring bug. Both are now toasts. The guard joins wrapped assignments before
scanning, so a formatter's line break cannot hide the next one.

The summary is also the bar's *only* account of document state. To its right sit
per-frame metrics — composite time, frame rate, the GPU badge — which are kept
out of the summary precisely because they would churn its AT-SPI name on every
frame. A second zoom readout had grown up in that cluster, printing the number
the summary already carries four items to its left;
`the_status_bar_does_not_repeat_the_document_summary` fails on any label there
that reads a field the summary states.

Severity is a vocabulary (`NoticeLevel`), not something the presenter infers
from the text — inferring "error" by searching a message for the word "failed"
is the same mistake as classifying a typed error by grepping its `Display`.
Info and warning fade after three seconds; **errors do not fade**, because a
save that did not happen must not scroll past while the user is looking at the
canvas. Every toast can be dismissed by hand, and hovering holds one open: a
message worth reading is often longer than three seconds' worth of reading.

A repeated message counts up in place rather than stacking, so one refused
command clicked four times does not become a wall. The queue is bounded at four
and drops the oldest, since a loop posting every frame would otherwise cover the
canvas it is reporting on.

Making messages reliable exposed one that had always been posted and always
swallowed: the composite runs from a timer and could fire before the GPU side of
the document existed, reporting a failure for a transient state. The host asks
`phototux_canvas::has_document()` rather than matching the error's text.

Dialogs remain for things that need a decision or a long read — the recovery
chooser, the `.ptx` integrity report. A toast is for something the user should
know, not something they must answer.

## Empty Panels

A dock panel with nothing to list shows a placeholder, never a blank rectangle.
An empty sunken rectangle says nothing: a user cannot tell "there is nothing
here yet" from "this panel is broken" or "I am looking at the wrong panel". The
placeholder names what *will* fill the panel, so an empty state carries the same
information scent as a full one.

Where the guidance differs by cause, the placeholder differs too. History has
two: with no document there is nothing to have a history of, and with one open
the list simply has not been written to yet. Layers has one, because a document
always has at least one layer — an empty list there means there is no document,
not that the layers went missing.

Controls belonging to absent content are hidden rather than disabled. The
Layers panel's blend, opacity and lock strip is not shown when no document is
open: chrome for a layer that is not there reads as broken rather than as an
empty document.

Placeholders are deliberately quiet — muted text, a dimmed glyph, no border. An
empty panel is a normal state, not a warning, and one loud enough to compete
with the canvas would be worse than the blank rectangle it replaces.

## Panel Vocabulary

`default_panels()` is the list of panels, and every entry must be one the shell
actually draws. A test reads `qml/Main.qml` for the `panelShowsInDock` call
naming each id and fails when one has no dock — the same shape as the test that
holds submenus to the shell.

That gap was real: `panel.paths` and `panel.character` were declared, offered as
Window-menu toggles, and rendered nowhere. Toggling one changed the persisted
workspace and put nothing on screen, with no feedback of any kind. Their content
lives as the `inspector.path` and `inspector.text` disclosure groups in
Properties; promoting either to a dock of its own is separate work, and
`default_panels()` is where it would start.

The Window menu's panel toggles are generated from that same list rather than
written out beside it, so a panel cannot be offered unless it is declared, and
cannot be declared unless it is drawn. The shell routes them by id prefix rather
than by enumerating cases — it used to carry a comment saying "a new panel needs
no case of its own" directly above seven such cases.

## Panel Placement

Panels go where Photoshop puts them. The reasoning is not deference: panel
position is muscle memory, and a user arriving from Photoshop who has to read
the shell every time has lost the thing a familiar layout is for. Plasma
governs how the chrome *looks*; Photoshop governs where things *are*. The two
are not in tension — a Photoshop layout drawn in Plasma's idiom is the target.

In practice: the tool shelf on the left, the tool options bar under the main
toolbar, the dock column on the right, and menu entries under the Photoshop
menu they belong to. The menu bar itself reads File, Edit, Image, Layer,
Select, Filter, View, Window, Help — `Select` sits between Layer and Filter, not
between Edit and Image, because reaching for Image by position and opening
Select is exactly the cost this rule exists to remove.

The rule cuts both ways: an entry under the Photoshop menu it belongs to has
to *be* the operation that menu implies. Flip Horizontal and Flip Vertical sat
in the Image menu and mirrored the **active layer**, so on a five-layer
document one layer moved and the rest did not — which reads as a bug, because
Photoshop's Image ▸ Image Rotation ▸ Flip Canvas mirrors everything and its
layer flip lives under Edit ▸ Transform. The layer flips moved to
**Edit ▸ Transform**, and **Image ▸ Image Rotation** carries the canvas
operations Photoshop puts there: 180°, 90° CW, 90° CCW and the two canvas
flips. A quarter-turn count rather than three commands, so any rotation is one
undo step and one document rebuild.

The Image menu is four entries — Image Size…, Canvas Size…, Image Rotation ▸,
Color ▸ — in Photoshop's order. It was eleven flat ones, eight of them colour management,
which is the depth of that menu rather than its first screen.

Blend mode, opacity and the lock row live at the **top of the Layers panel**,
not in Properties. They are the most-used control cluster in the application
and the one a user reaches for without looking, and Properties is the panel
with the tightest height budget. Layers holds what every layer always has.

Properties holds what is contextual **to the selection**, and only that. Tool
settings belong to the options bar, which is where Photoshop keeps them: brush
size, hardness and texture, the brush preset picker, the selection combine
modes, the zoom controls. The foreground colour belongs to Swatches, which
already has the wells, the hex field, the palette and the recents. Each of
these was once a second copy inside Properties, editing the same host state
through a second set of controls, in the panel that could least afford the
height. Overlap is permitted where a parameter genuinely qualifies for both
surfaces — align and distribute are in both, as they are in Photoshop — but a
duplicate that exists only because it was written twice is a defect.

Depth is reached by progressive disclosure rather than by showing everything.
A surface opens with the controls most users need and reveals the rest on
demand — disclosure groups in the inspector, flyouts on the tool shelf,
submenus under a menu that would otherwise overflow the window.

## Responsibilities

The information architecture MUST:

- expose one coherent hierarchy from application to workspace, document, view, object, property, and operation;
- make active document, focused view, selected objects, active tool, and contextual target independently observable;
- place actions according to scope and frequency, not implementation ownership;
- preserve equivalent semantic actions across menu, shortcut, toolbar, panel, command search, and context menu;
- keep destructive, irreversible, expensive, or scope-broad operations explicit;
- provide predictable click, drag, double-click, and secondary-click behavior;
- support keyboard, pointer, pen, touchpad, and accessibility technology without creating separate products;
- let extensions add concepts only through declared slots and semantic contracts;
- remain usable with panels hidden, reordered, resized, or moved;
- distinguish document content from workspace state and application preferences.

It SHOULD:

- minimize modal interaction;
- preserve spatial context during property editing;
- show likely next actions near their objects;
- expose advanced control through progressive disclosure rather than parallel basic/advanced modes;
- keep action names stable even when presentation differs by context;
- permit experienced users to work without forcing novice tutorials into the primary surface.

It MAY:

- offer workspace presets;
- allow customizable shortcuts, tool groups, panel layouts, and command favorites;
- adapt low-frequency actions to narrower windows while preserving command search and menu access.

## User Mental Model

Users should understand PhotoTux as five nested ideas:

1. **Application:** manages windows, documents, preferences, resources, and application-wide actions.
2. **Workspace:** arranges views and panels for a task. It is presentation state, not document content.
3. **Document:** owns editable pixels, object graph, color metadata, selection channels, and history.
4. **Canvas view:** shows one document through zoom, pan, rotation, proofing, overlays, and display options.
5. **Object and operation:** layers, masks, selections, guides, and resources are targets; tools and commands act on them.

```mermaid
flowchart TB
    Application[Application]
    Window[Application window]
    Workspace[Workspace]
    Document[Document]
    CanvasView[Canvas view]
    Panel[Panel]
    Object[Document object]
    Action[Semantic action]

    Application --> Window
    Application --> Document
    Window --> Workspace
    Workspace --> CanvasView
    Workspace --> Panel
    CanvasView --> Document
    Panel --> Document
    Document --> Object
    Action --> Object
```

The visible tree is not an ownership tree. A document can appear in multiple canvas views. A panel can follow the active document or be pinned to one. A window can contain one or more workspaces depending on host capabilities. Closing a view removes a projection; closing a document invokes document lifecycle policy.

## Application Hierarchy

### Application Session

The application session owns process-lifetime coordination:

- open document registry;
- window and workspace registry;
- global resource catalogs;
- preferences;
- recent local items;
- recovery discovery;
- command registry;
- extension registry;
- host integration status.

Session state MUST NOT silently become document state. Changing workspace layout MUST NOT dirty a document. Changing a document’s embedded color profile MUST.

### Window

A window is a native top-level host surface. It presents application menus or equivalent action access, workspace chrome, document views, and panels. Window close requests MUST resolve unsaved documents across all views and windows without assuming one document per window.

### Workspace

A workspace is an arrangement of canvas regions, panels, tool presentation, status information, and navigation affordances. Workspaces SHOULD be serializable independently from documents. A workspace preset stores layout and visibility, not current document selections or private file paths unless explicitly documented.

### Document

A document is the unit of editable persistence and history. Document identity MUST remain stable while open. Display name may derive from file name, imported source, untitled sequence, or user-assigned title; identity MUST NOT depend on display name.

### Canvas View

A canvas view owns:

- zoom and pan;
- view rotation and mirroring;
- display proofing mode;
- overlay visibility;
- pixel-grid and guide presentation;
- viewport size and device scale;
- temporary navigation state.

It does not own layers, selection content, masks, or history. Two views of one document MAY show different zoom, channels, overlays, or proofing while sharing mutations.

#### Zoom commands (shipped)

The View menu opens on the four Photoshop puts first, in that order: Zoom In
(`Ctrl+=`), Zoom Out (`Ctrl+-`), Actual Pixels (`Ctrl+1`) and Fit on Screen
(`Ctrl+0`). The step commands walk `Camera2D::ZOOM_STOPS`, a ladder rather than
a multiplier, so zooming in and out again returns to the number you started
from and the ladder passes through 100% exactly. They anchor on the viewport
centre, which needs no pan correction because `pan` *is* the world point drawn
there; the wheel and pinch keep using `view.zoom-at`, which anchors on the
pointer.

Zoom In is bound to `Ctrl+=` rather than the `Ctrl++` Photoshop prints: only
one chord binds to an action, the plus is a shifted key on most layouts, and
`=` is the key people actually press. Photoshop accepts both, and so does the
chord parser — see [09 — Chord spelling](09-Shortcut-System.md#chord-spelling-shipped).

### Panels

Panels expose object structure, properties, resources, navigation, history, and diagnostics. Panels MUST declare whether they follow:

- application scope;
- active document;
- focused canvas view;
- selected object set;
- pinned object or document.

Ambiguous following behavior causes cross-document edits and is forbidden.

## Internal Hierarchy

```text
Application
├── Global actions
│   ├── New/Open/Preferences
│   ├── Resource management
│   └── Window/workspace management
├── Window
│   ├── Primary action access
│   ├── Workspace
│   │   ├── Tool presentation
│   │   ├── Canvas region
│   │   │   ├── Document tab/list item
│   │   │   ├── Canvas view
│   │   │   └── View overlays
│   │   ├── Object panels
│   │   │   ├── Layers
│   │   │   ├── Masks/channels
│   │   │   └── Properties
│   │   ├── Resource panels
│   │   │   ├── Brushes
│   │   │   ├── Gradients/patterns
│   │   │   └── Presets
│   │   └── Temporal panels
│   │       ├── History
│   │       ├── Progress/tasks
│   │       └── Diagnostics
│   └── Status region
└── Document registry
    └── Document
        ├── Object graph
        ├── Selection state
        ├── History
        └── Save/recovery state
```

Primary hierarchy MUST remain intelligible when every optional panel is hidden. Menu/action search, canvas, document identity, active tool, and critical status cannot depend on a particular panel.

## Spatial Layout Model

Exact placement is toolkit- and form-factor-dependent, but the default desktop workspace SHOULD preserve this spatial grammar:

```text
┌─────────────────────────────────────────────────────────────────────┐
│ Application actions │ Document identity │ View controls │ Status   │
├──────────┬──────────────────────────────────────────┬───────────────┤
│ Tools    │ Canvas region                            │ Object stack  │
│          │ ┌──────────────────────────────────────┐ │ Layers        │
│          │ │                                      │ │ Masks         │
│          │ │              Document                │ ├───────────────┤
│          │ │              viewport                │ │ Properties    │
│          │ │                                      │ │ Contextual    │
│          │ └──────────────────────────────────────┘ │ controls      │
├──────────┴──────────────────────────────────────────┴───────────────┤
│ Tool hints │ Coordinates/sample │ Progress │ Color/profile status │
└─────────────────────────────────────────────────────────────────────┘
```

This establishes information scent:

- left or compact edge: what action/tool will be used;
- center: where document content is manipulated;
- object stack: what content is targeted and how it is ordered;
- properties: how selected context is parameterized;
- top: application/document/view scope transitions;
- bottom/status: transient feedback, measurements, progress, and warnings.

Alternative layouts MAY move regions. Labels, roles, action IDs, and following behavior MUST remain stable.

## Object Navigation Hierarchy

Object navigation follows containment and effect order:

```mermaid
flowchart TB
    Document[Document]
    LayerRoot[Layer root]
    Group[Layer group]
    Layer[Layer]
    Mask[Mask]
    Effect[Effect]
    Resource[Referenced resource]

    Document --> LayerRoot
    LayerRoot --> Group
    LayerRoot --> Layer
    Group --> Layer
    Group --> Group
    Layer --> Mask
    Layer --> Effect
    Effect --> Resource
```

The layer panel MUST expose parent/child relation, compositing order, visibility, lock state, active edit target, and relevant mask/effect attachment. Indentation alone is insufficient; accessibility level, expanded state, and position MUST also be available.

Navigation operations:

- Up/Down: move focus among visible siblings or rows.
- Left: collapse expanded node; otherwise move to parent.
- Right: expand collapsed node; otherwise move to first child.
- Home/End: first/last visible item.
- Type-ahead: locate by visible name.
- Range extension: expand object selection from anchor.
- Toggle selection: add/remove focused item without losing prior selection.
- Activate: make focused object the primary edit target when valid.

Focus movement MUST NOT implicitly reorder, delete, toggle visibility, or alter pixels.

## Selection, Focus, Context, and Active Target

These states are related but not interchangeable.

### Selection

Selection identifies one or more objects or a pixel/shape region to which commands may apply. Object selection belongs to interaction state associated with the document and view policy; pixel selection is document content when it affects editing and persistence.

### Focus

Focus identifies the control or view receiving keyboard input. Exactly one focus locus exists per active window. Focus may sit on a layer row that is not selected when keyboard navigation uses a roving focus model.

### Context Target

Context target is the object under a secondary click or invocation point. Opening a context menu MUST NOT necessarily replace the current selection. Policy:

- if context target is inside current selection, actions apply to current selection;
- if context target is outside current selection, non-destructive inspection MAY preserve selection, but mutating context actions MUST clearly identify whether they target clicked object or require selection replacement;
- the default desktop policy SHOULD temporarily context-target the clicked object and preserve prior selection until an action requiring target resolution is chosen;
- keyboard context-menu invocation targets focused item.

### Active Edit Target

The active target is where continuous tools write: layer pixels, mask, channel, path, or other editable surface. It MUST be visibly distinguishable from merely selected rows. Selecting a layer with a mask MUST NOT make it ambiguous whether painting affects layer pixels or mask.

```mermaid
stateDiagram-v2
    [*] --> FocusOnly
    FocusOnly --> Selected: Select action
    Selected --> ActiveTarget: Activate editable surface
    ActiveTarget --> Selected: Activate another surface
    Selected --> ContextInvoked: Open context menu
    FocusOnly --> ContextInvoked: Keyboard context invocation
    ContextInvoked --> Selected: Dismiss or execute
```

## Universal Interaction Grammar

Universal grammar defines defaults. Specialized controls MAY refine behavior only when consistent with semantics, discoverable, and keyboard-accessible.

### Primary Click

Primary click means select, place insertion/cursor, activate a simple control, or establish the target for a subsequent gesture.

- Clicking an unselected object selects it and makes it primary.
- Clicking inside a current multi-selection preserves the set unless the control’s standard selection model requires collapse on release.
- Clicking empty canvas MAY clear object selection only when active tool semantics define that behavior; it MUST NOT silently clear pixel selection.
- Clicking a button invokes one action on release while pointer remains eligible.
- Clicking a disclosure control changes expansion only, not object selection, unless the entire row is the documented target.

### Drag

Drag means direct manipulation with preview:

- threshold MUST prevent small pointer jitter from becoming a drag;
- source, proposed target, operation, and validity MUST remain visible;
- model mutation SHOULD commit once on drop or gesture completion;
- cancel MUST restore pre-gesture state;
- auto-scroll MUST be bounded and proportional near edges;
- dropping on collapsed containers SHOULD reveal target intent without surprise expansion;
- cross-document drag MUST declare copy/move semantics and color/resource conversions.

Brush and continuous tools may emit mergeable command segments for latency and memory control. The history result SHOULD appear as one meaningful gesture.

### Double-Click

Double-click invokes the object’s primary inspect-or-edit action, never a destructive action. Examples:

- layer name: begin rename;
- document item: focus existing view or open view;
- resource: choose or inspect resource;
- numeric field: MAY select value, following host convention;
- canvas: tool-specific only if documented and non-destructive.

Double-click MUST NOT be the sole way to reach an action.

### Secondary Click

Secondary click opens a context menu at the invocation point. It MUST avoid performing a mutation before the user chooses an item. Pen barrel buttons and keyboard context invocation map to the same semantic request.

### Modifiers

Modifiers refine selection, constrain geometry, temporarily switch navigation, or alter copy/move policy. Meanings MUST be consistent across tools. A modifier legend SHOULD appear in tool hints during gestures. Modifier-only features MUST have discoverable non-chord alternatives for accessibility where practical.

### Press, Hold, and Repeat

Press-and-hold MAY reveal grouped tools or temporary alternate behavior but MUST NOT be the sole discovery path. Repeating actions, such as nudging, MUST use bounded transaction merging and stop on focus loss.

## Gesture Resolution Workflow

```mermaid
sequenceDiagram
    participant Host as Host input adapter
    participant Hit as Hit tester
    participant Focus as Focus manager
    participant Tool as Tool state machine
    participant Action as Action resolver
    participant Command as Command router

    Host->>Hit: Normalized pointer event
    Hit-->>Focus: Semantic target path
    Focus->>Tool: Focus and capture context
    Tool->>Action: Gesture intent
    Action->>Action: Resolve scope and availability
    Action->>Command: Submit mutation if committed
    Command-->>Tool: Result or typed failure
```

Hit testing returns semantic target paths, not widget pointers, so action resolution can be tested independent of toolkit. Pointer capture MUST have a cancellation route for focus loss, device removal, window closure, or tool switch.

## Action Model

Every action MUST have:

- stable action identifier;
- concise user-facing name;
- optional description;
- scope;
- target resolver;
- parameter schema;
- availability predicate and disabled reason;
- mutation classification;
- undoability classification;
- default presentation locations;
- optional shortcut;
- accessibility label and state;
- command mapping or view-only handler.

### Action Scopes

```mermaid
flowchart LR
    AppScope[Application scope] --> WindowScope[Window scope]
    WindowScope --> WorkspaceScope[Workspace scope]
    WorkspaceScope --> ViewScope[View scope]
    ViewScope --> DocumentScope[Document scope]
    DocumentScope --> ObjectScope[Object scope]
    ObjectScope --> GestureScope[Gesture scope]
```

Narrower scope wins only when action identity specifies contextual resolution. For example, “Zoom In” is view-scoped; “Duplicate Layer” is object/document-scoped; “Preferences” is application-scoped.

### Action Placement

Placement follows this matrix:

- **Primary application menu:** complete, stable taxonomy; all non-gesture operations.
- **Toolbar/tool shelf:** frequent mode or operation selection.
- **Properties panel:** parameter editing for current selection or active tool.
- **Context menu:** relevant actions for invoked object, complete enough for local work.
- **Canvas overlay:** transient direct manipulation and high-context controls.
- **Command search:** all named actions with scope, shortcut, and availability.
- **Shortcut:** frequent actions with low ambiguity and safe invocation.
- **Status region:** feedback and navigation, not hidden mutation controls.

One semantic action may have many presentations. Implementations MUST NOT duplicate business logic per presentation.

## Menu Architecture

Top-level menu categories SHOULD use stable, vendor-neutral domains:

- File: document lifecycle, import, save, export, print if supported.
- Edit: undo/redo, clipboard, command repetition, preferences where host convention permits.
- Select: pixel and object selection operations.
- View: zoom, pan, rotation, overlays, proofing, view creation.
- Image: document-wide canvas, dimensions, mode, profile, and global operations.
- Layer: layer creation, structure, masks, transforms, and compositing.
- Filters: applicable processing operations grouped by semantic family.
- Tools: tool selection and tool-specific commands.
- Window: windows, workspaces, panels, and document-view navigation.
- Help: handbook, shortcut reference, diagnostics, and about information.

Menu taxonomy MUST not mirror crate structure. Actions unavailable in current scope SHOULD remain visible but disabled when their location teaches capability; disabled reason MUST be available. Actions irrelevant to the product configuration MAY be omitted.

## Context Menu Completeness

Context menus are accelerators, not mystery menus. For each context object, menu design MUST cover these groups when applicable:

1. primary action;
2. create/insert;
3. select and navigate;
4. edit properties or rename;
5. duplicate/copy/paste;
6. structure and ordering;
7. enable/disable, show/hide, lock/unlock;
8. convert/rasterize/flatten with consequences;
9. delete/remove;
10. inspect/reveal diagnostics.

```text
Layer context menu
├── Edit layer properties
├── Rename
├── Duplicate
├── Copy / Paste
├── Create
│   ├── Layer above
│   ├── Group from selection
│   └── Mask
├── Arrange
│   ├── Raise / Lower
│   ├── Move to top / bottom
│   └── Move into group
├── Visibility and locking
├── Convert
│   ├── Rasterize effect result
│   └── Merge/flatten choices
└── Delete
```

Completeness does not mean showing every command. Context menus SHOULD prioritize locally meaningful operations and use submenus only for coherent families. Destructive actions MUST be separated spatially and named precisely. “Remove Mask” and “Apply Mask then Remove” are distinct actions.

Automated tests MUST compare context action sets against object capability declarations to catch missing primary, rename, duplicate, and delete paths.

## Progressive Disclosure

PhotoTux targets experienced users but still needs manageable density. Progressive disclosure has four layers:

1. **Immediate:** current tool, canvas, target, essential parameters, undo, save state.
2. **Nearby:** panel properties, contextual actions, common modifiers, object operations.
3. **On demand:** advanced parameter groups, channel controls, blend details, metadata.
4. **Specialized:** diagnostics, performance budgets, format internals, extension permissions.

Rules:

- disclosure MUST preserve values when collapsed;
- hidden invalid values MUST surface at the collapsed group;
- “Advanced” SHOULD name a coherent concept instead of becoming a dumping ground;
- defaults MUST be safe and visible enough to explain output;
- expert shortcuts MUST not remove menu/action-search discovery;
- modal dialogs SHOULD be reserved for bounded tasks requiring validation before commit;
- live preview SHOULD be cancelable and based on isolated command state.

### Disclosure Group Registry

Collapsible inspector sections are named descriptors, not ad-hoc widgets. Each group declares a stable `id`, a concept `title`, its disclosure `level`, and `open_by_default`. Level 1 content is never collapsible and therefore never registers a group.

The registry order below is also the **layout order**. The Properties panel **MUST** present groups in registration order, and a conformance test **MUST** compare the presented order against the registry rather than relying on review, since the layout is declarative and carries no runtime handle to assert against.

Each group also declares the **subjects** it describes (see *Inspector
subjects* below). A group with no subject could never appear; a group whose
subject list names something that is not a subject would appear for a
selection nobody can make. Both are compile-time errors of the declaration,
checked by tests rather than by review.

| Group id | Subjects | Level | Open by default | Collapsed summary |
| --- | --- | --- | --- | --- |
| `inspector.document` | document | 2 — nearby | yes | canvas size |
| `inspector.guides` | document | 2 — nearby | no | guide count |
| `inspector.color` | document | 3 — on demand | no | soft-proof profile |
| `inspector.diagnostics` | document | 4 — specialized | no | composite GPU time |
| `inspector.text` | text | 2 — nearby | yes | font family |
| `inspector.fill` | fill | 2 — nearby | yes | fill colour |
| `inspector.adjustment` | adjustment | 2 — nearby | yes | primary parameters |
| `inspector.smart` | smart object | 2 — nearby | yes | placement scale |
| `inspector.shape` | shape | 2 — nearby | yes | fill colour |
| `inspector.path` | shape | 2 — nearby | yes | anchor count and closure |
| `inspector.transform` | every layer | 2 — nearby | yes | pending crop extent or rotation |
| `inspector.align` | every layer | 2 — nearby | yes | selected layer count |
| `inspector.mask` | every layer | 2 — nearby | yes | mask density, or none |
| `inspector.styles` | every layer | 3 — on demand | no | style count |
| `inspector.blend-if` | every layer | 3 — on demand | no | active or off |
| `inspector.effects` | every layer | 3 — on demand | no | effect count |

`inspector.selection` and `inspector.brush` were retired rather than renamed:
both described the active *tool*, which is the options bar's subject, not the
inspector's.

Groups at level 3 and above **MUST** default to collapsed; levels 1–2 carry the parameters an active tool or layer kind needs to be usable without further interaction.

Every registered group **MUST** declare a collapsed summary. A summary names the parameter a user is most likely to check before deciding to expand, so a collapsed group still carries information scent ([28 — UX Guidelines](28-UX-Guidelines.md#disclosure-group-header)).

### Where a Parameter Lives

Level 1 and level 2 are different surfaces, not different amounts of the same surface. A parameter reached *during* a gesture — brush size mid-stroke, selection combine mode before a drag, the commit control for an uncommitted crop — belongs on the tool options bar, always visible and never collapsible. Everything else belongs in the inspector's disclosure groups.

The options bar **MUST NOT** become a second inspector. Its test is whether reaching the parameter interrupts the gesture that needs it; a parameter set once per session does not qualify however useful it is.

Overlap between the two surfaces is permitted where a parameter genuinely qualifies for both, provided both edit through the same host operations so neither can drift ([06 — Toolbar System](06-Toolbar-System.md)).

Options-bar content is chosen by **presence**, not disclosure: an absent control means the parameter does not apply to the active tool. Nothing on this bar collapses, so an empty region is a statement about the tool, not about the user's last click.

A control whose absence would strand the user **MUST NOT** live in an overflow region. Commit and cancel for an uncommitted operation are the clear case: they stay outside any scrolling area, because a narrow window scrolling them out of reach leaves the document in a state the user cannot resolve from the surface that created it.

### Reading a Panel That Does Not Fit

The dock is the narrowest surface in the application and its panels routinely
hold more than fits. A panel in that state **MUST** say so without being
touched: the scroll bar is pinned on whenever the content exceeds the height,
and the cut edge fades. `AsNeeded` was not enough — it shows the bar only while
flicking, so a section heading sliced in half by a hard edge read as a
rendering fault rather than as "there is more below".

The scroll bar is an overlay, so the panel body reserves its width at the right
margin whether or not the bar is showing. Making that margin depend on the
bar's visibility would feed the content width back into the height that decides
that visibility.

A label **MUST NOT** repeat the heading directly above it. Three separate
sections opened with a row restating their own group title — "Fill" under
*Fill*, "Character" under *Character*, "Free Transform" under a button reading
*Free Transform* — each spending a row of the panel with the least height to
spend on saying nothing.

### Inspector Badge Rules

Header badges are **derived from host state, never from the group's widgets**. The rules are a pure function of an inspector state snapshot, so a badge is computed identically whether or not the body exists, and each rule is testable without a running shell.

Shipped rules:

| Group | Condition | Severity |
| --- | --- | --- |
| `inspector.adjustment` | a stored parameter lies outside the range the editor can represent | warning |
| `inspector.document` | an active selection's outline shares no pixel with the canvas | warning |
| `inspector.text` | the active text layer's font family is absent from the discovered families | warning |
| `inspector.diagnostics` | the graphics device is lost | error |

A rule **MUST NOT** assert a condition it cannot establish. Font family absence, in particular, is only decidable once font discovery has run; before that the rule stays silent rather than reporting a family as missing on the strength of the fallback list.

**Editor ranges are a registered contract.** The bounds each adjustment parameter can be edited within are declared once and read by both the parameter controls and the out-of-range rule, so the two cannot disagree about what is showable. Editor ranges are narrower than the engine's accepted ranges: a document may legally carry a value this editor cannot reach, and that case **MUST** raise a badge rather than silently pinning the control and misreporting the value. Driving a control to either extreme **MUST NOT** raise a badge, including after the engine re-clamps coupled parameters.

**Presence and disclosure are independent axes.** Presence answers "does this group apply to the current tool, layer kind, and selection?" Disclosure answers "how much of an applicable group is shown?" A group hidden because the eraser is active is not a collapsed group, and re-selecting the brush **MUST NOT** be treated as the user expanding anything. Implementations **MUST NOT** collapse a group as a substitute for hiding an inapplicable one, and **MUST NOT** build a group's body while the group is absent.

Expansion state is **presentation state**: it persists per user alongside workspace state and **MUST NOT** enter the document, document history, or the saved file. Overrides persist sparsely — a group the user has never toggled continues to follow its descriptor default, so changing a default reaches existing users. Safe start clears all overrides.

## Properties and Inspector Architecture

### Inspector subjects

The Properties panel is a **contextual** surface: it describes one subject at a
time and shows nothing else. The subject vocabulary is eight values — the
document, and one per layer kind (raster, group, text, adjustment, shape, fill,
smart object) — declared once in the engine and derived from the layer kinds,
so adding a kind adds a subject rather than leaving a hole.

Every disclosure group declares the subjects it belongs to. Presence is then a
lookup, not a condition written per group: the panel asks the registry whether
a group describes the subject on screen. This exists because the panel used to
decide presence by comparing layer-kind strings written into QML — the layer
vocabulary written a second time, in a language with nothing to check it, where
renaming or adding a kind silently stops sections appearing. Nothing in the
shell may name a layer kind to decide what to show.

A group **MAY** narrow presence further with a live condition of its own (a
styles list with no styles stays away). It **MUST NOT** carry a second subject
rule.

Which subject is on screen is not always the one the selection reports. The
panel offers a **scope**: Layer follows the selection, Document pins the
document subject. Photoshop reaches document properties by having nothing
selected in the layers panel; PhotoTux always has an active layer, so asking is
the honest equivalent, and it is reachable rather than a state a user has to
discover by accident. Selecting a layer returns the panel to the Layer scope,
because selecting something means you want to see it. Scope is presentation
state and **MUST NOT** enter the document or its history.

Chrome that shows a subject's name or glyph **MUST** resolve them from the
subject it is displaying, not from the live selection: in the document scope a
layer is still active, and reading the selection's title and icon puts the
layer's glyph on the document.

### The paint target

A stroke lands either on the layer's pixels or on its mask. Photoshop has no
control for this because it does not need one — the layer and mask thumbnails
in the layers panel *are* the selector, and the ringed one is the target.
PhotoTux presents it the same way, in two places that share one state: the
mask chip on the layer row, and a pair of thumbnail chips at the head of
Properties.

The chips appear **only when the layer has a mask**. A layer without one
offers a choice with a single legal answer, and drawing that choice is worse
than not drawing it: it invites a click that does nothing and implies a mask
that is not there.

This replaced a block headed "Edit target" carrying a four-part summary line
above two labelled buttons. It named a concept that existed nowhere else in
the application, restated facts the layers panel and status bar already show,
and offered the choice unconditionally.

### Organisation

Properties are organized by target, not by implementation module:

```mermaid
flowchart TB
    Target[Resolved target set] --> Common[Common editable properties]
    Target --> Specific[Type-specific properties]
    Target --> Relations[Attached masks and effects]
    Target --> Diagnostics[Read-only diagnostics]
    Common --> PropertyAction[Parameterized semantic action]
    Specific --> PropertyAction
    Relations --> PropertyAction
```

Multi-selection inspectors MUST distinguish:

- same value across targets;
- mixed values;
- unavailable property on some targets;
- partially applicable operation.

Editing a mixed value applies the new explicit value to applicable targets. Partial application MUST be disclosed before commit or returned as a structured result. Property fields MUST define units, range, precision, clamping, and commit policy.

## Naming and Discoverability

Names are domain contracts. They MUST:

- use object first where ambiguity exists: “Layer Opacity,” “Canvas Rotation”;
- use verbs for actions and nouns for objects or modes;
- describe result, not implementation: “Merge Visible Layers,” not “Run Composite Pass”;
- distinguish document-wide “Image” from viewport “View”;
- distinguish “Save” editable document from “Export” delivery representation;
- distinguish “Close View” from “Close Document”;
- distinguish “Clear Selection” from “Delete Selected Pixels”;
- avoid proprietary feature names and metaphors;
- remain stable across presentations.

Command search SHOULD index synonyms, descriptions, menu path, object type, and current shortcut. Synonyms MAY include generic industry terms but displayed canonical name remains vendor-neutral.

### Information Scent

Every discoverable object should advertise:

- what it is;
- current state;
- whether it is editable;
- primary action;
- where additional actions live;
- why an expected action is unavailable.

Icons alone are insufficient for uncommon or destructive actions. Tooltips MUST not be the sole label for accessibility. Status text SHOULD describe consequence during drag: “Move 3 layers into Group A,” not “Drop allowed.”

## Document and View Navigation

Document navigation MUST expose:

- all open documents;
- which documents are modified;
- which have active saves/exports;
- which document each view shows;
- current active document and focused view;
- views belonging to the same document.

Closing follows this decision flow:

```mermaid
flowchart TD
    CloseRequest[Close request] --> TargetKind{View or document}
    TargetKind -->|View| OtherViews{Other views remain}
    OtherViews -->|Yes| CloseView[Close view only]
    OtherViews -->|No| Retained{Background owner remains}
    Retained -->|Yes| CloseView
    Retained -->|No| DirtyCheck{Document modified}
    TargetKind -->|Document| DirtyCheck
    DirtyCheck -->|No| CloseDocument[Close document]
    DirtyCheck -->|Yes| Resolve[Save discard or cancel]
    Resolve --> CloseDocument
```

“Discard” is destructive and MUST identify document and unsaved scope. Multiple-document shutdown SHOULD present a consolidated resolution surface rather than serial prompts.

## Tool Information Architecture

A tool is a modeful interpreter of input, not the command itself. Each tool declares:

- canonical name and stable identifier;
- target requirements;
- active edit-surface compatibility;
- cursor/feedback semantics;
- parameter groups;
- modifier behavior;
- gesture states and cancellation;
- command output;
- accessibility alternative where feasible.

Tool groups MAY reduce shelf size, but the current tool MUST remain visible. Hidden tools MUST be reachable through action search and keyboard navigation. Last-used tool within a group MAY persist as workspace preference.

```mermaid
stateDiagram-v2
    [*] --> Ready
    Ready --> HoverPreview: Pointer enters canvas
    HoverPreview --> Gesturing: Press accepted
    Gesturing --> Gesturing: Move or sample
    Gesturing --> Committing: Release
    Committing --> Ready: Command accepted
    Committing --> ErrorState: Command rejected
    Gesturing --> Cancelled: Escape focus loss or device removal
    Cancelled --> Ready
    ErrorState --> Ready: Feedback acknowledged
```

Switching tools during a gesture MUST cancel or explicitly commit according to tool policy; silent partial commits are forbidden.

## Accessibility Semantics

Accessibility is encoded at semantic model boundaries. Each presented object/action MUST expose:

- role;
- accessible name;
- optional description;
- state: selected, focused, expanded, checked, pressed, busy, invalid, unavailable;
- position and hierarchy where relevant;
- value, range, units, and text representation;
- supported actions;
- relationships: controls, controlled-by, labelled-by, described-by;
- live updates with appropriate priority.

Canvas content is visually dense and cannot be represented as a flat image only. PhotoTux SHOULD expose a navigable document object summary, active layer/mask, selection bounds, cursor coordinates, sampled color, and tool state. Pixel-level exploration MAY use a specialized inspector.

Keyboard requirements:

- logical tab order follows major regions;
- panels use internal arrow navigation rather than thousands of tab stops;
- shortcuts remain disabled while text entry owns conflicting keys;
- Escape unwinds one interaction layer at a time: menu, popover, gesture, temporary mode;
- focus returns to invoking control after modal or popover closure;
- focus MUST not disappear when selected objects are deleted; it moves predictably.

Announcements:

- completed undoable actions SHOULD use polite status;
- destructive confirmation and failed save MUST use assertive status;
- continuous brush movement MUST NOT flood accessibility events;
- progress announcements SHOULD be rate-limited and include operation identity.

## Workspace State and Persistence

Workspace persistence may include:

- panel positions, visibility, sizes, and pinning;
- tool shelf grouping;
- recent tool parameters where safe;
- shortcut customizations;
- view arrangement;
- non-sensitive display options.

It MUST NOT be embedded into the editable document by default. Workspace restoration MUST tolerate missing displays, changed scale, absent extensions, renamed resources, and narrower windows. Offscreen floating panels MUST be recovered into visible bounds.

State precedence:

```mermaid
flowchart LR
    BuiltIn[Built-in defaults] --> UserPrefs[User preferences]
    UserPrefs --> WorkspacePreset[Workspace preset]
    WorkspacePreset --> SessionState[Restored session state]
    SessionState --> Temporary[Temporary interaction state]
```

Higher layers override lower ones only for declared fields. Reset actions SHOULD exist per panel, workspace, shortcut set, and full preferences.

## Extension Points

Future extensions MAY contribute:

- semantic actions;
- import/export handlers;
- filters or effect nodes;
- tools;
- resource types;
- panels or inspector sections;
- command-search metadata.

They MUST NOT:

- mutate document objects outside commands;
- insert arbitrary top-level menu categories without policy;
- intercept unrelated input globally;
- hide core security or save-state indicators;
- assume toolkit widget access;
- block UI thread;
- create inaccessible controls without semantic metadata.

```mermaid
flowchart TB
    ExtensionManifest[Extension manifest] --> ContributionRegistry[Contribution registry]
    ContributionRegistry --> ActionSlots[Action slots]
    ContributionRegistry --> PanelSlots[Panel slots]
    ContributionRegistry --> ToolSlots[Tool slots]
    ActionSlots --> ActionModel[Core action model]
    PanelSlots --> SemanticUI[Semantic presentation contract]
    ToolSlots --> ToolContract[Tool state contract]
    ActionModel --> CommandRouter[Command router]
    ToolContract --> CommandRouter
```

Contribution ordering MUST be deterministic. Missing or disabled extensions MUST leave recoverable document placeholders where their data cannot be interpreted, never silent deletion.

## Concurrency Implications

UI surfaces observe asynchronous state:

- action availability may change between display and invocation;
- selected objects may be deleted by another view or completed command;
- save may complete for an older document version;
- render preview may lag authoritative state;
- property computation may arrive after target changes.

Therefore:

- actions MUST revalidate at command execution;
- UI projections MUST carry document and object versions;
- stale asynchronous results MUST be discarded;
- disabled states are hints, not security or invariant enforcement;
- selection references MUST use stable IDs, not array indices;
- busy states MUST be scoped to affected object/operation;
- progress cancellation MUST address an operation ID;
- visual optimism MUST reconcile with typed command outcome.

## Failure and Security Implications

Information architecture can prevent destructive mistakes:

- Save, Save As, Save a Copy, and Export MUST have distinct consequences.
- Closing a view MUST not imply discarding a document.
- Applying a mask and deleting a mask MUST be separate.
- Flattening MUST disclose lost editability.
- Color-profile assignment and pixel conversion MUST be separate.
- Context menus MUST resolve target scope before mutation.
- Clipboard paste MUST indicate when content becomes embedded or linked.
- Extension actions MUST identify extension provenance where trust matters.

On failure, UI MUST retain operation context and offer safe next steps. A failed export does not dirty the document unless export settings are document state. A failed rename must preserve old name and focus. A rejected drop must return objects to original structure.

Sensitive paths, metadata, and clipboard content MUST not appear in command search history, diagnostics previews, or accessibility announcements without purpose.

## Workflows

### Create and Save New Document

1. User invokes New from File menu, shortcut, or command search.
2. Parameter surface groups canvas geometry, pixel representation, color, and initial background.
3. Validation runs without creating partial documents.
4. Create command registers one untitled document and initial view.
5. Document identity and modified state are visible.
6. Save resolves local destination through host adapter.
7. Staged write completes; visible state changes to saved only for matching version.

### Select and Edit Layer Mask

1. User navigates layer tree.
2. Layer row exposes attached mask as distinct target.
3. Click selects layer object; activating mask surface changes active edit target.
4. Canvas overlay and status identify mask editing.
5. Brush gesture previews and commits to mask command.
6. History labels operation by mask and tool.
7. Undo restores mask transaction without changing view navigation.

### Reorder Multiple Layers

1. User selects a contiguous or discontiguous set.
2. Drag starts after threshold.
3. Insertion indicator distinguishes before, after, and into group.
4. Status names count, operation, and target.
5. Invalid cyclic or locked target is visibly rejected.
6. Drop submits one reorder command with stable IDs.
7. Selection and focus follow moved objects.

### Export Delivery Copy

1. Export action opens format-neutral destination/options flow.
2. Selected format contribution declares supported dimensions, color, alpha, and metadata.
3. Unsupported document features produce explicit conversion summary.
4. Export runs from stable snapshot without blocking edits.
5. Progress identifies destination and supports cancellation.
6. Completion offers reveal/open actions; document saved state is unchanged.

## Design Rationale and Alternatives
### Stable semantic actions versus widget callbacks

Widget callbacks are easy initially but fragment shortcuts, menus, accessibility, history, and extension access. Stable actions centralize scope and availability while allowing native presentation.

### Persistent panels versus task dialogs

Panels preserve canvas context and support iterative adjustment. Dialogs provide bounded validation but interrupt comparison. Use panels for ongoing object/tool state; use dialogs for document creation, export contracts, and destructive transformations requiring explicit commit.

### One active target versus implicit last-used surface

Implicit targets reduce clicks but cause catastrophic painting into masks or channels. Explicit active surface, clearly indicated in layer tree and status, costs attention but protects intent.

### Context click selects versus preserves selection

Always selecting destroys multi-selection context. Never selecting makes single-object commands ambiguous. Temporary context targeting with explicit resolution preserves both; command tests must enforce it.

### Adaptive UI versus stable geography

Aggressive adaptive rearrangement damages muscle memory and documentation. Responsive presentation MAY collapse regions, but action IDs, menu taxonomy, keyboard access, and conceptual zones remain stable.

## Anti-Patterns

- Treating active document, focused view, and selected object as one global variable.
- Making layer thumbnail click and row click indistinguishable when thumbnail selects edit surface.
- Hidden context-menu-only commands.
- Icons without names for uncommon actions.
- Menus generated from crate/module hierarchy.
- Reusing “Open,” “Apply,” or “Convert” without naming target/result.
- Modal dialogs for every property edit.
- Global busy overlay for document-local work.
- Disabling actions without exposing reason.
- Mutating model during hover or before drag threshold.
- Double-click destructive operations.
- Changing selection on secondary-button press before context resolution.
- Persisting workspace layout inside image document.
- Using row indices as selection identity.
- Treating canvas as inaccessible bitmap.
- Letting extensions inject arbitrary widgets or intercept all input.
- Collapsing assignment and conversion into one color-profile action.
- Presenting export as save.
- Silently flattening unsupported features.
- Hiding save/device-loss/recovery status in transient toast only.

## Best Practices

- Test action semantics headlessly using target fixtures.
- Generate menu, shortcut, context, and command-search presentations from one registry.
- Keep canonical names short; put nuance in descriptions and consequences.
- Preserve focus and selection across model updates by stable ID.
- Include target name and scope in destructive confirmations.
- Make preview state visually and semantically distinct from committed state.
- Use one undo label per meaningful gesture.
- Restore invoking focus after temporary surfaces.
- Keep critical state visible without hover.
- Support reduced motion and high contrast at semantic component level.
- Validate every action again in domain layer.
- Record local interaction traces with redaction for reproducible failures.

## Future Extensibility

Information architecture can extend to:

- multiple simultaneous views and comparison modes;
- detachable or multi-window workspaces;
- specialized channel, animation, or asset panels if product scope expands;
- headless/batch command presentation;
- local scripting through same action registry;
- sandboxed extension panels rendered from semantic component contracts;
- alternate input devices and configurable gesture maps;
- accessibility-oriented structured canvas exploration;
- task-focused workspace presets.

New concepts MUST identify their position in application hierarchy, owner, persistence domain, action scope, accessibility semantics, and failure behavior before adoption.

## Acceptance Tests

### Hierarchy

- Opening two views of one document shows independent zoom and shared edits.
- Closing one view does not prompt to discard while another view remains.
- Workspace rearrangement does not mark document modified.
- Active document and focused view remain distinguishable across windows.

### Actions

- Every non-gesture mutation has a stable action identifier and command mapping.
- Primary menu or command search reaches every named action.
- Toolbar, menu, shortcut, and context presentations produce equivalent semantic commands.
- Action invocation after target deletion fails safely through revalidation.
- Disabled action exposes a reason.

### Selection, Focus, and Context

- Keyboard focus can move without changing object selection.
- Secondary click inside multi-selection preserves multi-object operation scope.
- Secondary click outside selection does not mutate selection before action choice.
- Layer and attached mask show distinct active edit-target states.
- Deleting focused item moves focus predictably.

### Pointer Grammar

- Jitter below drag threshold produces click, not reorder.
- Escape during drag restores original structure.
- Double-click actions are non-destructive and available elsewhere.
- Focus loss or device removal cancels active gesture safely.
- Pen secondary action invokes same semantic context menu as mouse.

### Context Menus

- Layer, mask, resource, document, and canvas contexts include primary and lifecycle actions when applicable.
- Destructive actions name exact consequence.
- Context actions match object capability declarations.
- Keyboard invocation targets focused object.

### Progressive Disclosure

- Collapsing advanced groups preserves values.
- Validation errors inside collapsed groups are visible at group level.
- Default workspace supports open, edit, undo, save, and export without revealing advanced diagnostics.
- Command search discovers hidden panel and tool actions.

### Accessibility

- All interactive elements expose role, name, state, and action.
- Layer tree announces hierarchy, expanded state, selection, and active edit target.
- Keyboard can reach every named action.
- Continuous painting does not flood announcements.
- Save failure and destructive confirmation are announced with appropriate urgency.
- Focus remains visible and valid after dialogs, popovers, deletion, and workspace restoration.

### Concurrency and Failure

- Stale property result cannot overwrite newer target.
- Export from snapshot can finish while later edits remain dirty.
- Failed reorder leaves tree unchanged.
- Device-loss indication does not block save or document inspection.
- Missing extension preserves unknown document content and reports unavailable editor.

## Cross References

- [00 — Introduction and System Charter](00-Introduction.md)
- [Glossary](Appendix/Glossary.md)
- [Requirement Keywords](Appendix/Requirement-Keywords.md)
- [Cross-Reference Index](Appendix/Cross-Reference-Index.md)
- Planned: 02 — Application Lifecycle
- Planned: 03 — Workspace and Window Model
- Planned: 04 — Document Model
- Planned: 06 — Commands and Transactions
- Planned: 08 — Action Registry
- Planned: 18 — Input and Gesture Model
- Planned: 19 — Tool Framework
- Planned: 20 — Canvas and Navigation
- Planned: 21 — Layer and Object Panels
- Planned: 22 — Accessibility
- Planned: 23 — Workspace Persistence
- Planned: 28 — Extension Architecture
