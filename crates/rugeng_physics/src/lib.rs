pub mod collision_3d;
pub mod collision_solver;
pub mod raycast;
pub mod spatial_hash;
pub mod spatial_hash_3d;

pub use collision_3d::{
    resolve_aabb3d_collision, resolve_sphere3d_collision, Capsule3D, CollisionManifold3D, OBB3D,
};
pub use collision_solver::{
    resolve_aabb_collision, resolve_aabb_collision_with_velocity, resolve_one_way_platform,
    resolve_slope_collision, resolve_slope_collision_with_velocity, test_aabb_overlap,
    CollisionManifold, SlopeType,
};
pub use raycast::{
    ray_intersect_aabb, ray_intersect_aabb_3d, ray_intersect_sphere, raycast_grid,
    raycast_grid_custom, Ray2D, Ray3D, RayHit2D, RayHit3D,
};
pub use rugeng_core::{BoundingSphere, AABB3D};
pub use spatial_hash::{SpatialEntry, SpatialHashGrid, DEFAULT_BUCKET_COUNT, DEFAULT_CELL_SIZE};
pub use spatial_hash_3d::{
    SpatialEntry3D, SpatialHashGrid3D, DEFAULT_BUCKET_COUNT_3D, DEFAULT_CELL_SIZE_3D,
};
