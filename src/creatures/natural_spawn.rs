/// Phase-1 boundary while biome/world generation is rebuilt.
///
/// Natural spawning depends on authoritative biome and terrain queries. The
/// legacy implementation reconstructed those worldgen internals directly, so
/// it stays disabled until the new query capabilities exist.
pub(super) fn natural_spawn_creatures() {}
