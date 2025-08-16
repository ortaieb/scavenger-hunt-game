pub mod api;

use crate::auth::AuthState;
use crate::db::DatabasePool;
use crate::services::{AuthService, ImageService, LocationService};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: DatabasePool,
    pub auth_service: Arc<AuthService>,
    pub location_service: Arc<LocationService>,
    pub image_service: Arc<ImageService>,
    pub auth_state: AuthState,
}

pub use api::create_api_router;
