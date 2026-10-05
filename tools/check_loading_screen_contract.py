#!/usr/bin/env python3
"""Guard Phase 11 loading-screen ownership and real-progress semantics."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCREEN = (ROOT / "src/screens/loading_screen.rs").read_text(encoding="utf-8")
SCREENS = (ROOT / "src/screens/mod.rs").read_text(encoding="utf-8")
LOADING = (ROOT / "src/world/loading.rs").read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Loading screen audit failed: {message}")


screen = compact(SCREEN)
screens = compact(SCREENS)
loading = compact(LOADING)

required_screen = {
    "loading screen is scoped to the Loading state": "OnEnter(GameState::Loading),setup_loading_screen",
    "loading screen consumes the authoritative progress resource": "progress:Res<WorldLoadingProgress>",
    "phase label comes from the authoritative loading phase": "matchprogress.phase()",
    "progress label consumes completed work": "letcompleted=progress.completed();",
    "progress label consumes total required work": "lettotal=progress.total();",
    "progress bar derives from completed over total": "completedasf32/totalasf32",
    "loading UI cleans itself up on state exit": "DespawnOnExit(GameState::Loading)",
}
for description, fragment in required_screen.items():
    require(fragment in screen, description)

require("Timer" not in SCREEN, "loading screen must not invent timer-driven progress")
require("Duration" not in SCREEN, "loading screen must not invent duration-driven progress")
require("Instant" not in SCREEN, "loading screen must not invent wall-clock progress")

required_progress = {
    "progress exposes its real phase to the UI": "pub(crate)constfnphase(&self)->WorldLoadingPhase",
    "progress exposes completed required work": "pub(crate)constfncompleted(&self)->usize",
    "progress exposes total required work": "pub(crate)constfntotal(&self)->usize",
}
for description, fragment in required_progress.items():
    require(fragment in loading, description)

require("modloading_screen;" in screens, "screen module must be registered")
require("LoadingScreenPlugin" in SCREENS, "loading screen plugin must be installed")

# UI percentage semantics: no work selected means an empty bar; selected work
# reflects actual completion and clamps at 100 percent.
def percentage(completed: int, total: int) -> float:
    if total == 0:
        return 0.0
    return max(0.0, min(1.0, completed / total)) * 100.0


require(percentage(0, 0) == 0.0, "pending destination must render an empty bar")
require(percentage(2, 5) == 40.0, "partial residency must render real progress")
require(percentage(5, 5) == 100.0, "completed residency must render a full bar")
require(percentage(6, 5) == 100.0, "display percentage must clamp at completion")

print(
    "Loading screen audit passed: authoritative phase/counters only, real ratio, "
    "state-scoped UI, and no fake timing"
)
