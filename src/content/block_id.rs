use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

static BLOCK_ID_INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

pub(crate) fn intern_block_id(id: &str) -> &'static str {
    let interner = BLOCK_ID_INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut ids = interner
        .lock()
        .expect("block ID interner lock was poisoned");

    if let Some(&interned) = ids.get(id) {
        return interned;
    }

    let interned = Box::leak(id.to_owned().into_boxed_str());
    ids.insert(id.to_owned(), interned);
    interned
}

#[cfg(test)]
mod tests {
    use super::intern_block_id;

    #[test]
    fn block_ids_are_interned_once_per_process() {
        let first = intern_block_id("asteria:test");
        let second = intern_block_id("asteria:test");

        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
    }
}
