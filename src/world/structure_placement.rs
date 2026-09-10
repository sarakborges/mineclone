use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StructureRotation {
    #[default]
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePlacement {
    pub structure_id: String,
    pub origin: IVec3,
    pub rotation: StructureRotation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureSetPlacement {
    pub structure_set_id: String,
    pub origin: IVec3,
    pub rotation: StructureRotation,
    pub structures: Vec<StructurePlacement>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StructureRegion {
    pub standalone_structures: Vec<StructurePlacement>,
    pub structure_sets: Vec<StructureSetPlacement>,
}

impl StructureRegion {
    pub fn iter_structures(&self) -> impl Iterator<Item = &StructurePlacement> {
        self.standalone_structures.iter().chain(
            self.structure_sets
                .iter()
                .flat_map(|structure_set| structure_set.structures.iter()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_sets_remain_distinct_from_their_child_structures() {
        let house = StructurePlacement {
            structure_id: "asteria:house".to_string(),
            origin: IVec3::new(4, 0, 2),
            rotation: StructureRotation::Degrees90,
        };
        let well = StructurePlacement {
            structure_id: "asteria:well".to_string(),
            origin: IVec3::ZERO,
            rotation: StructureRotation::Degrees0,
        };
        let region = StructureRegion {
            standalone_structures: Vec::new(),
            structure_sets: vec![StructureSetPlacement {
                structure_set_id: "asteria:village".to_string(),
                origin: IVec3::new(100, 70, -40),
                rotation: StructureRotation::Degrees180,
                structures: vec![house, well],
            }],
        };

        assert_eq!(region.structure_sets.len(), 1);
        assert_eq!(region.iter_structures().count(), 2);
    }
}
