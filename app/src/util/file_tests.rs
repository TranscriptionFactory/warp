use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use warp_util::path::{CleanPathResult, LineAndColumnArg};

use super::{
    absolute_path_if_valid, LinkValidationContext, RemoteDirListing, RemoteLinkValidation,
    ShellPathType,
};

const CWD: &str = "/home/me/proj";

fn remote_ctx(resolved: &[(&str, Option<bool>)]) -> LinkValidationContext {
    let listing = RemoteDirListing::new(
        PathBuf::from(CWD),
        HashMap::from([("README.md".to_string(), false)]),
    );
    LinkValidationContext::Remote(RemoteLinkValidation {
        cwd_listing: Some(Arc::new(listing)),
        resolved: Arc::new(
            resolved
                .iter()
                .map(|(path, is_dir)| (PathBuf::from(path), *is_dir))
                .collect(),
        ),
        unresolved: Default::default(),
    })
}

fn check(path: &str, line: Option<usize>, ctx: &LinkValidationContext) -> Option<PathBuf> {
    let clean = CleanPathResult {
        path: path.to_string(),
        line_and_column_num: line.map(|line_num| LineAndColumnArg {
            line_num,
            column_num: None,
        }),
    };
    absolute_path_if_valid(
        &clean,
        ShellPathType::ShellNative(CWD.to_string()),
        None,
        ctx,
    )
}

fn unresolved(ctx: &LinkValidationContext) -> Vec<PathBuf> {
    match ctx {
        LinkValidationContext::Remote(remote) => remote.unresolved.lock().unwrap().clone(),
        LinkValidationContext::Local => vec![],
    }
}

#[test]
fn remote_cwd_child_uses_listing() {
    let ctx = remote_ctx(&[]);
    assert_eq!(
        check("README.md", None, &ctx),
        Some(PathBuf::from("/home/me/proj/README.md"))
    );
}

#[test]
fn remote_nested_relative_path_uses_resolved_cache() {
    let ctx = remote_ctx(&[("/home/me/proj/app/src/main.rs", Some(false))]);
    assert_eq!(
        check("app/src/main.rs", Some(12), &ctx),
        Some(PathBuf::from("/home/me/proj/app/src/main.rs"))
    );
}

#[test]
fn remote_absolute_path_uses_resolved_cache() {
    let ctx = remote_ctx(&[("/tmp/out.png", Some(false))]);
    assert_eq!(
        check("/tmp/out.png", None, &ctx),
        Some(PathBuf::from("/tmp/out.png"))
    );
}

#[test]
fn remote_unknown_path_is_invalid_and_queued_for_resolution() {
    let ctx = remote_ctx(&[]);
    assert_eq!(check("app/src/main.rs", None, &ctx), None);
    assert!(unresolved(&ctx).contains(&PathBuf::from("/home/me/proj/app/src/main.rs")));
}

#[test]
fn remote_known_missing_path_is_not_queued_again() {
    let ctx = remote_ctx(&[("/home/me/proj/gone.rs", None), ("gone.rs", None)]);
    assert_eq!(check("gone.rs", None, &ctx), None);
    assert!(unresolved(&ctx).is_empty());
}

#[test]
fn remote_directory_with_line_number_is_invalid() {
    let ctx = remote_ctx(&[("/home/me/proj/app/src", Some(true))]);
    assert_eq!(check("app/src", Some(3), &ctx), None);
    assert_eq!(
        check("app/src", None, &ctx),
        Some(PathBuf::from("/home/me/proj/app/src"))
    );
}
