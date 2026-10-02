#[derive(Debug, PartialEq, Eq)]
pub(super) enum ParsedLine<'a> {
    Say(&'a str),
    Spawn(&'a str, Option<&'a str>),
    Place(&'a str, Option<usize>),
    Locate(&'a str, &'a str, Option<usize>),
    Warp(bevy::prelude::IVec3, Option<&'a str>),
    Kill,
    Modify(ModifyAction, &'a str, Option<&'a str>),
    Usage(&'static str),
    Unknown(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModifyAction {
    Add,
    Remove,
    Edit,
}

fn parse_optional_variation(value: Option<&str>) -> Result<Option<usize>, ()> {
    let Some(value) = value else {
        return Ok(None);
    };
    value
        .parse::<usize>()
        .ok()
        .filter(|variation| *variation > 0)
        .map(Some)
        .ok_or(())
}

pub(super) fn parse_line(input: &str) -> ParsedLine<'_> {
    let line = input.trim();
    if !line.starts_with('/') {
        return ParsedLine::Say(line);
    }

    let mut words = line.split_whitespace();
    let command = words.next().unwrap_or("/");
    let args: Vec<_> = words.collect();
    match command {
        "/spawn" => match args.as_slice() {
            [id] => ParsedLine::Spawn(id, None),
            [id, meta_tag] => ParsedLine::Spawn(id, Some(meta_tag)),
            _ => ParsedLine::Usage("/spawn <id> [meta_tag]"),
        },
        "/place" => {
            if args.len() < 2 || args.len() > 3 || args[0] != "structure" {
                return ParsedLine::Usage("/place structure <id> [variation]");
            }
            let Ok(variation) = parse_optional_variation(args.get(2).copied()) else {
                return ParsedLine::Usage("/place structure <id> [variation]");
            };
            ParsedLine::Place(args[1], variation)
        }
        "/locate" => match args.as_slice() {
            ["biome", id] => ParsedLine::Locate("biome", id, None),
            ["structure", id] => ParsedLine::Locate("structure", id, None),
            ["structure", id, variation] => match parse_optional_variation(Some(variation)) {
                Ok(variation) => ParsedLine::Locate("structure", id, variation),
                Err(()) => ParsedLine::Usage(
                    "/locate biome <id> | /locate structure <id> [variation]",
                ),
            },
            _ => ParsedLine::Usage(
                "/locate biome <id> | /locate structure <id> [variation]",
            ),
        },
        "/warp" => {
            let (x, z, y, dimension) = match args.as_slice() {
                [x, z, y] => (*x, *z, *y, None),
                [x, z, y, dimension] => (*x, *z, *y, Some(*dimension)),
                _ => return ParsedLine::Usage("/warp <x> <z> <y> [dimension]"),
            };
            let (Ok(x), Ok(z), Ok(y)) = (x.parse::<i32>(), z.parse::<i32>(), y.parse::<i32>()) else {
                return ParsedLine::Usage("/warp <x> <z> <y> [dimension]");
            };
            ParsedLine::Warp(bevy::prelude::IVec3::new(x, y, z), dimension)
        }
        "/kill" => {
            if args.is_empty() {
                ParsedLine::Kill
            } else {
                ParsedLine::Usage("/kill")
            }
        }
        "/modify" => {
            if args.len() < 2 || args.len() > 3 {
                return ParsedLine::Usage("/modify <add|remove|edit> <meta_tag> [value]");
            }
            let action = match args[0] {
                "add" => ModifyAction::Add,
                "remove" => ModifyAction::Remove,
                "edit" => ModifyAction::Edit,
                _ => return ParsedLine::Usage("/modify <add|remove|edit> <meta_tag> [value]"),
            };
            if action == ModifyAction::Remove && args.len() == 3 {
                return ParsedLine::Usage("/modify <add|remove|edit> <meta_tag> [value]");
            }
            ParsedLine::Modify(action, args[1], args.get(2).copied())
        }
        _ => ParsedLine::Unknown(command),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entity_commands() {
        assert_eq!(parse_line("/kill"), ParsedLine::Kill);
        assert_eq!(
            parse_line("/spawn asteria:slime NO_AI"),
            ParsedLine::Spawn("asteria:slime", Some("NO_AI"))
        );
        assert_eq!(
            parse_line("/modify add NO_AI"),
            ParsedLine::Modify(ModifyAction::Add, "NO_AI", None)
        );
        assert_eq!(
            parse_line("/modify edit NO_AI frozen"),
            ParsedLine::Modify(ModifyAction::Edit, "NO_AI", Some("frozen"))
        );
    }

    #[test]
    fn parses_warp_with_optional_dimension() {
        let target = bevy::prelude::IVec3::new(10, 30, 20);
        assert_eq!(parse_line("/warp 10 20 30"), ParsedLine::Warp(target, None));
        assert_eq!(
            parse_line("/warp 10 20 30 asteria:umbral"),
            ParsedLine::Warp(target, Some("asteria:umbral"))
        );
        assert_eq!(
            parse_line("/warp 10 20"),
            ParsedLine::Usage("/warp <x> <z> <y> [dimension]")
        );
    }
}
