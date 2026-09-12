use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use warp_core::sync_queue::{IsTransientError, SyncQueueTaskTrait};

use super::diff_state::{DiffMode, DiffStateModel, FileDiffAndContent};
use crate::util::git::GitExecTarget;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FileInvalidationError(#[from] anyhow::Error);

impl IsTransientError for FileInvalidationError {
    fn is_transient(&self) -> bool {
        true
    }
}

pub struct FileInvalidationTask {
    pub file: PathBuf,
    /// Transport the diff for `file` is computed against. Derived from the
    /// active [`DiffStateModel`] at enqueue time so a remote session runs its
    /// per-file invalidation on the remote host.
    pub exec_target: GitExecTarget,
    pub mode: DiffMode,
    pub merge_base: Option<String>,
}

impl SyncQueueTaskTrait for FileInvalidationTask {
    type Error = FileInvalidationError;
    type Result = (PathBuf, Option<FileDiffAndContent>);
    #[cfg(not(target_arch = "wasm32"))]
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Result, Self::Error>> + Send>>;
    #[cfg(target_arch = "wasm32")]
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Result, Self::Error>>>>;

    fn run(&mut self) -> Self::Fut {
        let exec_target = self.exec_target.clone();
        let file = self.file.clone();
        let mode = self.mode.clone();
        let merge_base = self.merge_base.clone();
        Box::pin(async move {
            DiffStateModel::retrieve_diff_state(&exec_target, &file, &mode, merge_base.as_deref())
                .await
                .map_err(FileInvalidationError::from)
        })
    }
}
