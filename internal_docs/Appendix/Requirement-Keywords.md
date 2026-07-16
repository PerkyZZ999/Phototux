# Requirement Keywords

## Purpose

This appendix defines normative language used throughout the PhotoTux engineering handbook (`docs/00-Introduction.md` through `docs/32-Developer-Guide.md` and `docs/Appendix/*`). It follows the intent of RFC 2119 and RFC 8174 while remaining self-contained. Uppercase keywords carry normative force only when written as **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, or **OPTIONAL**.

Lowercase forms are ordinary prose. “Must” inside a quotation, example, diagram label, or external interface name is not automatically normative.

Handbook navigation: [docs/README.md](../README.md), [Cross-Reference Index](Cross-Reference-Index.md). Product principles and boundaries: [00 — Introduction](../00-Introduction.md).

## Normative Keywords

### MUST, REQUIRED, SHALL

An absolute requirement. A conforming implementation cannot omit or violate it. If a requirement cannot be met on a supported configuration, implementation MUST:

1. reject the affected operation or configuration safely;
2. preserve document integrity;
3. communicate the limitation;
4. record the incompatibility in conformance evidence.

MUST does not mean “implement immediately.” Planned components can have mandatory requirements before implementation. It means any implementation claiming conformance is bound by them.

### MUST NOT, SHALL NOT

An absolute prohibition. A conforming implementation cannot perform the described behavior. Failure paths, fallbacks, extensions, and platform adapters remain subject to the prohibition unless the requirement explicitly scopes an exception.

### SHOULD, RECOMMENDED

There may be valid reasons to choose another behavior, but the full implications MUST be understood and documented before deviation. A deviation record SHOULD include:

- requirement and document section;
- operational constraint motivating deviation;
- alternatives considered;
- safety, compatibility, performance, and accessibility impact;
- validation evidence;
- owner and review date.

“Hard to implement” alone is not sufficient rationale.

### SHOULD NOT, NOT RECOMMENDED

The behavior is normally harmful. A conforming implementation may use it only when specific circumstances make it preferable and consequences are documented and tested.

### MAY, OPTIONAL

The behavior is genuinely optional. Implementations may include or omit it. Optional capability MUST NOT be silently required by mandatory workflows. Components interacting with an optional capability MUST behave correctly when it is absent, disabled, denied, or unsupported.

## Requirement Construction

Normative requirements SHOULD identify:

- responsible subsystem;
- triggering conditions;
- observable behavior;
- failure behavior;
- scope and exceptions;
- verification method where non-obvious.

Good:

> The save coordinator **MUST** clear the modified indicator only when the persisted document version equals the current authoritative version.

Weak:

> Saving **MUST** work correctly.

Requirements MUST avoid relying on undefined adjectives such as “fast,” “secure,” “intuitive,” “large,” or “soon.” Such terms require measurable thresholds, threat assumptions, acceptance tests, or explicit provisional status.

## Priority and Conflict Resolution

Normative force does not establish implementation priority. Planning assigns priority separately.

When requirements conflict, apply this order:

1. document integrity and user safety;
2. security and least authority;
3. explicit requirements in the narrower subsystem specification (`02`–`32` as applicable);
4. requirements in earlier foundation documents (`00`, `01`, and accepted [Decision Register](Decision-Register.md) entries);
5. recommendations and optional behavior.

Narrower documents may refine broader requirements but MUST NOT silently contradict them. A contradiction MUST be resolved through an explicit [Decision Register](Decision-Register.md) entry and updates to the [Cross-Reference Index](Cross-Reference-Index.md) and [Subsystem Dependency Matrix](Subsystem-Dependency-Matrix.md) when navigation or edges change.

Dependency direction constraints in the Subsystem Dependency Matrix are normative for crate and review boundaries even when phrased without repeating every MUST from source documents.

## Scope

A requirement applies to the section’s subject and inherited parent scope. Requirements in examples do not automatically apply outside the example. Requirements qualified by “during export,” “on Linux,” or “for extensions” do not imply behavior in other scopes.

Platform limitations do not erase requirements. A host unable to provide a capability MUST expose unsupported status or disable dependent operations safely.

## Conformance

A conforming component:

- satisfies every applicable MUST and MUST NOT;
- documents justified deviations from SHOULD and SHOULD NOT;
- declares implemented MAY capabilities;
- supplies tests, measurements, review evidence, or inspected contracts appropriate to each requirement.

Conformance claims SHOULD name document revision or commit. Partial conformance MUST name excluded sections and user-visible consequences.

## Provisional Targets

Targets marked provisional (including many entries in [30 — Performance](../30-Performance.md) and the [Performance Budget Ledger](Performance-Budget-Ledger.md)) are normative design constraints for measurement and engineering. They become unconditional product promises only after a [Decision Register](Decision-Register.md) entry promotes them. Before promotion, teams MUST measure against them and MUST document fixture and hardware context. Provisional does not mean ignorable; it means evidence can revise a threshold without treating revision as automatic product regression.

Deferred items (for example UI toolkit choice and stable plugin ABI in the Decision Register) MUST NOT be treated as silently chosen. Seams and capability policies in [23 — Plugin SDK](../23-Plugin-SDK.md) and presentation contracts remain binding even while ABI bytes are deferred.

## Examples and Notes

“For example,” “such as,” rationale paragraphs, alternatives, diagrams, and notes are informative unless they contain uppercase normative keywords. Acceptance criteria are normative when expressed with these keywords; otherwise they define expected verification evidence.

## Usage Rules

- Use MUST for interoperability, integrity, safety, security, ownership, and invariant boundaries.
- Use SHOULD for strong defaults with plausible environment-specific exceptions.
- Use MAY for extension points and optional user-facing capability.
- Do not use MUST to express preference.
- Do not weaken a requirement by pairing MUST with “where practical.”
- State a bounded exception instead.
- Avoid multiple unrelated obligations in one sentence.
- Link specialized terms to the [Glossary](Glossary.md) when ambiguity is likely.

## Suite Application Notes

- Shell documents (`02`–`07`, `09`, `24`–`26`, `28`) specify presentation and host-facing behavior; they MUST NOT authorize bypassing [08 — Command System](../08-Command-System.md) for semantic mutation.
- Domain documents (`10`–`16`, `18`–`21`) specify authoritative semantics; render caches remain non-authoritative per [17 — Rendering Engine](../17-Rendering-Engine.md).
- I/O and trust documents (`22`, `23`, `27`) treat external bytes and extensions as hostile or least-authority by default.
- Quality documents (`29`–`32`) define verification and accessibility obligations that apply across the suite.
- Appendices index and refine; on conflict of detail, the owning numbered specification wins after Decision Register resolution.

## Cross References

- [docs/README.md](../README.md)
- [00 — Introduction and System Charter](../00-Introduction.md)
- [01 — Information Architecture](../01-Information-Architecture.md)
- [08 — Command System](../08-Command-System.md)
- [Glossary](Glossary.md)
- [Cross-Reference Index](Cross-Reference-Index.md)
- [Decision Register](Decision-Register.md)
- [Subsystem Dependency Matrix](Subsystem-Dependency-Matrix.md)
