/*
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * This file is part of HarTex.
 *
 * HarTex
 * Copyright (c) 2021-2023 HarTex Project Developers
 *
 * HarTex is free software; you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * HarTex is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License along
 * with HarTex. If not, see <https://www.gnu.org/licenses/>.
 */

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Build::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Build::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Build::Repository).string().not_null())
                    .col(ColumnDef::new(Build::Branch).string().not_null())
                    .col(ColumnDef::new(Build::CommitHash).string().not_null())
                    .col(ColumnDef::new(Build::Status).string().not_null())
                    .col(
                        ColumnDef::new(Build::CreatedAt)
                            .timestamp()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp))
                            .not_null(),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("unique-build-repo-branch-commit")
                            .col(Build::Repository)
                            .col(Build::Branch)
                            .col(Build::CommitHash),
                    )
                    .clone(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Build::Table).clone())
            .await
    }
}

#[derive(Iden)]
pub enum Build {
    Table,
    Id,
    Repository,
    Branch,
    CommitHash,
    Status,
    CreatedAt,
}
