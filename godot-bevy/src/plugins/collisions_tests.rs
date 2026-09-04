#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_state_add_remove() {
        let mut state = CollisionState::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        let e3 = Entity::from_bits(3);

        state.add_collision(e1, e2);
        assert!(state.contains(e1, e2));
        assert!(state.contains(e2, e1)); // Symmetric
        assert!(!state.contains(e1, e3));

        assert_eq!(state.colliding_with(e1), &[e2]);
        assert_eq!(state.colliding_with(e2), &[e1]);
        assert!(state.colliding_with(e3).is_empty());

        assert_eq!(state.started_this_frame.len(), 1);

        state.remove_collision(e1, e2);
        assert!(!state.contains(e1, e2));
        assert!(state.colliding_with(e1).is_empty());

        assert_eq!(state.ended_this_frame.len(), 1);
    }

    #[test]
    fn test_collision_state_begin_frame() {
        let mut state = CollisionState::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);

        state.add_collision(e1, e2);
        assert_eq!(state.started_this_frame.len(), 1);

        state.begin_frame();
        assert!(state.started_this_frame.is_empty());
        assert!(state.ended_this_frame.is_empty());

        // But collision should still be active
        assert!(state.contains(e1, e2));
    }

    #[test]
    fn test_normalize_pair() {
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);

        assert_eq!(normalize_pair(e1, e2), normalize_pair(e2, e1));
    }

    #[test]
    fn test_collision_state_multiple_collisions() {
        let mut state = CollisionState::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        let e3 = Entity::from_bits(3);

        state.add_collision(e1, e2);
        state.add_collision(e1, e3);

        // e1 collides with both
        let colliding = state.colliding_with(e1);
        assert_eq!(colliding.len(), 2);
        assert!(colliding.contains(&e2));
        assert!(colliding.contains(&e3));

        // e2 only collides with e1
        assert_eq!(state.colliding_with(e2), &[e1]);

        // e3 only collides with e1
        assert_eq!(state.colliding_with(e3), &[e1]);
    }

    #[test]
    fn test_purge_entity_removes_all_pairs() {
        let mut state = CollisionState::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        let e3 = Entity::from_bits(3);

        state.add_collision(e1, e2);
        state.add_collision(e1, e3);
        state.add_collision(e2, e3);

        let mut purged = state.purge_entity(e1);
        purged.sort();
        assert_eq!(purged, vec![e2, e3]);

        // e1's pairs are gone; the e2-e3 pair survives.
        assert!(!state.contains(e1, e2));
        assert!(!state.contains(e1, e3));
        assert!(state.contains(e2, e3));

        // e1 is dropped from every neighbor's adjacency list.
        assert_eq!(state.colliding_with(e2), &[e3]);
        assert_eq!(state.colliding_with(e3), &[e2]);
        assert!(state.colliding_with(e1).is_empty());
    }

    #[test]
    fn test_purge_entity_with_no_pairs() {
        let mut state = CollisionState::default();
        let e1 = Entity::from_bits(1);
        assert!(state.purge_entity(e1).is_empty());
    }

    #[test]
    fn test_duplicate_collision_ignored() {
        let mut state = CollisionState::default();
        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);

        state.add_collision(e1, e2);
        state.add_collision(e1, e2); // Duplicate
        state.add_collision(e2, e1); // Same pair, different order

        assert_eq!(state.len(), 1);
        assert_eq!(state.started_this_frame.len(), 1);
    }
}
