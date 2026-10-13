use crate::{
    application::update::{State, UpdateService},
    domain::update::{Preferences, VERSION},
    ui::{
        common::{actions, icons::Icon, theme, widgets},
        settings::section,
    },
};
use eframe::egui::{self, RichText};
#[cfg(test)]
mod tests;

pub(super) fn show(ui: &mut egui::Ui, preferences: &mut Preferences, service: &mut UpdateService) {
    section(ui, "版本与在线更新", |ui| {
        ui.label(
            RichText::new(format!("桌面工作助理 {VERSION}"))
                .size(18.0)
                .strong()
                .color(theme::TEXT),
        );
        ui.weak("通过 GitHub / Gitee 官方仓库 Release 更新，无需账号登录或授权码。");
        widgets::switch(
            ui,
            &mut preferences.automatic,
            "启动后自动检查版本（只检查，不自动安装）",
        );
        widgets::switch(ui, &mut preferences.pinned, "固定当前版本，暂停更新检查");
        widgets::switch(
            ui,
            &mut preferences.prefer_gitee,
            "优先使用 Gitee，连接失败时尝试 GitHub",
        );
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!service.busy() && !preferences.pinned, |ui| {
                if actions::secondary(ui, Icon::History, "检查更新").clicked() {
                    preferences.skipped.clear();
                    service.check(preferences);
                }
            });
            ui.hyperlink_to(
                "GitHub 发布页",
                "https://github.com/xiaoyanse/desktop-pet/releases",
            );
            ui.hyperlink_to(
                "Gitee 发布页",
                "https://gitee.com/xiaoyanse/desktop-pet/releases",
            );
        });
        ui.add_space(12.0);
        match &service.state {
            State::Idle => {
                ui.weak("尚未检查；检查与下载按需运行，不在后台持续轮询。");
            }
            State::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("正在检查公开发布版本…");
                });
            }
            State::Latest => {
                ui.colored_label(theme::ACCENT, "当前没有可用的新稳定版本。");
            }
            State::Failed(error) => {
                ui.colored_label(theme::DANGER, error);
            }
            State::Available(release) => {
                let version = release.version.clone();
                let page = release.page.clone();
                ui.label(format!(
                    "发现 {} · {} · 全量 {:.1} MB",
                    version,
                    release.source,
                    release.manifest.size as f64 / 1048576.0
                ));
                if let Some(delta) = &release.manifest.delta {
                    ui.weak(format!(
                        "匹配 {} 时优先增量 {:.1} MB；不匹配或重建失败则自动改用全量包。",
                        delta.base_version,
                        delta.size as f64 / 1048576.0
                    ));
                }
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .id_salt("release-notes")
                    .show(ui, |ui| {
                        ui.label(&release.notes);
                    });
                ui.horizontal_wrapped(|ui| {
                    if actions::secondary(ui, Icon::Down, "下载并校验").clicked() {
                        service.download();
                    }
                    if actions::secondary(ui, Icon::Cancel, "忽略此版本").clicked() {
                        preferences.skipped = version;
                        service.state = State::Latest;
                    }
                    ui.hyperlink_to("查看发布说明", page);
                });
            }
            State::Downloading(release) => {
                ui.label(format!("正在下载、校验与重建 {}…", release.version));
                ui.add(
                    egui::ProgressBar::new((service.progress() as f64 / 10000.0).min(1.0) as f32)
                        .show_percentage(),
                );
                if actions::secondary(ui, Icon::Cancel, "取消下载").clicked() {
                    service.cancel();
                }
            }
            State::Ready(release) => {
                ui.colored_label(
                    theme::ACCENT,
                    format!("{} 已下载并通过校验", release.version),
                );
                ui.weak("安装会先保存设置并退出监视器，保留旧版备份；新版启动失败时恢复原版。");
                ui.horizontal_wrapped(|ui| {
                    if actions::secondary(ui, Icon::Check, "保存并安装").clicked() {
                        service.install_requested = true;
                    }
                    if actions::secondary(ui, Icon::Timer, "下次启动安装").clicked()
                        && let Err(error) = service.schedule()
                    {
                        service.state = State::Failed(error);
                    }
                });
                if service.scheduled {
                    ui.weak("已安排：退出后，下次正常启动时安装。此刻继续工作。");
                }
            }
        }
    });
}
