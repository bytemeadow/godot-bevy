#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_collection_accessors_report_exact_contents() {
        let first = Entity::from_bits(7);
        let second = Entity::from_bits(11);
        let absent = Entity::from_bits(13);
        let children = GodotChildren(vec![first, second]);

        assert_eq!(children.len(), 2);
        assert!(!children.is_empty());
        assert_eq!(children.get(0), Some(first));
        assert_eq!(children.get(1), Some(second));
        assert_eq!(children.get(2), None);
        assert!(children.contains(first));
        assert!(!children.contains(absent));
        assert!(GodotChildren::default().is_empty());
    }
}
