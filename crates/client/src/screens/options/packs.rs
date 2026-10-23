//! The resource packs tab: enable, disable and order the packs of the resource packs
//! folder. The list is kept lowest priority first; the last pack wins.

/// Enabled: removed; disabled: added on top.
pub fn toggle(enabled: &mut Vec<String>, name: &str) {
    match enabled.iter().position(|n| n == name) {
        Some(index) => {
            enabled.remove(index);
        }
        None => enabled.push(name.to_owned()),
    }
}

/// Raises (`step` > 0) or lowers an enabled pack's priority by one place.
pub fn reorder(enabled: &mut [String], name: &str, step: i32) {
    let Some(index) = enabled.iter().position(|n| n == name) else {
        return;
    };
    let target = (index as i64 + i64::from(step.signum())).clamp(0, enabled.len() as i64 - 1);
    enabled.swap(index, target as usize);
}

/// 1 for the pack that wins over all others, `None` when disabled.
pub fn rank(enabled: &[String], name: &str) -> Option<usize> {
    let index = enabled.iter().position(|n| n == name)?;
    Some(enabled.len() - index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_are_enabled_on_top_and_reordered() {
        let mut enabled = Vec::new();
        toggle(&mut enabled, "a");
        toggle(&mut enabled, "b");
        assert_eq!(
            (rank(&enabled, "b"), rank(&enabled, "a")),
            (Some(1), Some(2))
        );
        reorder(&mut enabled, "a", 1);
        assert_eq!(rank(&enabled, "a"), Some(1));
        reorder(&mut enabled, "a", 1);
        assert_eq!(enabled, ["b", "a"]);
        toggle(&mut enabled, "a");
        assert_eq!((rank(&enabled, "a"), enabled.len()), (None, 1));
    }
}
