//! Domain layer: models, relations and franchise clustering.

pub mod cluster;
pub mod model;
pub mod relation;
pub mod watch_order;

pub use cluster::{Cluster, ClusterMember, RelationGraph, build_clusters};
pub use model::{Anime, Library, LibraryEntry, LibraryStatus};
pub use relation::RelationKind;
