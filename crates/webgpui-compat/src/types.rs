use thiserror::Error;

/// Opaque node handle. Invalidated after `node_remove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// Opaque listener handle returned by `event_on`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(pub u64);

/// Node type — mirrors the legacy string `"container"` / `"text"` / `"image"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Container,
    Text,
    Image,
}

/// Errors returned by compat API calls.
#[derive(Debug, Error)]
pub enum CompatError {
    #[error("invalid node id")]
    InvalidNode,
    #[error("invalid parent id")]
    InvalidParent,
    #[error("operation not supported in MVP")]
    NotSupported,
}

pub type CompatResult<T> = Result<T, CompatError>;
