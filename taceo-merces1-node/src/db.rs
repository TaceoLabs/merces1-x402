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
use std::str::FromStr;

use alloy::primitives::{Address, TxHash};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use chrono::{DateTime, Utc};
use eyre::Context as _;
use mpc_core::protocols::rep3::Rep3PrimeFieldShare;
use mpc_nodes::map::{DepositValueShare, PrivateDeposit};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, Row, migrate::Migrator};
use std::time::Duration;
use taceo_nodes_common::postgres::{PostgresConfig, SanitizedSchema, pg_pool_with_schema};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Deserialize)]
#[sqlx(type_name = "transaction_type")]
pub enum TransactionKind {
    #[sqlx(rename = "deposit")]
    Deposit,
    #[sqlx(rename = "withdraw")]
    Withdraw,
    #[sqlx(rename = "transfer")]
    Transfer,
}

#[derive(Debug, Clone, Serialize)]
pub enum Transaction {
    Deposit {
        id: i64,
        receiver: Address,
        tx_hash: Option<TxHash>,
        #[serde(with = "ark_serde_compat::field")]
        amount: ark_bn254::Fr,
        timestamp: DateTime<Utc>,
    },
    Withdraw {
        id: i64,
        sender: Address,
        tx_hash: Option<TxHash>,
        #[serde(with = "ark_serde_compat::field")]
        amount: ark_bn254::Fr,
        timestamp: DateTime<Utc>,
    },
    Transfer {
        id: i64,
        sender: Address,
        receiver: Address,
        tx_hash: Option<TxHash>,
        #[serde(with = "ark_serde_compat::field")]
        amount_commitment: ark_bn254::Fr,
        // For simplicity, we serialize the amount share as additive instead of REP3 (more convenient for clients, and wire format)
        #[serde(with = "ark_serde_compat::field")]
        amount_share: ark_bn254::Fr,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct TransactionFilter {
    pub sender: Option<Address>,
    pub receiver: Option<Address>,
    pub kind: Option<TransactionKind>,
}

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

            let share_bytes: Option<Vec<u8>> = row.get("share");
            let share = if let Some(share_bytes) = share_bytes {
                DepositValueShare::<ark_bn254::Fr>::deserialize_with_mode(
                    share_bytes.as_slice(),
                    ark_serialize::Compress::No,
                    ark_serialize::Validate::No,
                )
                .context("while deserializing share")?
            } else {
                DepositValueShare {
                    amount: Rep3PrimeFieldShare::default(),
                    blinding: Rep3PrimeFieldShare::default(),
                }
            };

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
                INSERT INTO map (address, pending)
                VALUES ($1, $2)
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

    /// Returns a page of transactions ordered by `id DESC`, with optional filters.
    pub(crate) async fn load_transactions(
        &self,
        offset: i64,
        limit: i64,
        filter: TransactionFilter,
    ) -> eyre::Result<Vec<Transaction>> {
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"SELECT id, sender, receiver, "type", tx_hash, amount::text, amount_commitment, amount_share, "timestamp" FROM transactions WHERE TRUE"#,
        );
        if let Some(sender) = filter.sender {
            qb.push(" AND sender = ")
                .push_bind(sender.as_slice().to_vec());
        }
        if let Some(receiver) = filter.receiver {
            qb.push(" AND receiver = ")
                .push_bind(receiver.as_slice().to_vec());
        }
        if let Some(kind) = filter.kind {
            qb.push(r#" AND "type" = "#).push_bind(kind);
        }
        qb.push(" ORDER BY id DESC LIMIT ").push_bind(limit);
        qb.push(" OFFSET ").push_bind(offset);

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .context("while loading transactions")?;

        let mut txs = Vec::with_capacity(rows.len());
        for row in rows {
            let sender_bytes: Vec<u8> = row.get("sender");
            let receiver_bytes: Vec<u8> = row.get("receiver");
            let tx_hash_bytes: Option<Vec<u8>> = row.get("tx_hash");
            let kind: TransactionKind = row.get("type");

            match kind {
                TransactionKind::Deposit => {
                    txs.push(Transaction::Deposit {
                        id: row.get("id"),
                        receiver: Address::try_from(receiver_bytes.as_slice())
                            .expect("valid address"),
                        tx_hash: tx_hash_bytes.map(|b| {
                            <[u8; 32]>::try_from(b.as_slice())
                                .map(TxHash::from)
                                .expect("valid tx_hash in DB")
                        }),
                        amount: ark_bn254::Fr::from_str(&row.get::<String, _>("amount"))
                            .expect("valid field element"),
                        timestamp: row.get("timestamp"),
                    });
                }
                TransactionKind::Withdraw => {
                    txs.push(Transaction::Withdraw {
                        id: row.get("id"),
                        sender: Address::try_from(sender_bytes.as_slice()).expect("valid address"),
                        tx_hash: tx_hash_bytes.map(|b| {
                            <[u8; 32]>::try_from(b.as_slice())
                                .map(TxHash::from)
                                .expect("valid tx_hash in DB")
                        }),
                        amount: ark_bn254::Fr::from_str(&row.get::<String, _>("amount"))
                            .expect("valid field element"),
                        timestamp: row.get("timestamp"),
                    });
                }
                TransactionKind::Transfer => {
                    txs.push(Transaction::Transfer {
                        id: row.get("id"),
                        sender: Address::try_from(sender_bytes.as_slice()).expect("valid address"),
                        receiver: Address::try_from(receiver_bytes.as_slice())
                            .expect("valid address"),
                        tx_hash: tx_hash_bytes.map(|b| {
                            <[u8; 32]>::try_from(b.as_slice())
                                .map(TxHash::from)
                                .expect("valid tx_hash in DB")
                        }),
                        amount_commitment: ark_bn254::Fr::from_str(
                            &row.get::<String, _>("amount_commitment"),
                        )
                        .expect("valid field element"),
                        amount_share: ark_bn254::Fr::from_str(
                            &row.get::<String, _>("amount_share"),
                        )
                        .expect("valid field element"),

                        timestamp: row.get("timestamp"),
                    });
                }
            }
        }

        Ok(txs)
    }

    #[expect(clippy::too_many_arguments)]
    pub(crate) async fn insert_transaction(
        &self,
        sender: Address,
        receiver: Address,
        kind: TransactionKind,
        tx_hash: Option<TxHash>,
        amount: Option<ark_bn254::Fr>,
        amount_commitment: Option<ark_bn254::Fr>,
        amount_share: Option<Rep3PrimeFieldShare<ark_bn254::Fr>>,
        executor: &mut PgConnection,
    ) -> eyre::Result<()> {
        sqlx::query(
            r#"
                INSERT INTO transactions
                    (sender, receiver, "type", tx_hash, amount, amount_commitment, amount_share)
                VALUES ($1, $2, $3, $4, $5::numeric, $6, $7)
                "#,
        )
        .bind(sender.as_slice())
        .bind(receiver.as_slice())
        .bind(kind)
        .bind(tx_hash.map(|h| h.as_slice().to_vec()))
        .bind(amount.map(|a| a.to_string()))
        .bind(amount_commitment.map(|c| c.to_string()))
        .bind(amount_share.map(|s| s.a.to_string()))
        .execute(&mut *executor)
        .await
        .context("while inserting transaction")?;
        Ok(())
    }

    pub(crate) async fn commit_map_update(
        &self,
        addresses: &[Address],
        executor: &mut PgConnection,
    ) -> eyre::Result<()> {
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
        .execute(&mut *executor)
        .await
        .context("while committing map update")?;

        Ok(())
    }
}
