use sea_orm_migration::{prelude::*, schema::*, sea_orm::raw_sql};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261031_080034_access_keys"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table("entries")
                    .add_column(string("reference_id").null())
                    .to_owned(),
            )
            .await?;

        let connection = manager.get_connection();
        connection
            .execute_raw(raw_sql!(
                Postgres,
                "update entries set reference_id = gen_random_uuid()"
            ))
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table("entries")
                    .modify_column(
                        ColumnDef::new("reference_id")
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table("access_keys")
                    .col(string("key").not_null().primary_key())
                    .col(string("entry_reference").not_null())
                    .col(timestamp("created_at").not_null())
                    .col(date_time("valid_until"))
                    .col(boolean("retracted").not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from_col("entry_reference")
                            .to("entries", "reference_id"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table("access_keys")
                    .col("entry_reference")
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("access_keys").to_owned())
            .await?;

        manager
            .alter_table(Table::alter().table("entries").drop_column("reference_id").to_owned())
            .await?;

        Ok(())
    }
}
