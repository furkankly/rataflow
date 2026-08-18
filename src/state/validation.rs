//! Connection validation operations for Flow.

use std::sync::Arc;

use super::Flow;
use crate::content::{EdgeContent, NodeContent};
use crate::types::Connection;

/// Type alias for connection validator callbacks.
pub(crate) type ConnectionValidator = Arc<dyn Fn(&Connection) -> bool + Send + Sync>;

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Sets a callback for validating connections.
    ///
    /// The callback is invoked during edge creation to determine if a connection
    /// is valid. Return `true` to allow the connection, `false` to reject it.
    pub fn set_connection_validator<F>(&mut self, validator: F)
    where
        F: Fn(&Connection) -> bool + Send + Sync + 'static,
    {
        self.connection_validator = Some(Arc::new(validator));
    }

    /// Clears the connection validator.
    pub fn clear_connection_validator(&mut self) {
        self.connection_validator = None;
    }

    /// Checks if a connection is valid according to the configured validator.
    ///
    /// Returns `true` if no validator is set or if the validator returns `true`.
    pub(crate) fn is_connection_valid(&self, connection: &Connection) -> bool {
        self.connection_validator
            .as_ref()
            .is_none_or(|v| v(connection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Node, Position};
    use crate::ui::TextContent;

    fn make_test_state(nodes: Vec<Node<TextContent>>) -> Flow {
        Flow::with_graph(nodes, Vec::new()).unwrap()
    }

    #[test]
    fn test_validator_allows() {
        let nodes = vec![Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )];
        let mut state = make_test_state(nodes);
        state.set_connection_validator(|_| true);

        let conn = Connection::new(
            "a",
            Some("right".to_string()),
            "b",
            Some("left".to_string()),
        );
        assert!(state.is_connection_valid(&conn));
    }

    #[test]
    fn test_validator_rejects() {
        let nodes = vec![Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )];
        let mut state = make_test_state(nodes);
        state.set_connection_validator(|conn| !conn.target.starts_with("blocked_"));

        let conn = Connection::new(
            "a",
            Some("right".to_string()),
            "blocked_b",
            Some("left".to_string()),
        );
        assert!(!state.is_connection_valid(&conn));
    }

    #[test]
    fn test_no_validator_allows_all() {
        let nodes = vec![Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )];
        let state = make_test_state(nodes);

        let conn = Connection::new(
            "a",
            Some("right".to_string()),
            "b",
            Some("left".to_string()),
        );
        assert!(state.is_connection_valid(&conn));
    }

    #[test]
    fn test_clear_validator() {
        let nodes = vec![Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )];
        let mut state = make_test_state(nodes);
        state.set_connection_validator(|_| false);

        let conn = Connection::new(
            "a",
            Some("right".to_string()),
            "b",
            Some("left".to_string()),
        );
        assert!(!state.is_connection_valid(&conn));

        state.clear_connection_validator();
        assert!(state.is_connection_valid(&conn));
    }
}
