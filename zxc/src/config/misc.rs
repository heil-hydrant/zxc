// Combine two Option<Vec<String>>
pub fn add_option_vec(
    one: Option<Vec<String>>,
    two: Option<Vec<String>>,
) -> Option<Vec<String>> {
    match (one, two) {
        (Some(a), Some(b)) => Some(a.into_iter().chain(b).collect()),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

/// Removes empty values and duplicates from an `Option<Vec<String>>`, then
/// sorts it.
///
/// # Note
/// `is_empty()` is not implemented for generics:
/// <https://github.com/rust-lang/rust/issues/35428>
pub fn sanitize_option_vec_string(list: &mut Option<Vec<String>>) {
    if let Some(vec) = list.as_mut() {
        vec.retain(|s| !s.is_empty());
        vec.sort_unstable();
        vec.dedup();
        if vec.is_empty() {
            *list = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add_option_vec(None, None), None);
        assert_eq!(
            add_option_vec(None, Some(vec!["a".to_string()])),
            Some(vec!["a".to_string()])
        );
        assert_eq!(
            add_option_vec(Some(vec!["a".to_string()]), None),
            Some(vec!["a".to_string()])
        );
        assert_eq!(
            add_option_vec(
                Some(vec!["a".to_string(), "b".to_string()]),
                Some(vec!["b".to_string(), "a".to_string()])
            ),
            Some(vec![
                "a".to_string(),
                "b".to_string(),
                "b".to_string(),
                "a".to_string()
            ])
        );
    }

    #[test]
    fn test_remove_empty_single_val_dup() {
        let mut list =
            Some(vec!["a".to_string(), "".to_string(), "a".to_string()]);
        sanitize_option_vec_string(&mut list);
        assert_eq!(list, Some(vec!["a".to_string()]));
    }

    #[test]
    fn test_remove_empty_no_val() {
        let mut list = Some(vec!["".to_string(), "".to_string()]);
        sanitize_option_vec_string(&mut list);
        assert_eq!(list, None);
    }
}
