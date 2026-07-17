use super::Ctx;
use crate::cli::ReleaseCmd;
use crate::error::Result;
use crate::models::{Release, ReleaseAsset};
use crate::out;

pub fn execute(ctx: &Ctx, cmd: ReleaseCmd) -> Result<()> {
    let o = ctx.repo.owner.as_str();
    let r = ctx.repo.name.as_str();
    match cmd {
        ReleaseCmd::List { limit } => {
            let path = format!("/repos/{o}/{r}/releases");
            let items: Vec<Release> = ctx.client.get_paged(&path, &[], limit)?;
            ctx.out.render(&items, || out::release_table(&items));
        }
        ReleaseCmd::View { tag } => {
            let release: Release = ctx
                .client
                .get(&format!("/repos/{o}/{r}/releases/tags/{tag}"), &[])?;
            ctx.out.render(&release, || out::one_release(&release));
        }
        ReleaseCmd::Create {
            tag,
            name,
            notes,
            target,
            prerelease,
        } => {
            let display_name = name.unwrap_or_else(|| tag.clone());
            let mut f: Vec<(&str, String)> = vec![("tag_name", tag), ("name", display_name)];
            if let Some(n) = notes {
                f.push(("body", n));
            }
            if let Some(t) = target {
                f.push(("target_commitish", t));
            }
            f.push((
                "prerelease",
                if prerelease {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ));
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let release: Release = ctx.client.post(&format!("/repos/{o}/{r}/releases"), &form)?;
            ctx.out.render(&release, || out::one_release(&release));
        }
        ReleaseCmd::Upload { tag, files } => {
            let release: Release = ctx
                .client
                .get(&format!("/repos/{o}/{r}/releases/tags/{tag}"), &[])?;
            let id = release.id;
            for file_path in files {
                let asset: ReleaseAsset = ctx.client.post_multipart(
                    &format!("/repos/{o}/{r}/releases/{id}/attach_files"),
                    &file_path,
                )?;
                println!("{}", asset.name);
            }
        }
    }
    Ok(())
}
