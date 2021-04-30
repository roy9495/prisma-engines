use crate::{runtime::run_with_tokio, AnyError, Tags};
use enumflags2::BitFlags;
use quaint::{prelude::Queryable, single::Quaint};
use url::Url;

pub(crate) fn get_postgres_tags(database_url: &str) -> Result<BitFlags<Tags>, String> {
    let fut = async {
        let quaint = Quaint::new(database_url).await.map_err(|err| err.to_string())?;
        let mut tags = Tags::Postgres.into();
        let version = quaint.version().await.map_err(|err| err.to_string())?;

        match version {
            None => Ok(tags),
            Some(version) => {
                eprintln!("version: {:?}", version);

                if version.contains("12.") {
                    tags |= Tags::Postgres12;
                }

                eprintln!("Inferred tags: {:?}", tags);

                Ok(tags)
            }
        }
    };

    run_with_tokio(fut)
}

pub async fn create_postgres_database(db_name: &str) -> Result<(Quaint, String), AnyError> {
    let mut url: Url = super::TAGS.as_ref().unwrap().database_url.parse()?;
    let mut postgres_db_url = url.clone();

    url.set_path(db_name);
    postgres_db_url.set_path("/postgres");

    let drop = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        "#,
        db_name = db_name,
    );

    let recreate = format!(
        r#"
        CREATE DATABASE "{db_name}";
        "#,
        db_name = db_name,
    );

    let conn = Quaint::new(postgres_db_url.as_str()).await?;

    // The two commands have to be run separately on postgres.
    conn.raw_cmd(&drop).await?;
    conn.raw_cmd(&recreate).await?;

    url.query_pairs_mut()
        .append_pair("statement_cache_size", "0")
        .append_pair("schema", "prisma-tests");

    let url_str = url.to_string();

    let conn = Quaint::new(&url_str).await?;

    conn.raw_cmd("CREATE SCHEMA \"prisma-tests\"").await?;

    Ok((conn, dbg!(url_str)))
}