#!/usr/bin/env python3
"""Guard Phase 10 world-loading orchestration and progress semantics."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOADING = (ROOT / "src/world/loading.rs").read_text(encoding="utf-8")
STREAMING = (ROOT / "src/world/streaming.rs").read_text(encoding="utf-8")
GENERATION = (ROOT / "src/world/streaming/generation.rs").read_text(encoding="utf-8")
WORLD = (ROOT / "src/world/mod.rs").read_text(encoding="utf-8")
ACTIVATION = (ROOT / "src/screens/world_selection/activation.rs").read_text(encoding="utf-8")
WARP = (ROOT / "src/world/warp.rs").read_text(encoding="utf-8")


def compact(source: str) -> str:
    return "".join(source.split())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"Loading pipeline audit failed: {message}")


loading = compact(LOADING)
streaming = compact(STREAMING)
generation = compact(GENERATION)
world = compact(WORLD)
activation = compact(ACTIVATION)
warp = compact(WARP)

required_loading = {
    "loading exposes logical parent phases": "PreparingDestination,MaterializingInitialArea,Ready,",
    "load/new-world destination restores saved position when present": "letsaved_position=saved.and_then(|player|player.position());",
    "new-world spawn uses generator destination queries": "find_safe_spawn_position(&context.generator,IVec2::ZERO,|_|true)",
    "player is created before residency work": "spawn_player_entity(",
    "loading center is derived from the destination": "loading.center=Some(center);",
    "progress publishes real residency counters": "let(completed,total)=streaming.desired_residency_counts(&world);",
    "gameplay waits for all required residency and idle materialization": "iftotal==0||completed!=total||!streaming.materialization_is_idle(){return;}",
    "ready loading transitions to gameplay": "next_state.set(GameState::Gameplay);",
}
for description, fragment in required_loading.items():
    require(fragment in loading, description)

require("Duration" not in LOADING and "Timer" not in LOADING, "loading progress must not use fake timing")

required_streaming = {
    "loading state is owned by the streaming/loading adapter": ".init_resource::<WorldLoadingState>()",
    "loading progress is an independent resource": ".init_resource::<WorldLoadingProgress>()",
    "loading entry resets stale materialization work": "reset_resource::<ChunkMaterializationTasks>",
    "loading entry resets progress": "reset_resource::<WorldLoadingProgress>",
    "destination preparation runs only while loading": "prepare_loading_destination.run_if(in_state(GameState::Loading))",
    "the same streaming owner runs during loading": ".run_if(world_streaming_active)",
    "loading completion is ordered after streaming": "finish_loading_when_ready.after(stream_chunks).run_if(in_state(GameState::Loading))",
    "presentation remains gameplay continuation": "publish_pending_chunk_presentations.after(process_dynamic_lighting).before(process_chunk_remesh_queue).run_if(in_state(GameState::Gameplay))",
    "loading selects its explicit small destination radius": "(center,inputs.loading.horizontal_radius())",
    "progress counts authoritative desired residency": "self.residency.desired.iter().filter(|coord|world.chunk(**coord).is_some()).count()",
    "completion observes pending and in-flight materialization": "self.pending.len()==0&&self.materializing.is_empty()",
}
for description, fragment in required_streaming.items():
    require(fragment in streaming, description)

required_generation = {
    "materialization results carry input revisions": "TaskInputRevision",
    "stale async results are rejected": "ifcompleted.revision!=current_revision",
    "still-required stale work is requeued": "ifstreaming.keeps_loaded(coord){streaming.enqueue_pending(coord);}",
}
for description, fragment in required_generation.items():
    require(fragment in generation, description)

required_world = {
    "loading receives a fresh frame work budget": "begin_world_frame_work_budget.run_if(in_state(GameState::Loading))",
    "new-world shell is prepared before generator installation": "prepare_world_session,install_world_generator",
    "new worlds reserve canonical persisted identity": "create_new_world(",
    "new worlds install an empty runtime voxel world": "commands.insert_resource(VoxelWorld::default());",
    "new worlds initialize in-memory save ownership": ".begin_new_world(seed,context.current_dimension.id.as_str(),rules);",
    "session release returns future entry to New mode": "commands.insert_resource(WorldLoadMode::New);",
}
for description, fragment in required_world.items():
    require(fragment in world, description)

required_activation = {
    "save loading commits the persisted voxel world": "commands.insert_resource(self.world);",
    "save loading marks the next Loading pass as Load mode": "commands.insert_resource(WorldLoadMode::Load);",
}
for description, fragment in required_activation.items():
    require(fragment in activation, description)

required_warp = {
    "dimension travel swaps runtime dimension state before loading": ".swap_to(&previous_dimension,&requested_dimension)",
    "dimension travel stores the requested destination in the shared save state": "dimension.save.prepare_dimension_warp(&requested_dimension);",
    "dimension travel selects the target dimension before loading": "dimension.current_dimension.id=DimensionId::from(requested_dimension.clone());",
    "dimension travel enters the same Load-mode pipeline": "*dimension.load_mode=WorldLoadMode::Load;",
    "dimension travel transitions through Loading": "dimension.next_game_state.set(GameState::Loading);",
}
for description, fragment in required_warp.items():
    require(fragment in warp, description)

# Small semantic fixture for the required-to-enter-gameplay gate. Presentation,
# remeshing and normal render-distance expansion are deliberately background
# continuation after this required residency boundary.
def ready(completed: int, total: int, idle: bool) -> bool:
    return total > 0 and completed == total and idle


require(not ready(0, 0, True), "an empty selection cannot report ready")
require(not ready(4, 5, True), "partial residency cannot report ready")
require(not ready(5, 5, False), "in-flight materialization cannot report ready")
require(ready(5, 5, True), "complete idle required residency must report ready")

print(
    "Loading pipeline audit passed: shared new/load/dimension entry, query-guided destination, "
    "real residency progress, stale-result rejection, and background presentation separation"
)
