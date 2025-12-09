// ussdgeth/src/reducers/keys.rs
// SpacetimeDB reducers to store/retrieve encrypted user private keys.
// Note: These reducers *expect* the controller/droplet to encrypt the private key using MASTER_KEY
// and send the encrypted base64 blob to the reducer. Reducers do NOT hold the master key.

use crate::{user_key, UserKey};
use spacetimedb::{reducer, ReducerContext, Table};

#[reducer]
pub fn store_user_key(ctx: &ReducerContext, phone_number: String, encrypted_blob: String) {
    // Upsert: if exists update, otherwise insert
    if let Some(existing) = ctx.db.user_key().phone_number().find(phone_number.clone()) {
        ctx.db.user_key().phone_number().update(UserKey {
            encrypted_key: encrypted_blob,
            updated_at: ctx.timestamp,
            ..existing
        });
        log::info!("Updated encrypted key for {}", phone_number);
    } else {
        ctx.db.user_key().insert(UserKey {
            phone_number: phone_number.clone(),
            encrypted_key: encrypted_blob,
            created_at: ctx.timestamp,
            updated_at: ctx.timestamp,
        });
        log::info!("Stored encrypted key for {}", phone_number);
    }
}

#[reducer]
pub fn delete_user_key(ctx: &ReducerContext, phone_number: String) {
    if let Some(existing) = ctx.db.user_key().phone_number().find(phone_number.clone()) {
        ctx.db
            .user_key()
            .phone_number()
            .delete(&existing.phone_number);
        log::info!("Deleted user key for {}", phone_number);
    } else {
        log::warn!("delete_user_key: not found {}", phone_number);
    }
}

/// Fetch reducer: finds and logs the encrypted blob. Controllers usually fetch via SQL API.
/// This reducer logs the blob (redacted) for debugging.
#[reducer]
pub fn fetch_user_key(ctx: &ReducerContext, phone_number: String) {
    if let Some(existing) = ctx.db.user_key().phone_number().find(phone_number.clone()) {
        let blob = &existing.encrypted_key;
        let short = if blob.len() > 12 {
            format!("{}...{}", &blob[0..6], &blob[blob.len() - 6..])
        } else {
            blob.clone()
        };
        log::info!("fetch_user_key for {} -> {}", phone_number, short);
    } else {
        log::warn!("fetch_user_key: not found {}", phone_number);
    }
}
