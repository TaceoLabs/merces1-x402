//! Database helpers for the Merces1 node service.
//!
//! Persists the `address → secret-share` map so that nodes can survive
//! restarts without losing their share of the protocol state.
//!
//! # Schema
//!
//! A single `secret_shares` table keyed by the 20-byte Ethereum address.
//! The share is stored as a CBOR-serialized blob using
//! [`ark_serialize::CanonicalSerialize`].

use std::collections::HashMap;

use alloy::primitives::Address;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use eyre::Context as _;
use mpc_nodes::map::{DepositValueShare, PrivateDeposit};
use secrecy::SecretString;
use sqlx::{PgPool, Row, migrate::Migrator};
use std::time::Duration;
use taceo_nodes_common::postgres::{PostgresConfig, SanitizedSchema, pg_pool_with_schema};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Thin wrapper around a [`sqlx::PgPool`] with secret-share–specific helpers.
#[derive(Clone, Debug)]
pub struct DbPool {
    pub(crate) pool: PgPool,
}

impl DbPool {
    /// Opens a PostgreSQL connection pool and applies pending migrations.
    pub(crate) async fn open(
        db_url: &SecretString,
        db_schema: &SanitizedSchema,
        acquire_timeout: Duration,
    ) -> eyre::Result<Self> {
        let mut postgres_config =
            PostgresConfig::with_default_values(db_url.clone(), db_schema.clone());
        postgres_config.acquire_timeout = acquire_timeout;

        let pool = pg_pool_with_schema(
            &postgres_config,
            taceo_nodes_common::postgres::CreateSchema::Yes,
        )
        .await
        .context("while creating DB pool")?;

        Self::from_pool(pool).await
    }

    /// Builds a [`DbPool`] from an existing pool and runs migrations.
    pub(crate) async fn from_pool(pool: PgPool) -> eyre::Result<Self> {
        MIGRATOR.run(&pool).await.context("while migrating DB")?;
        Ok(Self { pool })
    }

    /// Loads all rows from `secret_shares` and returns them as a `HashMap`.
    ///
    /// Returns an empty map when the table contains no rows (e.g. first
    /// startup of a fresh node).
    pub(crate) async fn load_map(
        &self,
    ) -> eyre::Result<PrivateDeposit<Address, DepositValueShare<ark_bn254::Fr>>> {
        let rows = sqlx::query("SELECT address, share FROM map")
            .fetch_all(&self.pool)
            .await
            .context("while loading secret_shares")?;

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            let addr_bytes: Vec<u8> = row.get("address");
            let address = Address::try_from(addr_bytes.as_slice())
                .map_err(|_| eyre::eyre!("invalid address bytes in DB"))?;

            let share_bytes: Vec<u8> = row.get("share");
            let share = DepositValueShare::<ark_bn254::Fr>::deserialize_with_mode(
                share_bytes.as_slice(),
                ark_serialize::Compress::No,
                ark_serialize::Validate::No,
            )
            .context("while deserializing share")?;

            map.insert(address, share);
        }

        Ok(map.into())
    }

    /// Inserts or updates all `(address, share)` pairs in a single statement.
    ///
    /// Uses upsert semantics so that calling this after a crash-and-replay is
    /// safe — existing rows are overwritten with the latest share.
    pub(crate) async fn update_map(
        &self,
        rows: &[(Address, DepositValueShare<ark_bn254::Fr>)],
    ) -> eyre::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        for (address, share) in rows {
            let mut buf = Vec::new();
            share
                .serialize_with_mode(&mut buf, ark_serialize::Compress::No)
                .context("while serializing share")?;

            sqlx::query(
                "
                INSERT INTO map (address, share, pending)
                VALUES ($1, $2, $2)
                ON CONFLICT (address) DO UPDATE SET pending = EXCLUDED.pending
                ",
            )
            .bind(address.as_slice())
            .bind(buf)
            .execute(&mut *tx)
            .await
            .context("while upserting share")?;
        }
        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn commit_map_update(&self, addresses: &[Address]) -> eyre::Result<()> {
        let address_bytes: Vec<Vec<u8>> = addresses
            .iter()
            .map(|addr| addr.as_slice().to_vec())
            .collect();

        sqlx::query(
            "
            UPDATE map
            SET share = pending, pending = NULL
            WHERE address = ANY($1::bytea[])
            ",
        )
        .bind(&address_bytes)
        .execute(&self.pool)
        .await
        .context("while committing map update")?;

        Ok(())
    }
}
