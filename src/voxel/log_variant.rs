use crate::content::{block::BlockRegistry, block_id::intern_block_id};

const LOG_PREFIX: &str = "asteria:log_";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LogVariant {
    Natural,
    Stripped,
    Hollow,
    StrippedHollow,
}

impl LogVariant {
    pub(crate) const fn is_hollow(self) -> bool {
        matches!(self, Self::Hollow | Self::StrippedHollow)
    }

    const fn suffix(self) -> &'static str {
        match self {
            Self::Natural => "",
            Self::Stripped => "_stripped",
            Self::Hollow => "_hollow",
            Self::StrippedHollow => "_stripped_hollow",
        }
    }
}

pub(crate) fn log_variant(block_id: &str) -> Option<LogVariant> {
    let body = block_id.strip_prefix(LOG_PREFIX)?;
    let (wood, variant) = if let Some(wood) = body.strip_suffix("_stripped_hollow") {
        (wood, LogVariant::StrippedHollow)
    } else if let Some(wood) = body.strip_suffix("_stripped") {
        (wood, LogVariant::Stripped)
    } else if let Some(wood) = body.strip_suffix("_hollow") {
        (wood, LogVariant::Hollow)
    } else {
        (body, LogVariant::Natural)
    };
    (!wood.is_empty()).then_some(variant)
}

pub(crate) fn is_hollow_log_id(block_id: &str) -> bool {
    log_variant(block_id).is_some_and(LogVariant::is_hollow)
}

pub(crate) fn transformed_log_id(
    block_id: &str,
    target: LogVariant,
    blocks: &BlockRegistry,
) -> Option<&'static str> {
    let body = block_id.strip_prefix(LOG_PREFIX)?;
    let wood = body
        .strip_suffix("_stripped_hollow")
        .or_else(|| body.strip_suffix("_stripped"))
        .or_else(|| body.strip_suffix("_hollow"))
        .unwrap_or(body);
    if wood.is_empty() {
        return None;
    }

    let candidate = format!("{LOG_PREFIX}{wood}{}", target.suffix());
    blocks.get(&candidate)?;
    Some(intern_block_id(&candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_log_variants() {
        assert_eq!(log_variant("asteria:log_oak"), Some(LogVariant::Natural));
        assert_eq!(
            log_variant("asteria:log_oak_stripped"),
            Some(LogVariant::Stripped)
        );
        assert_eq!(
            log_variant("asteria:log_oak_hollow"),
            Some(LogVariant::Hollow)
        );
        assert_eq!(
            log_variant("asteria:log_world_tree_stripped_hollow"),
            Some(LogVariant::StrippedHollow)
        );
    }

    #[test]
    fn rejects_non_log_ids() {
        assert_eq!(log_variant("asteria:stone"), None);
    }
}
