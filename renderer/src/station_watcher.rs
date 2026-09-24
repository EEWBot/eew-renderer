use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

const DEBOUNCE: Duration = Duration::from_millis(300);

pub fn spawn(path: Option<PathBuf>) -> Result<Option<tokio::task::JoinHandle<()>>, notify::Error> {
    let Some(path) = path else {
        return Ok(None);
    };

    let (tx, rx) = mpsc::unbounded_channel();
    let watcher = watch(&path, tx)?;

    tracing::info!("Watching intensity stations file: {}", path.display());

    let handle = tokio::spawn(async move {
        let _watcher = watcher;

        watch_loop(path, rx, |path| {
            renderer_assets::QueryInterface::reload_intensity_stations(&path)
        })
        .await;
    });

    Ok(Some(handle))
}

fn watch(path: &Path, tx: mpsc::UnboundedSender<()>) -> Result<RecommendedWatcher, notify::Error> {
    let target = path.to_owned();
    let file_name = path.file_name().map(|v| v.to_owned());

    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                let related = event.paths.is_empty()
                    || event
                        .paths
                        .iter()
                        .any(|p| *p == target || p.file_name() == file_name.as_deref());

                if related {
                    let _ = tx.send(());
                }
            }
            Err(e) => tracing::warn!("Intensity stations watcher error: {e:?}"),
        })?;

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    watcher.watch(dir, RecursiveMode::NonRecursive)?;

    Ok(watcher)
}

async fn watch_loop<F, E>(path: PathBuf, mut rx: mpsc::UnboundedReceiver<()>, reload: F)
where
    F: Fn(PathBuf) -> Result<(), E> + Clone + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    while rx.recv().await.is_some() {
        tokio::time::sleep(DEBOUNCE).await;

        while rx.try_recv().is_ok() {}

        let (reload, target) = (reload.clone(), path.clone());
        let result = tokio::task::spawn_blocking(move || reload(target)).await;

        match result {
            Ok(Ok(())) => tracing::info!("Reloaded intensity stations from {}", path.display()),
            Ok(Err(e)) => tracing::error!(
                "Failed to reload intensity stations from {}; keeping previous data: {e:?}",
                path.display()
            ),
            Err(e) => tracing::error!(
                "Failed to reload intensity stations from {}; keeping previous data: {e:?}",
                path.display()
            ),
        }
    }
}
