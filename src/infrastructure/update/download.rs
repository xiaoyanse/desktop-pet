use super::http;
use crate::domain::update::Release;
use std::{
    fs::File,
    future::Future,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    task::Poll,
    time::Duration,
};

pub(super) fn run(
    path: &Path,
    release: &Release,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<(), String> {
    let url = http::asset_url(&release.url, &release.source)?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("DesktopMeter/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(http::redirect_policy())
        .build()
        .map_err(|_| "无法初始化更新下载连接")?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "无法启动更新下载任务")?;
    runtime.block_on(async {
        let mut task = Box::pin(async {
            let mut response = client
                .get(url)
                .send()
                .await
                .map_err(|_| "更新包下载连接失败")?;
            if !response.status().is_success() {
                return Err(format!(
                    "更新包下载返回 HTTP {}",
                    response.status().as_u16()
                ));
            }
            if response
                .content_length()
                .is_some_and(|size| size != release.manifest.size)
            {
                return Err("更新包长度与清单不符".into());
            }
            let mut output = File::create(path).map_err(|_| "更新缓存无法写入")?;
            let mut written = 0u64;
            while let Some(chunk) = response.chunk().await.map_err(|_| "下载中断，请重新下载")?
            {
                if cancel.load(Ordering::Relaxed) {
                    return Err("下载已取消".into());
                }
                written += chunk.len() as u64;
                if written > release.manifest.size {
                    return Err("更新包超出清单大小".into());
                }
                output.write_all(&chunk).map_err(|_| "更新缓存写入失败")?;
                progress.store(written * 10000 / release.manifest.size, Ordering::Relaxed);
            }
            output.sync_all().map_err(|_| "更新缓存同步失败")?;
            if written != release.manifest.size {
                return Err("更新包长度与清单不符".into());
            }
            Ok(())
        });
        let mut timer = Box::pin(tokio::time::sleep(Duration::from_millis(50)));
        std::future::poll_fn(|context| {
            if cancel.load(Ordering::Relaxed) {
                return Poll::Ready(Err("下载已取消".into()));
            }
            if let Poll::Ready(result) = task.as_mut().poll(context) {
                return Poll::Ready(result);
            }
            if timer.as_mut().poll(context).is_ready() {
                timer
                    .as_mut()
                    .reset(tokio::time::Instant::now() + Duration::from_millis(50));
                context.waker().wake_by_ref();
            }
            Poll::Pending
        })
        .await
    })
}
