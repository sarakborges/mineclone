# Chat and scrollbar QA — feature/game-chat-creature-command

Scope: Asteria 0.17.2, stacked draft PR #10 based on `feature/slime-creature-integration` (PR #9). This is a manual acceptance checklist, not a claim of gameplay validation.

## Chat controls and behavior

- In a loaded world, `T` opens the transparent chat above the left player HUD and focuses the input without inserting `t`.
- Type a normal message; `Enter` submits it and closes the input. The history formats it as `<Yogg'Sara>: message` until a dedicated player identity exists.
- `Esc` closes the chat without pausing or submitting an unfinished message. While editing chat, the character/camera should not respond to ordinary typing.
- Older messages are ABOVE newer ones. The history grows upward to a **maximum** of 15 rendered text lines, including wrapped lines, and then scrolls; it must not reserve 15 empty lines. Upon receiving a message it follows the bottom; the mouse wheel can scroll while the chat is open.
- The history remains visible while the input is open. With the chat closed, the history hides after 10 seconds without a new message.

## Command smoke cases

- `/spawn_creature asteria:meadow_slime` and `/spawn_creature asteria:ember_slime` should resolve their respective creature JSON IDs and attempt to spawn at the player's exact foot coordinates. A collision-blocked or unloaded location must return a chat error rather than spawn through solid terrain.
- Try `/spawn_creature` without an ID, an unknown ID, and an unknown command. Inspect the resulting chat feedback; commands must not be echoed as player chat.
- Visually check the imported GLB, per-species tint and animation after successful spawn. Compilation alone cannot validate these.

## Scrollbar cases

- Chat history, creative inventory categories/catalogue, Settings sidebar/content, and New World sidebar/content must show their vertical scrollbar only after the measured content exceeds the available height. At or below the cap, scrollbar is hidden; a resize and adding/removing content should update its visibility. Verify scrolling still works when overflow occurs.

## Automation coverage and merge gate

The canonical `.github/workflows/ci.yml` runs `cargo clippy --all-targets --all-features -- -D warnings` and `cargo check`, but does **not** execute unit tests or render/gameplay tests. The repair workflow run 35151578331 passed both checks on its generated patch and committed the validated changes as `071ff820055dd007468254273ffeedad9fc5665d`. The subsequent canonical PR run 35152046480 reported `action_required` with no jobs (GitHub Actions bot-authored commit); this documentation commit is intended to request a normal PR run on the actual branch HEAD. Keep PR #10 draft until normal CI and gameplay checks are confirmed, and validate PR #9 independently before merging the stack.
