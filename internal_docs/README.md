# PhotoTux Engineering Handbook

## Purpose

This handbook (`internal_docs/`) is the **authoritative Engineering Handbook** for **PhotoTux**, a Linux-native, local-first professional raster editor. It defines product boundaries, subsystem contracts, ownership, concurrency, persistence, accessibility, performance, and verification expectations for core engineers, host/UI engineers, format engineers, and extension authors.

Prior documentation under `/docs/` is archived at [`archive/docs/`](../archive/docs/) (historical ADRs, journals, checklists). Do not treat the archive as normative. Codebase ↔ handbook gaps and alignment plan: [Appendix/Codebase-Handbook-Gap-Analysis.md](Appendix/Codebase-Handbook-Gap-Analysis.md).

Normative force uses **MUST** / **SHOULD** / **MAY** as defined in [Appendix/Requirement-Keywords.md](Appendix/Requirement-Keywords.md). Shared vocabulary lives in [Appendix/Glossary.md](Appendix/Glossary.md). Navigation and dependencies live in [Appendix/Cross-Reference-Index.md](Appendix/Cross-Reference-Index.md).

Start with [00 — Introduction and System Charter](00-Introduction.md).

## Product Boundaries (Summary)

**Included:** local document editing; layers, masks, selections; brushes, filters, color management; text and shapes with explicit rasterize boundaries; commands and undo transactions; GPU-first rendering via wgpu with CPU fallback; import/export adapters; Linux desktop integration; accessibility; crash recovery; extension seams.

**Excluded:** cloud sync/collaboration; accounts and entitlements; AI or generative tools; proprietary vendor workflows as product identity; network requirements for normal editing; frozen UI toolkit or plugin ABI before validation.

## System Principles (Summary)

1. Document owns truth.
2. Commands are the mutation spine.
3. History stores transactions.
4. Rendering reads immutable snapshots.
5. GPU-first, not GPU-only.
6. Concurrency and ownership are explicit.
7. Native quality at Linux host edges.
8. Local capability, least authority.
9. Measure before freezing high-cost choices.
10. Errors remain actionable.

## Reading Order

### Everyone

1. This README
2. [00-Introduction.md](00-Introduction.md)
3. [Appendix/Requirement-Keywords.md](Appendix/Requirement-Keywords.md)
4. [Appendix/Glossary.md](Appendix/Glossary.md)
5. [01-Information-Architecture.md](01-Information-Architecture.md)
6. [Appendix/Decision-Register.md](Appendix/Decision-Register.md)
7. Role path in [Appendix/Cross-Reference-Index.md](Appendix/Cross-Reference-Index.md)

### First deep technical path

[08-Command-System.md](08-Command-System.md) → [10-Document-Model.md](10-Document-Model.md) → [20-History-Undo.md](20-History-Undo.md) → [17-Rendering-Engine.md](17-Rendering-Engine.md)

## Directory Map

```text
internal_docs/
├── README.md                 ← you are here
├── 00-Introduction.md … 32-Developer-Guide.md
└── Appendix/
    ├── Glossary.md
    ├── Requirement-Keywords.md
    ├── Cross-Reference-Index.md
    ├── Subsystem-Dependency-Matrix.md
    ├── Command-Taxonomy.md
    ├── Event-Catalog.md
    ├── Document-Format-Versioning.md
    ├── Error-Taxonomy.md
    ├── Thread-Ownership-Map.md
    ├── Performance-Budget-Ledger.md
    ├── Accessibility-Checklist.md
    ├── Decision-Register.md
    └── Codebase-Handbook-Gap-Analysis.md
```

## Numbered Series (00–32)

| Range | Topics |
| --- | --- |
| 00–01 | Charter, information architecture |
| 02–07, 09 | Lifecycle, workspace, docking, panels, toolbars, context menus, shortcuts |
| 08, 10–21 | Commands, document/layers/selection/masks, brush/filter/color/render, text/shapes, history, clipboard |
| 22–27 | Import/export, plugins, preferences, themes, dialogs, file formats |
| 28–32 | UX, accessibility, performance, testing, developer guide |

Exact filenames are listed in [Appendix/Cross-Reference-Index.md](Appendix/Cross-Reference-Index.md).

## Conventions

- **Filenames:** `NN-Name.md` for specifications; appendices under `Appendix/`.
- **Links:** relative Markdown links with exact filenames; do not invent alternate numbering.
- **Normative language:** uppercase **MUST** / **SHOULD** / **MAY** only when intended ([Requirement Keywords](Appendix/Requirement-Keywords.md)).
- **Vendor neutrality:** describe semantics without proprietary branding or copied vendor workflows.
- **Diagrams:** Mermaid node IDs without spaces; no explicit colors or HTML styling in diagrams.
- **Page-count convention:** treat roughly **500 words ≈ 1 page** when estimating handbook length or review scope. Dense tables and diagrams count toward substance; placeholders are forbidden.
- **Dependencies:** follow [Subsystem Dependency Matrix](Appendix/Subsystem-Dependency-Matrix.md); policy inward, platform outward.
- **Decisions:** high-reversal-cost choices belong in [Decision Register](Appendix/Decision-Register.md).

## Appendices at a Glance

| Appendix | Use when |
| --- | --- |
| [Glossary](Appendix/Glossary.md) | Term ambiguity |
| [Requirement Keywords](Appendix/Requirement-Keywords.md) | Writing or interpreting requirements |
| [Cross-Reference Index](Appendix/Cross-Reference-Index.md) | Finding docs and reading paths |
| [Subsystem Dependency Matrix](Appendix/Subsystem-Dependency-Matrix.md) | Crate/review dependency edges |
| [Command Taxonomy](Appendix/Command-Taxonomy.md) | Naming or classifying commands |
| [Event Catalog](Appendix/Event-Catalog.md) | Notifications and host ingress |
| [Document Format Versioning](Appendix/Document-Format-Versioning.md) | Schema/feature compatibility |
| [Error Taxonomy](Appendix/Error-Taxonomy.md) | Typed failures and retry policy |
| [Thread Ownership Map](Appendix/Thread-Ownership-Map.md) | Concurrency and ownership |
| [Performance Budget Ledger](Appendix/Performance-Budget-Ledger.md) | Latency/memory gates |
| [Accessibility Checklist](Appendix/Accessibility-Checklist.md) | A11y review and release evidence |
| [Decision Register](Appendix/Decision-Register.md) | Architectural decision index |
| [Codebase–Handbook Gap Analysis](Appendix/Codebase-Handbook-Gap-Analysis.md) | Diff vs live crates; alignment plan |

## Related Starting Points by Role

- **Shell/UI:** 01 → 02 → 03 → 05 → 09 → 29
- **Document core:** 10 → 11 → 08 → 20 → 27
- **Renderer:** 17 → 16 → 30 → Thread Ownership Map
- **Formats:** 22 → 27 → Document Format Versioning
- **Plugins:** 23 → 08 → Command Taxonomy → Decision Register (plugin ABI deferred)

## Maintenance

When adding a subsystem requirement, update the owning numbered document first, then appendices that index it (cross-reference, dependency matrix, taxonomy, budgets, checklist, or decision register as applicable). Do not leave TODO/TBD placeholders in shipped handbook pages.
