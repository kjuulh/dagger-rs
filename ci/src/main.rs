use std::sync::Arc;

use dagger_sdk::gen::{Container, HostDirectoryOpts, Query};

fn main() -> eyre::Result<()> {
    color_eyre::install().unwrap();

    let matches = clap::Command::new("ci")
        .subcommand_required(true)
        .subcommand(clap::Command::new("pr"))
        .subcommand(clap::Command::new("release"))
        .get_matches();

    let client = dagger_sdk::client::connect()?;

    match matches.subcommand() {
        Some(("pr", _)) => {
            let base = select_base_image(client.clone());
            return validate_pr(client, base);
        }
        Some(("release", subm)) => return release(client, subm),
        Some(_) => {
            panic!("invalid subcommand selected!")
        }
        None => {
            panic!("no command selected!")
        }
    }
}

fn release(client: Arc<Query>, _subm: &clap::ArgMatches) -> Result<(), color_eyre::Report> {
    let src_dir = client.host().directory(
        ".".into(),
        Some(HostDirectoryOpts {
            exclude: Some(vec!["target/".into()]),
            include: None,
        }),
    );
    let base_image = client
        .container(None)
        .from("rust:latest".into())
        .with_workdir("app".into())
        .with_mounted_directory("/app/".into(), src_dir.id());

    let container = base_image
        .with_exec(
            vec![
                "cargo".into(),
                "install".into(),
                "cargo-smart-release".into(),
            ],
            None,
        )
        .with_exec(
            vec![
                "cargo".into(),
                "smart-release".into(),
                "--execute".into(),
                "--allow-fully-generated-changelogs".into(),
                "--no-changelog-preview".into(),
                "dagger-rs".into(),
                "dagger-sdk".into(),
            ],
            None,
        );
    let exit = container.exit_code();
    if exit != 0 {
        eyre::bail!("container failed with non-zero exit code");
    }

    println!("released pr succeeded!");

    Ok(())
}

fn get_dependencies(client: Arc<Query>) -> Container {
    let cargo_dir = client.host().directory(
        ".".into(),
        Some(HostDirectoryOpts {
            exclude: None,
            include: Some(vec![
                "**/Cargo.lock".into(),
                "**/Cargo.toml".into(),
                "**/main.rs".into(),
                "**/lib.rs".into(),
            ]),
        }),
    );

    let src_dir = client.host().directory(
        ".".into(),
        Some(HostDirectoryOpts {
            exclude: Some(vec!["target/".into()]),
            include: None,
        }),
    );

    let cache_cargo_index_dir = client.cache_volume("cargo_index".into());
    let cache_cargo_deps = client.cache_volume("cargo_deps".into());
    let cache_cargo_bin = client.cache_volume("cargo_bin_cache".into());

    let minio_url = "https://github.com/mozilla/sccache/releases/download/v0.3.3/sccache-v0.3.3-x86_64-unknown-linux-musl.tar.gz".into();

    let base_image = client
        .container(None)
        .from("rust:latest".into())
        .with_workdir("app".into())
        .with_exec(vec!["apt-get".into(), "update".into()], None)
        .with_exec(
            vec![
                "apt-get".into(),
                "install".into(),
                "--yes".into(),
                "libpq-dev".into(),
                "wget".into(),
            ],
            None,
        )
        .with_exec(vec!["wget".into(), minio_url], None)
        .with_exec(
            vec![
                "tar".into(),
                "xzf".into(),
                "sccache-v0.3.3-x86_64-unknown-linux-musl.tar.gz".into(),
            ],
            None,
        )
        .with_exec(
            vec![
                "mv".into(),
                "sccache-v0.3.3-x86_64-unknown-linux-musl/sccache".into(),
                "/usr/local/bin/sccache".into(),
            ],
            None,
        )
        .with_exec(
            vec!["chmod".into(), "+x".into(), "/usr/local/bin/sccache".into()],
            None,
        )
        .with_env_variable("RUSTC_WRAPPER".into(), "/usr/local/bin/sccache".into())
        .with_env_variable(
            "AWS_ACCESS_KEY_ID".into(),
            std::env::var("AWS_ACCESS_KEY_ID").unwrap_or("".into()),
        )
        .with_env_variable(
            "AWS_SECRET_ACCESS_KEY".into(),
            std::env::var("AWS_SECRET_ACCESS_KEY").unwrap_or("".into()),
        )
        .with_env_variable("SCCACHE_BUCKET".into(), "sccache".into())
        .with_env_variable("SCCACHE_REGION".into(), "auto".into())
        .with_env_variable(
            "SCCACHE_ENDPOINT".into(),
            "https://api-minio.front.kjuulh.io".into(),
        )
        .with_mounted_cache("~/.cargo/bin".into(), cache_cargo_bin.id(), None)
        .with_mounted_cache("~/.cargo/registry/index".into(), cache_cargo_bin.id(), None)
        .with_mounted_cache("~/.cargo/registry/cache".into(), cache_cargo_bin.id(), None)
        .with_mounted_cache("~/.cargo/git/db".into(), cache_cargo_bin.id(), None)
        .with_mounted_cache("target/".into(), cache_cargo_bin.id(), None)
        .with_exec(
            vec!["cargo".into(), "install".into(), "cargo-chef".into()],
            None,
        );

    let recipe = base_image
        .with_mounted_directory(".".into(), cargo_dir.id())
        .with_mounted_cache(
            "~/.cargo/.package-cache".into(),
            cache_cargo_index_dir.id(),
            None,
        )
        .with_exec(
            vec![
                "cargo".into(),
                "chef".into(),
                "prepare".into(),
                "--recipe-path".into(),
                "recipe.json".into(),
            ],
            None,
        )
        .file("/app/recipe.json".into());

    let builder_start = base_image
        .with_mounted_file("/app/recipe.json".into(), recipe.id())
        .with_exec(
            vec![
                "cargo".into(),
                "chef".into(),
                "cook".into(),
                "--release".into(),
                "--workspace".into(),
                "--recipe-path".into(),
                "recipe.json".into(),
            ],
            None,
        )
        .with_mounted_cache("/app/".into(), cache_cargo_deps.id(), None)
        .with_mounted_directory("/app/".into(), src_dir.id())
        .with_exec(
            vec![
                "cargo".into(),
                "build".into(),
                "--all".into(),
                "--release".into(),
            ],
            None,
        );

    return builder_start;
}

fn select_base_image(client: Arc<Query>) -> Container {
    let src_dir = get_dependencies(client.clone());

    src_dir
}

fn validate_pr(_client: Arc<Query>, container: Container) -> eyre::Result<()> {
    //let container = container.with_exec(vec!["cargo".into(), "test".into(), "--all".into()], None);

    let exit = container.exit_code();
    if exit != 0 {
        eyre::bail!("container failed with non-zero exit code");
    }

    println!("validating pr succeeded!");

    Ok(())
}
