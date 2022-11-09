/* SPDX-License-Identifier: AGPL-3.0-only
 *
 * This file is part of HarTex.
 *
 * HarTex
 * Copyright (c) 2021-2022 HarTex Project Developers
 *
 * HarTex is free software; you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * HarTex is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License along
 * with HarTex. If not, see <https://www.gnu.org/licenses/>.
 */

#![feature(int_roundings)]

use futures_util::future;
use futures_util::FutureExt;
use hartex_discord_core::discord::gateway::error::SendError;
use hartex_discord_core::discord::gateway::message::CloseFrame;
use hartex_discord_core::discord::gateway::{Session, Shard};
use hartex_discord_core::dotenvy;
use hartex_discord_core::log;
use hartex_discord_core::tokio;
use hartex_discord_core::tokio::signal;
use hartex_discord_core::tokio::task::LocalSet;
use lapin::options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties, ExchangeKind};

mod clusters;
mod inbound;
mod queue;

#[tokio::main(flavor = "multi_thread")]
pub async fn main() -> hartex_discord_eyre::Result<()> {
    hartex_discord_eyre::initialize()?;
    log::initialize();

    log::trace!("loading environment variables");
    dotenvy::dotenv()?;

    let username = std::env::var("GATEWAY_RABBITMQ_USERNAME")?;
    let password = std::env::var("GATEWAY_RABBITMQ_PASSWORD")?;
    let host = std::env::var("RABBITMQ_HOST")?;
    let port = std::env::var("RABBITMQ_PORT")?;
    let uri = format!("amqp://{username}:{password}@{host}:{port}");
    let uri_log = format!("amqp://{username}:<redacted>@{host}:{port}");

    log::trace!("creating rabbitmq amqp connection (uri: {})", &uri_log);
    let amqp_connection = Connection::connect(&uri, ConnectionProperties::default()).await?;

    let channel_inbound = amqp_connection.create_channel().await?;
    let channel_outbound = amqp_connection.create_channel().await?;

    log::trace!("declaring amqp exchange");
    channel_inbound
        .exchange_declare(
            "gateway",
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                passive: false,
                durable: true,
                auto_delete: false,
                internal: false,
                nowait: false,
            },
            FieldTable::default(),
        )
        .await?;

    log::trace!("declaring amqp outbound queue");
    channel_outbound
        .queue_declare(
            "gateway.outbound",
            QueueDeclareOptions {
                passive: false,
                durable: true,
                exclusive: false,
                auto_delete: false,
                nowait: false,
            },
            FieldTable::default(),
        )
        .await?;

    log::trace!("declaring and binding amqp inbound queue");
    channel_inbound
        .queue_declare(
            "gateway.inbound",
            QueueDeclareOptions {
                passive: false,
                durable: true,
                exclusive: false,
                auto_delete: false,
                nowait: false,
            },
            FieldTable::default(),
        )
        .await?;
    channel_inbound
        .queue_bind(
            "gateway.inbound",
            "gateway",
            "#",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    log::trace!("building clusters");
    let shards = std::env::var("NUM_SHARDS")?.parse::<u64>()?;
    let queue = queue::get_queue()?;
    let mut clusters = clusters::get_clusters(shards, queue.clone())?;

    log::trace!(
        "launching {} cluster(s) with {shards} shard(s)",
        clusters.len(),
    );

    let local = LocalSet::new();
    for (cluster_id, mut cluster) in clusters {
        let amqp = channel_inbound.clone();
        local.spawn_local(async move {
            inbound::handle_inbound(cluster_id as usize, cluster.iter_mut(), amqp).await
        });
    }

    let ctrlc = signal::ctrl_c();
    futures_util::select! {
        _ = local.fuse() => {},
        _ = ctrlc.fuse() => {
            log::warn!("ctrl-c signal received, shutting down");
            channel_inbound.close(1, "user-initiated shutdown").await?;
            channel_outbound.close(1, "user-initiated shutdown").await?;
        }
    }

    /*let _: Vec<Result<Option<Session>, SendError>> = future::join_all(
        clusters
            .iter_mut()
            .flat_map(|(_, mut cluster)| cluster.iter_mut())
            .map(|shard: &mut Shard| async { shard.close(CloseFrame::RESUME).await }),
    )
    .await;*/

    Ok(())
}
