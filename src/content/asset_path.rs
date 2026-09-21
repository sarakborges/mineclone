use std::path::{Component, Path};

pub(crate) fn is_safe_relative_asset_path(path: &str) -> bool {
    if path.trim().is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
    {
        return false;
    }

    Path::new(path)
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::is_safe_relative_asset_path;

    #[test]
    fn asset_paths_stay_relative_and_cannot_traverse() {
        assert!(is_safe_relative_asset_path("textures/blocks/stone.png"));
        assert!(!is_safe_relative_asset_path("../stone.png"));
        assert!(!is_safe_relative_asset_path("/textures/stone.png"));
        assert!(!is_safe_relative_asset_path("textures\\stone.png"));
        assert!(!is_safe_relative_asset_path("C:/textures/stone.png"));
        assert!(!is_safe_relative_asset_path(""));
    }
}
