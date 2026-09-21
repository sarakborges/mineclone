use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

static LAYER_ID_INTERNER: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();

pub(crate) fn intern_layer_id(id: &str) -> &'static str {
    let interner = LAYER_ID_INTERNER.get_or_init(|| Mutex::new(HashSet::new()));
    let mut ids = interner.lock().expect("layer ID interner lock was poisoned");
    if let Some(&interned) = ids.get(id) {
        return interned;
    }
    let interned = Box::leak(id.to_owned().into_boxed_str());
    ids.insert(interned);
    interned
}

#[cfg(test)]
mod tests {
    use super::intern_layer_id;

    #[test]
    fn layer_ids_are_interned_once_per_process() {
        let first = intern_layer_id("asteria:test_layer");
        let second = intern_layer_id("asteria:test_layer");
        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
    }
}
