# Asteria Agent Rules

These rules are mandatory for every implementation, refactor, review, and planning task in this repository.

## Mandatory reading

Before changing code, read and follow:

- `ARCHITECTURE.md`
- `ENGINEERING_PRACTICES.md`

These documents are jointly normative.

Use `docs/README.md` as the documentation map. `HANDOFF.md` is the only active handoff; files under `docs/archive/` and `docs/handoffs/` are historical context unless the current task explicitly points to them.

## Active branch and Git discipline

> **THE ACTIVE IMPLEMENTATION BRANCH IS THE REPOSITORY'S CURRENT DEFAULT BRANCH (`main` AT THE TIME OF WRITING). DO NOT WORK ON `develop` UNLESS THE USER EXPLICITLY REQUESTS IT.**

Branch identity is part of correctness. Never rely on an ambient checkout, cached state, repository history from a previous task, conversation context, memory, a stale handoff, or an omitted `ref`/`branch` argument.

### Mandatory repository bootstrap

Before any repository discovery, content/code search, file read, or write:

- If the user explicitly names a branch for the current task, that branch is authoritative.
- Otherwise, query repository metadata first and resolve the repository's current `default_branch`.
- Fetch the root `AGENTS.md` from that resolved branch before continuing repository work.
- Never infer the branch from previous conversations, previous tasks, recent commits, cached tool state, remembered project context, or an old instruction that named a branch.
- Never reuse a branch from an earlier task unless the current user instruction explicitly carries that branch forward.
- If remembered context conflicts with live repository metadata or root `AGENTS.md`, live repository state wins.

For every task that reads or writes repository state:

- Resolve the target branch before discovery or edits. When the user does not specify another branch, use the live repository `default_branch` resolved during bootstrap.
- Pass the target branch explicitly to every GitHub/file operation that accepts `ref` or `branch`.
- Before editing, read the remote target-branch HEAD and treat that SHA as the task base.
- Before publishing, verify the target branch has not moved unexpectedly. If it moved, reconcile with the new HEAD instead of overwriting concurrent work.
- Never force-update the target branch for normal implementation work.
- Never treat a local commit, temporary branch, generated patch, or successful tool response as proof that the change reached the target remote branch.
- After publication, re-read the remote target branch and verify that the expected commit is reachable there.

`develop` is now a legacy integration branch. It may be read for historical comparison, but implementation must not silently drift back to it.

## User instruction = delivery contract

Treat the complete user instruction as the acceptance contract for the task.

Before editing, derive a small internal checklist of every requested behavior, asset, file-level effect, and explicit constraint. Before publishing, check every item against the final diff/state.

- Do not stop after fixing the first visible symptom when the instruction contains multiple requirements.
- Do not quietly drop a requirement because its implementation lives in another file or subsystem.
- Do not broaden the task with unrelated cleanup merely because nearby problems were discovered.
- One user instruction is one delivery unit unless the user explicitly asks to deliver it piece by piece.
- Multiple technical commits are allowed when useful, but every commit required for the instruction must reach the target remote branch before the task may be reported complete.
- Prefer coherent commits that describe delivered behavior, not a trail of partial attempts.

## Repository navigation discipline

> **DO NOT WALK THE ENTIRE REPOSITORY TREE OR START WITH BROAD, UNBOUNDED SEARCHES. THEY DO NOT WORK RELIABLY IN THIS PROJECT.**

Repository discovery must be scoped and incremental. Full-tree traversal, recursive directory dumps, repository-wide `find`, and broad `grep`/search queries create too much noise, frequently time out or truncate, and are not a valid default discovery strategy.

When locating code or assets:

- Start from the most likely subsystem, directory, filename, symbol, asset id, error string, or documentation pointer.
- Prefer direct file reads and narrowly scoped searches over recursive exploration.
- Use `docs/README.md`, `ARCHITECTURE.md`, `ENGINEERING_PRACTICES.md`, and known module paths to choose the first search scope.
- Search for exact or distinctive identifiers first: type names, function names, config keys, asset ids, log messages, or filenames.
- Expand the scope one level at a time only when the targeted search fails.
- If the location is unknown, use a small number of specific search terms to discover candidate files; inspect those candidates instead of enumerating the repository.
- Never use an exhaustive tree walk merely to "understand the project" before beginning work. Build context from the relevant paths outward.
- For branch-sensitive reads, prefer direct fetches from the explicit target branch. Do not assume generic code-search results prove the current contents of that branch.
- If a tool response is truncated, treat it as partial evidence. Do not infer that omitted files, lines, commits, or results do not exist.

Treat a broad repository scan as a failure mode to avoid, not as a fallback strategy.

## Anti-loop tool discipline

> **DO NOT GET STUCK QUERYING. TOOLS ARE A MEANS TO REACH AN EDIT, VALIDATION, COMMIT, OR CONCRETE DIAGNOSIS.**

Repeated discovery without new information is an operational failure.

- If two consecutive reads/searches using the same strategy produce no new useful information, stop using that strategy immediately.
- Do not repeat essentially the same query with only cosmetic wording changes.
- After an unsuccessful strategy, narrow to a known path/identifier, switch tool/access method, or proceed using the evidence already collected.
- A tool failure does not justify repeating the same call indefinitely. Change approach after the bounded retry above.
- Once the authoritative owner and likely edit paths are known, stop exploratory searching and begin implementation.
- After each small discovery block, ask internally: "Do I know which owner/file must change next?" If yes, edit instead of continuing to browse.
- Do not keep researching merely for reassurance when a safe concrete next step is already available.
- In implementation tasks, discovery must remain subordinate to delivery. Spending the execution on repeated GitHub queries and ending with no artifact is not acceptable.
- An implementation execution that ends with zero repository changes is considered an operational failure unless investigation demonstrates that no change is needed, the requested behavior already exists, or a concrete blocker makes a safe change impossible. In those cases, report the evidence/blocker precisely.

## Implementation discipline

Before editing:

- Identify the authoritative owner of the behavior/data being changed.
- Read the current implementation at that owner before designing a replacement.
- Look for existing project primitives in the relevant subsystem before creating a parallel system.
- Distinguish a local bug fix from an architectural change; do not invent a new subsystem for a symptom owned by an existing one.

While editing:

- Preserve unrelated existing work.
- Never overwrite, revert, or normalize unrelated changes just to simplify the task.
- Do not add compatibility layers, fallback behavior, duplicate paths, or defensive legacy behavior unless required by the current contract.
- Keep the change scoped to the user instruction and the invariants required to implement it correctly.
- If blocked after making independently correct progress, preserve that progress only when it is safe and useful, but report the task as incomplete rather than pretending the full delivery succeeded.

## Validation and review before publication

Validation is part of implementation, not an optional follow-up.

Before publishing a change:

- Inspect the final changed-file set and diff/state against the original user instruction.
- Verify every requested item is represented and no unrelated files slipped in.
- Run the narrowest relevant validation first, then broader validation when the change requires it.
- For reproducible bugs, add or update a regression test when practical.
- For Rust changes, run the repository's applicable Rust checks/Clippy/test gate before claiming correctness when the environment permits it.
- For content/assets, run the corresponding localization/content/GLB/asset audits when applicable.
- Do not claim that behavior works merely because code compiles.
- Do not claim CI is green until the relevant workflow/status has actually been observed as successful for the delivered commit.

### CI completion is mandatory

> **AN IMPLEMENTATION TASK IS NOT COMPLETE UNTIL THE CI FOR THE DELIVERED COMMIT HAS FINISHED AND EVERY REQUIRED CHECK IS GREEN.**

For every implementation task in a repository with CI:

- After publishing the delivered commit, locate the CI/workflow run for that exact commit SHA and follow it through completion.
- A queued, waiting, requested, pending, or in-progress CI run means the task is still incomplete. Never report the task as ready, done, finished, delivered, fixed, or equivalent while CI has not completed.
- If any required job/check fails, inspect the failing logs, correct the underlying issue, publish a new commit, and repeat the CI cycle. Do not stop at the first fix if a later CI stage exposes another failure.
- Continue until the CI run associated with the final delivered commit completes successfully with every required job/check green.
- If CI cannot run or cannot be observed because of an external infrastructure failure, service outage, or missing permission, report the task as blocked/incomplete and identify the blocker. Do not reinterpret unavailable CI as success.
- Never use a successful CI run from an older commit, another branch, or an earlier attempt as evidence for the final delivered commit.

## Git delivery and completion criteria

> **ASSISTANT TEXT IS NOT DELIVERY EVIDENCE. THE TARGET REMOTE BRANCH AND ITS COMPLETED GREEN CI ARE THE SOURCE OF TRUTH.**

A task may be reported as complete only after this chain is satisfied:

`complete requirement coverage -> final diff/state review -> relevant validation -> commit(s) -> publish to target branch -> verify remote SHA/reachability -> CI for delivered SHA completed successfully with every required check green`

Concretely:

- Do not say "done", "applied", "pushed", "ready", "finished", or equivalent when changes only exist in an editor, local state, a temporary branch, an unverified commit, or a commit whose CI has not completed successfully.
- If publication fails, the task is not complete.
- If the target branch moved and the expected commit is not reachable from its new HEAD, the task is not complete.
- If validation or CI fails, state that failure explicitly; fix it and rerun the delivery cycle instead of converting it into a success claim.
- If CI is queued, pending, or in progress, the task remains incomplete.
- At the end of an implementation task, report the target branch, delivered commit SHA(s), and the completed CI run/check status actually verified for the final delivered SHA.
- When a task uses several commits, verify the remote contains the entire required sequence, not merely the last commit created by the agent.

## Forward-only feature policy

> **NEW FEATURES DO NOT CARE ABOUT BACKWARD COMPATIBILITY.**

Asteria is developed forward-only. When implementing a new feature, prefer the cleanest current architecture and data model even if that breaks compatibility with previous implementations, serialized formats, APIs, authored data, saves, configs, or internal behavior.

Do not add compatibility shims, legacy branches, migrations, aliases, duplicated paths, deprecated fallbacks, or preservation logic for old behavior unless the user explicitly requests compatibility for that specific change.

When old code or data conflicts with the intended new design, replace or remove the old contract instead of preserving it.

## River generation architecture

Rivers are authored connected structures, not a separate hydrology graph or world-generation subsystem.

- A river has no independent `source`. Its endpoints are existing bodies of water such as oceans and lakes.
- Oceans/lakes may author spaced, deterministic connector placements on their margins. Connector spacing/chance/jitter belong to the normal structure-placement rules.
- River mouths and river segments use the generic Structure/connector system. River segments excavate terrain and place water through authored structure payloads.
- River chains continue through authored river-structure variations until they reach another body of water or ordinary generic structure constraints prevent continuation.
- River variation belongs in Structure groups; do not hardcode river shapes, meanders, downstream graphs, sinks, basins, source selection, or bespoke river rasterization in Rust.
- Do not recreate `rivers.rs`, `DimensionRiverNetwork`, drainage graphs, basin/source selection, or any equivalent parallel hydrology owner.
- Generic connector improvements required by rivers must remain generic and usable by other connected structures.

Any older documentation describing rivers as a dedicated hydrology network is superseded by this section.

## Documentation placement

- Keep repository-wide normative rules in `AGENTS.md`, `ARCHITECTURE.md`, or `ENGINEERING_PRACTICES.md`.
- Keep agent/tool/Git execution discipline in `AGENTS.md`; do not duplicate these rules into feature docs or handoffs.
- Keep only the current implementation handoff in root `HANDOFF.md`.
- Put feature/design/guides/QA documentation under the matching category in `docs/`.
- Put superseded plans and checklists in `docs/archive/`.
- Put historical handoffs in `docs/handoffs/`; never create new `HANDOFF_ARCHIVE_*.md` files in the repository root.
