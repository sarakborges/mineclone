# Asteria Engineering Practices

This document is the project-wide, technology-agnostic engineering standard for Asteria. `ARCHITECTURE.md` remains authoritative for Asteria-specific domain contracts; this file defines the reusable software-engineering rules used to implement and refactor those contracts.

The core principle is simple:

> Prefer code with one clear owner, one clear responsibility, explicit dependencies, explicit mutation, explicit lifecycle, explicit failure behavior, reusable domain primitives, measurable performance characteristics, deterministic behavior where required, and small testable boundaries. Architecture should make correct code easier to write than incorrect code.

## 1. Responsibility, ownership, and boundaries

- A module, type, function, component, service, or subsystem should have one primary reason to change. Split by responsibility, never by arbitrary file size.
- Every mutable gameplay/business fact has one authoritative owner. Derived views may be cached only when their invalidation and lifetime are explicit.
- Keep domain, orchestration, persistence, serialization, rendering/UI, external integration, and background work behind explicit boundaries.
- Dependency direction must be intentional. Stable domain concepts must not depend on volatile framework, storage, UI, transport, logging, or platform details unless the domain itself requires them.
- Prefer composition over god objects. Contexts/SystemParams must be purpose-specific and cohesive rather than bags of unrelated resources.
- Reuse real invariants, state machines, queue mechanics, validation rules, caches, policies, and visual primitives. Do not abstract code merely because two call sites look similar.
- Keep public surfaces small. Default to private/internal visibility and expose capabilities rather than storage layout.
- Keep related invariant + state + transitions + validation + cache invalidation close together.

## 2. Modeling and mutation

- Encapsulate mutable collections and runtime state. Mutation belongs where the invariant is enforced.
- Separate immutable/authored definitions from mutable runtime instances.
- Use strong domain types when they prevent accidental mixing or centralize validation; do not wrap primitives without benefit.
- Validate external/authored input at subsystem boundaries and near the owner of the invariant.
- Make invalid states hard to represent with enums/tagged unions, validated constructors, explicit state types, and state-machine transitions.
- Lifecycle behavior should use explicit state machines instead of unrelated booleans and distributed conditionals.
- Meaningful state changes should flow through intent-revealing operations/commands rather than arbitrary field mutation.
- Read models may be optimized for consumers, but never become an independent mutation owner.
- Use stable identifiers across system, persistence, async, cache, and serialization boundaries rather than object identity.
- Do not update/publish a value if it did not change when change propagation is expensive.
- Track monotonically increasing revisions when they simplify cache invalidation, UI refresh, stale-result rejection, snapshots, or optimistic concurrency. Increment only on meaningful authoritative change.

## 3. APIs and abstraction

- Keep abstractions narrow: small APIs, narrow return values, explicit parameters, purpose-specific contexts.
- Avoid Manager/Context/Utils/Service dumping grounds unless the broad name genuinely corresponds to one responsibility.
- Avoid boolean-flag APIs when separate operations, enums, or option structures express intent more clearly.
- Design APIs around capabilities (what callers need to do), not internal maps/lists/tables.
- Prefer explicit mapping between domain, storage, API, UI, and serialized models when their contracts differ.
- Keep serialization shape outside core domain semantics when mirroring the wire/file format would distort the model.
- Add interfaces/traits only when they protect a meaningful boundary or multiple relevant implementations exist.
- Avoid premature generalization. Prefer concrete implementation -> observed duplication -> extracted invariant -> stable abstraction.
- Prefer composition to deep inheritance/subtyping hierarchies.
- Keep orchestration thin: boundary validation, obtain inputs, invoke domain operation, persist/publish, map outputs.

## 4. Side effects, errors, and failure

- Separate pure calculation/validation/transformation/policy from I/O, logging, persistence, UI mutation, networking, and engine effects where practical.
- Make side effects visible in names and APIs. Query-looking functions must not hide writes or I/O.
- Separate policy ("what should happen") from mechanism ("how it happens").
- Errors should distinguish invalid input, expected rejection, unavailable dependency, timeout, corruption, and broken programmer invariant.
- Fail fast near impossible internal invariant violations; return normal errors for user/external invalid input.
- Never silently swallow failures. If ignoring an error is intentional, encode and explain that decision.
- Treat files, APIs, plugins, messages, database rows, and user data as untrusted boundaries.
- Preserve correctness under partial failure with atomic publication, transactions, idempotency, recovery states, or compensating actions where appropriate.
- Keep transactions small and aligned with the invariant that must change atomically.
- Setup and cleanup should mirror each other. Resource lifetime and ownership must be explicit.

## 5. Concurrency and asynchronous work

- Treat concurrency as an ownership problem. Prefer task-local state, immutable snapshots, message passing, and bounded synchronized ownership.
- Do not introduce concurrency without a measured/necessary reason.
- Every async result whose inputs can change must carry identity such as a revision, generation, sequence, or request ID; stale results must be rejected.
- Bound tasks, queues, worker counts, and memory. Use backpressure, priorities, preemption, or cancellation when justified.
- Long-running work that can become irrelevant should support cancellation without leaving invalid state.
- Immutable snapshots are preferred for shared/background work.
- Event-driven flow must define publisher, delivery timing, ordering, idempotency/retry, failure behavior, and mutation ownership. Do not use events merely to hide dependencies.

## 6. Change-driven work and caching

- Prefer change-driven work over repeated frame/tick/request recomputation.
- Every cache must define: key, value, owner, validity rule, invalidation, lifetime, memory bound, and why caching is cheaper than recomputation.
- Derived metadata belongs near the owner of its source.
- Precompute immutable/rarely changing relationships: lookup tables, parsed rules, normalized IDs, dependency graphs, bounds, indexes, capability flags, compiled forms.
- If definitions can replace an existing ID, rebuild derived metadata from authoritative definitions instead of keeping monotonic/stale summaries.
- Centralize expensive lookup/index rules rather than letting consumers rediscover them.
- Use specialized data structures when they encode real semantics: deduplicating queues, priority queues, ring buffers, LRU caches, spatial indexes, stable ordered sets, interval structures.
- Test specialized collections independently.
- Every growing cache/queue/history/task registry must have a bounded lifecycle or an explicit reason unlimited growth is safe.

## 7. Determinism and time

- If output depends on order, sort explicitly and define stable tie-breakers. Never rely on hash-map/set, filesystem, or concurrent completion order.
- Randomness that affects behavior must use explicit deterministic seeds/sources when reproducibility matters.
- Separate simulation/logical time from wall-clock time. Gameplay transitions should use ticks/turns/sequence time unless realtime is itself the rule.
- Prefer explicit/injectable clock abstractions when wall-clock time controls domain behavior.
- Keep tests independent of uncontrolled wall clock, random global state, filesystem ordering, network, execution order, and race timing.

## 8. Performance and realtime work

- Correctness and clarity come first; optimize from evidence.
- Measure the hot path, reduce unnecessary work before micro-optimizing, and verify improvement after the change.
- Optimize the subsystem that owns the cost rather than hiding it through UI tricks or duplicated caches elsewhere.
- Avoid unnecessary allocation/copy/serialization/string churn in measured hot paths.
- Keep hot-path dependencies small and data-oriented.
- Prefer compact/contiguous representations and IDs when large/high-frequency workloads justify them.
- Batch repeated expensive boundary calls (disk, network, GPU) when latency/error semantics allow it.
- Use work budgets for latency-sensitive loops. Logical outcomes must not depend on machine speed unless the domain explicitly requires realtime timing.
- Prefer incremental processing for large workloads; use checkpoints/revisions where resumability matters.
- Avoid unnecessary synchronization and logging in extremely hot paths; use counters, aggregation, sampling, or diagnostic modes.
- Logging and metrics are observational and must not become correctness state.

## 9. Function and code clarity

- Prefer small cohesive functions that operate at one abstraction level and represent a named concept.
- Do not split merely to reduce line count.
- Avoid deep nesting; use guard clauses, early returns, named operations, and explicit transitions.
- Name by domain intent rather than vague Process/Handle/Helper/Manager terminology when a more precise concept exists.
- Replace meaningful magic values with named constants/domain types and document units.
- Behavioral configuration should have an owner, defaults, validation, units, and bounds.
- Prefer readable code over compact cleverness. Language tricks that reduce readability require a concrete payoff.
- Avoid hidden control flow, reflection/magic hooks, ambient service location, global mutable state, and excessive static runtime state.

## 10. Testing

- Test invariants and behavior, not private implementation structure.
- For automatically reproducible bugs, add a regression test before/with the fix and keep it.
- Keep core tests fast and framework-light when practical.
- Use integration tests at actual boundaries: filesystem, serialization, database, network, engine adapters.
- Use property-based testing when general invariants (roundtrips, ordering, uniqueness, algebraic/state-machine properties) matter more than a few examples.
- Keep fixtures focused and small.
- Tests for caches/queues/schedulers/state machines should directly validate uniqueness, ordering, invalidation, state transitions, and failure semantics.

## 11. Refactoring triggers

Refactor when any of these become true:

- one module owns multiple unrelated invariants;
- one file repeatedly changes for unrelated features;
- multiple components can mutate the same fact;
- a method/SystemParam/context requires too many unrelated dependencies;
- consumers bypass the intended mutation boundary;
- business/game rules are copy-pasted;
- strings/integers have become an implicit protocol;
- lifecycle is represented by scattered booleans/conditionals;
- consumers independently reimplement the same cache/index;
- async tasks can apply stale results;
- queues/caches/memory grow without bounds;
- expensive work runs despite unchanged inputs;
- tests require excessive unrelated setup;
- framework/infrastructure leaks into core domain logic;
- a helper needs many flags/exceptions;
- optimization requires cross-layer hacks because cost ownership is unclear.

When parameters grow, create a context only if those dependencies are genuinely cohesive. Never solve parameter count by creating an everything-context.

## 12. Review checklist

Before accepting a change, verify:

1. What invariant does each new component own?
2. Is there exactly one mutable owner for every fact?
3. Are responsibilities cohesive and dependencies narrow?
4. Are domain and infrastructure/framework boundaries explicit?
5. Is the abstraction based on a real shared concept?
6. Is invalid input rejected at the correct boundary?
7. Are invalid state combinations structurally prevented where practical?
8. Are mutation paths and lifecycle transitions explicit?
9. Are side effects obvious?
10. Can async results become stale, and if so are they revision-checked?
11. Are concurrency and queues bounded?
12. Does every cache have key/validity/invalidation/lifetime/memory rules?
13. Can repeated work become change-driven or precomputed?
14. Are ordering/randomness deterministic where correctness depends on them?
15. Is hot-path allocation/copy/synchronization justified by measurement?
16. Can any collection grow without a retention strategy?
17. Is the public surface no larger than needed?
18. Are failures meaningful and non-silent?
19. Do tests protect behavior/invariants rather than code layout?
20. Is the resulting code clearer and easier to evolve than before?
