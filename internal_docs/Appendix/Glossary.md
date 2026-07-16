# Glossary

## Purpose

Canonical, vendor-neutral vocabulary for PhotoTux product and engineering specifications. Definitions describe semantic intent, not a final Rust API or UI toolkit. Normative force comes from numbered specifications and [Requirement Keywords](Requirement-Keywords.md).

## Terms

### Accessibility tree

Structured representation of roles, names, states, values, hierarchy, and actions exposed to assistive technology. It is not inferred solely from rendered pixels.

### Action

Named semantic operation exposed through one or more presentations such as menu item, shortcut, toolbar control, command search, panel control, or context menu. An action resolves scope and parameters, then invokes a command or view-only behavior.

### Action availability

Current determination that an action can be invoked for a resolved scope and target. Availability in UI is advisory; command execution revalidates invariants.

### Active document

Document associated with the focused work context. It may differ from documents visible in other views or windows.

### Active edit target

Specific editable surface receiving continuous tool output, such as layer pixels or an attached mask. It is distinct from object selection and keyboard focus.

### Alpha

Per-pixel coverage or opacity component. Every buffer and operation using alpha must define whether values are straight or premultiplied and how zero-alpha color is handled.

### Application session

Process-lifetime coordination context containing open documents, windows, workspaces, preferences, global resources, commands, and host integration.

### Authoritative state

State whose version determines document truth. For editable content this belongs to the document model, never a panel, view, render cache, or GPU resource.

### Backpressure

Policy that limits work production when consumers, memory, or devices cannot keep up. Bounded queues, coalescing, cancellation, and priority shedding are forms of backpressure.

### Blend mode

Defined operation combining source and destination color/alpha values. Semantics include color space, precision, alpha convention, clamping, and edge behavior.

### Brush

Resource and behavior configuration used by a painting tool. Brush parameters may include shape, spacing, dynamics, texture, blending, and sampling. A brush is not itself a document mutation.

### Cache

Reconstructible derived data retained for performance. Cache eviction must not lose authoritative unsaved document state.

### Canvas

Conceptual image plane of a document, including its coordinate system and extent. “Canvas” does not mean a UI widget.

### Canvas view

Viewport projection of a document with navigation and display state such as zoom, pan, rotation, overlays, and proofing. Multiple views may reference one document.

### Capability

Explicit grant of authority to access a resource or invoke a class of operation. Extensions and adapters receive capabilities instead of ambient authority.

### Channel

Single scalar component or logical plane, such as red, alpha, mask coverage, or saved selection data. Channel meaning, precision, and color interpretation must be explicit.

### Checkpoint

Materialized document state retained to accelerate history traversal, save, or recovery. A checkpoint does not replace transaction semantics.

### Codec

Importer or exporter component responsible for a file representation. Codecs process untrusted input and operate under allocation, recursion, time, and capability limits.

### Color profile

Data describing interpretation or conversion of color values. Assigning a profile changes interpretation; converting a profile changes pixel values. These are distinct operations.

### Command

Semantic, validated request to inspect or mutate application or document state. Mutating commands produce transactions or typed failures.

### Command merge

Policy combining related command transactions into one history step, commonly for continuous gestures. Merge must preserve deterministic undo boundaries.

### Command router

Subsystem resolving command target, validating authority and preconditions, scheduling execution, and returning structured results.

### Compositing

Evaluation of ordered layers, masks, effects, blending, color operations, and visibility into an output image or intermediate surface.

### Context target

Object under secondary-click or keyboard context-menu invocation. It may differ from selected objects and focused control.

### Core

Portable semantic implementation of documents, commands, history, snapshots, and related policies. Host-specific UI and desktop APIs remain outside core boundaries.

### Delta

Bounded, versioned description of changes between document states. Deltas may contain semantic object changes, spatial dirtiness, resource changes, or invalidation hints.

### Destructive operation

Operation that discards editability, source information, or reversible structure beyond configured history/recovery guarantees. Destructive operations require precise naming and explicit consequences.

### Dirty region

Object, tile, or spatial scope requiring recomputation following a transaction or view change.

### Document

Unit of editable persistence containing authoritative object graph, raster resources, color metadata, selections, and history association.

### Document identity

Stable internal identifier for an open document. It is independent of file path and display name.

### Document version

Monotonic identifier of authoritative state evolution. Undo and redo create new versions rather than rewinding version numbers.

### Edit surface

Raster or logical target to which a painting or transformation operation applies, such as layer pixels or a mask.

### Effect

Parameterized operation evaluated as part of document compositing or explicit filtering. Effects may be nondestructive nodes or destructive command results.

### Export

Creation of a delivery representation from a stable document snapshot. Export does not establish the editable document’s saved identity unless a specification explicitly says so.

### Extension

Optional locally installed contribution providing declared actions, codecs, filters, tools, resources, or semantic UI. Extension trust, capabilities, compatibility, and lifecycle are explicit.

### Focus

Single locus receiving keyboard input within an active window. Focus does not imply object selection or active edit target.

### Gesture

Bounded sequence of input events interpreted by a tool or control, with explicit begin, preview, commit, and cancel semantics.

### GPU-first

Architecture prioritizing GPU rendering and suitable compute while preserving CPU reference, fallback, or non-GPU paths where correctness and compatibility require them.

### History

Ordered representation of committed undoable transactions and optional checkpoints. It is document state evolution, not a list of raw UI events.

### Host adapter

Platform-specific implementation connecting portable contracts to native windows, surfaces, input, dialogs, portals, clipboard, accessibility, lifecycle, and desktop services.

### Immutable snapshot

Read-only versioned view of document state safe for concurrent consumers. Immutability is semantic: internal sharing or lazy materialization may occur without exposing mutation.

### Information scent

Cues communicating what an object is, its state, likely actions, scope, and where additional capability can be found.

### Intent

Normalized semantic interpretation of input before domain mutation, such as “pan view,” “activate mask surface,” or “set layer opacity.”

### Layer

Ordered document object contributing pixels or generated content to compositing. Layer kinds, children, masks, effects, blending, visibility, and locks are explicit.

### Layer tree

Hierarchical ordered graph of layers and groups. It defines compositing structure and object navigation.

### Local-first

Product property where normal editing, persistence, recovery, resources, and extensions operate on the local system without accounts or required network services.

### Mask

Scalar coverage object attached to a layer, group, effect, or operation to limit contribution. A mask is a distinct editable target.

### Modified state

Condition that current authoritative document version differs from its persisted editable version. Export completion does not clear modified state.

### Native host

Application layer integrating with platform conventions and services. Linux-native refers to behavior and integration quality, not mandatory commitment to one toolkit.

### Nondestructive operation

Operation retaining source information and editable parameters so result can be changed or removed without reconstructing lost data.

### Object ID

Stable identifier for a document object. IDs are not positional indices and are not reused during a document lifetime.

### Operation ID

Identifier for asynchronous work such as save, export, import, or filter evaluation, used for progress, cancellation, and diagnostics.

### Panel

Workspace region presenting structure, properties, resources, history, tasks, or diagnostics. A panel declares what application, document, view, selection, or pinned target it follows.

### Persistence

Encoding and durable writing of editable document state, preferences, workspace state, or recovery data. Each persistence domain has separate ownership and compatibility policy.

### Pixel selection

Document-associated coverage field defining affected image regions. It is distinct from selected layer objects.

### Presentation

Concrete UI exposure of semantic actions and state. Presentations may vary by toolkit or host without changing action identity.

### Preview

Transient result shown before command commitment. Preview is cancelable, version-bound, and not authoritative document state.

### Progressive disclosure

Organization that exposes essential controls immediately and reveals specialized detail on demand without creating inconsistent modes.

### Recovery

Process and data enabling restoration after interruption or failure. Recovery data supplements but does not replace explicit save.

### Render graph

Dependency representation resolving document snapshot, view parameters, tiles, effects, color transforms, and compositing into executable render work.

### Renderer

Subsystem producing visual output from immutable snapshots and view state. It owns derived GPU resources and caches, not document truth.

### Resource

Reusable local asset or configuration such as brush, gradient, pattern, palette, font reference, or profile. Resources may be embedded, referenced, or application-global under explicit policy.

### Save

Durable persistence of editable document representation. Save operates on a stable version and clears modified state only if persisted and current versions match.

### Secondary click

Input intent requesting contextual actions, usually from a secondary pointer button, pen barrel control, or keyboard context key.

### Selection

Set or coverage identifying targets for commands. Object selection, pixel selection, focus, context target, and active edit target are distinct concepts.

### Snapshot publisher

Subsystem making versioned immutable document views and deltas available to rendering, save, export, and analysis consumers.

### Staged write

Persistence technique writing a complete new representation before replacing the destination, reducing risk of corruption on interruption.

### Surface

Depending on context, a native presentation target or editable raster target. Specifications should qualify “window surface,” “GPU surface,” or “edit surface” to avoid ambiguity.

### Tile

Bounded rectangular unit for raster storage, processing, invalidation, transfer, or caching. Tile dimensions and border policy are deferred architecture decisions.

### Tool

Interaction state machine translating input gestures, parameters, target context, and modifiers into transient previews and semantic commands.

### Transaction

Atomic validated state transition resulting from a mutating command. It records sufficient reversible information, affected objects/regions, history metadata, and invalidation.

### Trust boundary

Interface across which data or behavior changes trust level and requires validation, capability checks, limits, or isolation.

### Undo

Application of a committed transaction’s inverse under current invariants, producing a new monotonic document version.

### View state

Non-document presentation state for a canvas view, including navigation and overlays. View state normally does not mark document modified.

### wgpu

Rust GPU abstraction selected as PhotoTux’s primary rendering and compute interface. Its use does not imply that every operation must run on GPU.

### Workspace

Arrangement and state of canvas views, panels, tools, and status presentation for a task. Workspace state is persisted separately from editable documents by default.

### Workspace preset

Named reusable subset of workspace layout and presentation preferences. It does not contain authoritative document content.

### Active tool

Currently selected interaction tool whose gesture machine receives primary pointer input for a view. Distinct from focused control and from active edit target.

### Adjustment layer

Nondestructive layer kind that parameterizes an effect evaluated during compositing rather than permanently rewriting underlying pixels until explicitly applied or rasterized.

### AT-SPI

Assistive Technology Service Provider Interface used on Linux desktops. The host accessibility adapter maps PhotoTux semantic nodes to AT-SPI; core owns semantic identity.

### Autosave

Automatic persistence of recovery-oriented state. Autosave MUST NOT be presented as a user-confirmed save of the editable document.

### Behavior version

Version identifying observable evaluation rules (blend, color, filter, text shaping, and similar). Distinct from syntactic schema version and from container generation.

### Brush engine

Subsystem that samples input, plans dabs, applies dynamics, and prepares raster writes committed through commands and transactions.

### Chunk

Bounded persistence unit in the native document container carrying a kind, schema version, integrity code, and payload. Corruption and streaming localize at chunk boundaries.

### Clipboard payload

Transfer representation moving through internal schemas and/or host MIME types. Host-facing payloads are untrusted and validated like import data.

### Coalescing

History policy merging consecutive mergeable transactions (commonly stroke segments) into one undo step without changing semantic ordering guarantees.

### Color management

Subsystem defining color spaces, profiles, assign versus convert operations, proofing, precision, and transforms for display and export.

### Command ID

Stable namespaced identifier for a command (for example `layer.set-opacity`). Independent of menu path, shortcut, toolkit widget, or localized label.

### Command taxonomy

Classification of commands by scope, mutation class, execution class, undo policy, and related axes. See [Command Taxonomy](Command-Taxonomy.md).

### Container generation

Identifier of one committed manifest set inside a native document file. Distinct from runtime document version.

### Context menu

Presentation of actions for a context target, usually from secondary click or keyboard context key. Completeness is measured against the action model, not against a vendor product.

### Destructive disclosure

User-visible explanation that an operation discards editability or exceeds ordinary undo/recovery guarantees, required before committing such commands.

### Dialog

Task or modal interaction surface with explicit focus entry/exit, default and destructive action rules, and optional host portal participation.

### Docking

Layout topology of splits, stacks, and regions that place panels and views. Docking owns geometry; panels own semantic content.

### Document format

Native editable persistence container and its schemas, features, and migration rules. Interchange formats are adapters, not native stores.

### Edit target announcement

Accessibility and UI communication of whether continuous editing applies to layer pixels, a mask, or another declared surface.

### Event envelope

Typed notification carrying family, sequence, scope generation, correlation, and bounded payload. Events report; they do not replace commands.

### Export profile

Declared settings and capability expectations for a delivery export. Export success does not clear document modified state.

### Feature ID

Stable identifier for a semantic capability declared in native manifests (core or extension-namespaced). Readers must support required features or refuse unsafe interpretation.

### Filter engine

Subsystem registering filter/effect descriptors, executing CPU or GPU paths, producing previews, and committing through commands.

### Follow target

Panel binding that tracks active document, view, selection, or pinned object according to its descriptor.

### History timeline

Ordered structure of committed transactions and optional checkpoints for a document.

### Host portal

Desktop-mediated capability grant (for example file selection) that returns opaque handles rather than ambient filesystem authority.

### Import adapter

Codec path that decodes untrusted external bytes into a validated document or layer placement under limits.

### Information architecture

Organization of objects, actions, navigation, and information scent in the product. Specified in [01 — Information Architecture](../01-Information-Architecture.md).

### Invariant failure

Error class indicating impossible or corrupted semantic state. Affected mutation authority freezes; speculative repair is forbidden.

### Job

Bounded asynchronous operation managed by the command system, with progress, cancellation, and versioned applicability for results.

### Layer kind

Discriminated type of layer contribution such as raster, adjustment, fill, text, shape, or reference, each with explicit compositing rules.

### Least authority

Design rule that code paths receive only capabilities required for the operation, especially for files, clipboard, and extensions.

### Manifest

Root descriptive structure of a native document generation listing schemas, features, and references to required chunks.

### Mergeable command

Command whose successive transactions may coalesce under history policy, typically continuous gestures.

### Modified indicator

UI reflection that authoritative document version differs from persisted editable identity.

### Native container

PhotoTux editable file representation: chunked, versioned, integrity-checked, independent from Rust memory layout.

### Panel descriptor

Declarative registration of a panel’s identity, placement defaults, follow/pin behavior, and accessibility metadata.

### Pin target

Explicit panel binding to a chosen document object or view that does not automatically follow global activation.

### Plugin / extension host

Future isolation boundary mediating contributions (commands, filters, formats, tools, panels) under capabilities and budgets. Stable binary ABI remains deferred.

### Preference domain

Persisted settings schema separate from documents and workspaces, with its own migration rules.

### Progressive render

Explicit contract allowing incomplete visual refinement of one document version. Must not silently mix incompatible versions.

### Proofing

Display simulation of a target color condition without necessarily converting the document’s working pixels permanently.

### Rasterize

Command that converts non-raster or higher-structure content into pixels, often destructive to prior editability and requiring disclosure.

### Recovery candidate

Local recovery artifact offered after interruption. Never labeled as a confirmed user save.

### Render coordinator

Role that schedules GPU/CPU render work from immutable snapshots and view state under backpressure.

### Resource pressure

Condition where memory, disk, GPU, or job budgets constrain work. Truth is preserved; caches and quality may shed.

### Schema version

Version of a persisted or exchanged structural contract (manifest, chunk, command, preference, and similar). Evolves independently from Rust types.

### Shape engine

Subsystem for vector-like path/shape objects with deterministic local evaluation and explicit rasterization boundaries.

### Shortcut binding

Mapping from key sequence to an action ID, subject to conflict resolution and IME/text-yield rules.

### Snapshot lease

Temporary right to retain an immutable snapshot for save, export, or analysis without blocking unrelated mutations of newer versions.

### Staged write

Persistence technique that completes a new representation before replacing the destination.

### Text engine

Subsystem for text objects, shaping, and editability with deterministic local fonts/resources and explicit rasterize boundaries.

### Theme tokens

Named presentation variables for color, contrast, spacing, and motion that adapt to host contrast and reduced-motion preferences.

### Thread role

Logical executor class (UI, document executor, render coordinator, workers, I/O, extensions) defining ownership. Not necessarily one OS thread.

### Toolbar

Presentation strip for tools and frequent actions. Emits actions/commands; does not own document truth.

### Transaction group

Explicit begin/end grouping of multiple transactions into one undo presentation unit.

### Unavailable contribution

Extension-provided object, filter, panel, or format that cannot run but whose opaque document data may still be preserved.

### View ID

Stable identifier for a canvas view instance projecting a document.

### wgpu device loss

Condition where the GPU device becomes invalid. Document authoritative state remains; renderer reconstructs under a new generation.

### Workspace topology

Committed docking and region arrangement used for restore and accessibility reading order.

## Cross References

- [00 — Introduction and System Charter](../00-Introduction.md)
- [01 — Information Architecture](../01-Information-Architecture.md)
- [08 — Command System](../08-Command-System.md)
- [10 — Document Model](../10-Document-Model.md)
- [17 — Rendering Engine](../17-Rendering-Engine.md)
- [23 — Plugin SDK](../23-Plugin-SDK.md)
- [27 — File Formats](../27-File-Formats.md)
- [29 — Accessibility](../29-Accessibility.md)
- [30 — Performance](../30-Performance.md)
- [Requirement Keywords](Requirement-Keywords.md)
- [Cross-Reference Index](Cross-Reference-Index.md)
- [Command Taxonomy](Command-Taxonomy.md)
- [Decision Register](Decision-Register.md)
