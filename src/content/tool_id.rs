use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

static TOOL_ID_INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

pub(crate) fn intern_tool_id(id: &str) -> &'static str {
    let interner = TOOL_ID_INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut ids = interner
        .lock()
        .expect("tool ID interner lock was poisoned");

    if let Some(&interned) = ids.get(id) {
        return interned;
    }

    let interned = Box::leak(id.to_owned().into_boxed_str());
    ids.insert(id.to_owned(), interned);
    interned
}

#[cfg(test)]
mod tests {
    use super::intern_tool_id;

    #[test]
    fn tool_ids_are_interned_once_per_process() {
        let first = intern_tool_id("asteria:test_tool");
        let second = intern_tool_id("asteria:test_tool");

        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
    }
}
