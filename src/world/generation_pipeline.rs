#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationStage {
    SurfaceColumns,
    Density,
    Materials,
    Fluids,
    Features,
}

pub const GENERATION_STAGE_ORDER: [GenerationStage; 5] = [
    GenerationStage::SurfaceColumns,
    GenerationStage::Density,
    GenerationStage::Materials,
    GenerationStage::Fluids,
    GenerationStage::Features,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_stage_order_keeps_dependencies_explicit() {
        assert_eq!(
            GENERATION_STAGE_ORDER,
            [
                GenerationStage::SurfaceColumns,
                GenerationStage::Density,
                GenerationStage::Materials,
                GenerationStage::Fluids,
                GenerationStage::Features,
            ]
        );
    }
}
