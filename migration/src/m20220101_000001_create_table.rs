use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("entries")
                    .if_not_exists()
                    .col(string("owner").not_null())
                    .col(string("dir").not_null())
                    .col(string("name").not_null())
                    // Above is composite primary key
                    .col(string("owner_type").not_null())
                    .col(boolean("is_dir").not_null())
                    .col(string("creator").not_null())
                    .col(string("storage_path").null())
                    .col(string("thumbnail_path").null())
                    .col(timestamp("created_at").not_null())
                    .primary_key(Index::create().col("owner").col("dir").col("name"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .table("entries")
                    .col("dir")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("entries").to_owned())
            .await?;

        Ok(())
    }
}
