//! xAI OAuth state shared by generic managed-auth commands.

use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct XaiOAuthState(pub Arc<RwLock<XaiOAuthManager>>);
