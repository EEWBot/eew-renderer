use ordered_float::OrderedFloat;
use rangemap::RangeMap;

pub struct LOD<Type> {
    levels: Vec<Type>,
    scale_level_map: RangeMap<OrderedFloat<f32>, usize>,
}

impl<Type> LOD<Type> {
    pub fn get_level(&self, scale: f32) -> Option<&Type> {
        let level_index = *self.scale_level_map.get(&OrderedFloat::from(scale))?;
        self.levels.get(level_index)
    }

    pub fn new(levels: Vec<Type>, scale_level_map: RangeMap<OrderedFloat<f32>, usize>) -> Result<LOD<Type>, ()> {
        Ok(LOD { levels, scale_level_map })
    }
}
