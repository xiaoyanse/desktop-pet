mod list;
mod panels;
#[cfg(test)]
mod tests;
use crate::{
    application::clipboard::{ClipboardService, State},
    domain::clipboard::{ClipboardConfig, Content},
    ui::common::{
        actions,
        components::{
            fields, selection,
            status::{self, Tone},
        },
        icons::Icon,
        theme,
    },
};
use eframe::egui::{self, RichText};
use std::collections::HashMap;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Text,
    Picture,
}
enum Confirmation {
    Delete(u64),
    Clear,
}
#[derive(Default)]
pub struct Page {
    query: String,
    filter: Filter,
    key: Option<(u64, String, Filter)>,
    filtered: Vec<u64>,
    textures: HashMap<u64, egui::TextureHandle>,
    confirm: Option<Confirmation>,
    detail: Option<u64>,
    policy: Option<ClipboardConfig>,
}
impl Page {
    pub fn release(&mut self) {
        self.textures.clear();
        self.confirm = None;
        self.detail = None;
        self.policy = None;
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut ClipboardConfig,
        service: &mut ClipboardService,
    ) {
        self.controls(ui, service);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            for (filter, label) in [
                (Filter::All, "全部"),
                (Filter::Text, "文字"),
                (Filter::Picture, "图片"),
            ] {
                if selection::choice(ui, label, self.filter == filter, 64.0).clicked() {
                    self.filter = filter;
                }
            }
            fields::line(ui, &mut self.query, "搜索文字或图片尺寸", 128);
        });
        self.refresh(service);
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!(
                "{} 条匹配 · {} 条历史 · {:.1} / {} MiB",
                self.filtered.len(),
                service.history.items.len(),
                service.history.bytes() as f64 / 1048576.0,
                service.config.mib
            ))
            .size(12.0)
            .color(theme::CHAMPAGNE),
        );
        ui.add_space(6.0);
        ui.add_enabled_ui(self.confirm.is_none(), |ui| list::show(ui, self, service));
        if self.confirm.is_some() || self.policy.is_some() || self.detail.is_some() {
            let width = (ui.ctx().screen_rect().width() - 64.0).clamp(300.0, 480.0);
            let dialog = egui::Modal::new(egui::Id::new("clipboard-dialog"))
                .frame(crate::ui::common::components::layout::dialog_frame())
                .show(ui.ctx(), |ui| {
                    ui.set_width(width);
                    if self.confirm.is_some() {
                        panels::confirmation(ui, self, service);
                    } else if self.policy.is_some() {
                        panels::policy(ui, self, config, service);
                    } else {
                        panels::detail(ui, self, service);
                    }
                });
            if dialog.should_close() {
                self.confirm = None;
                self.policy = None;
                self.detail = None;
            }
            self.refresh(service);
        }
    }
    pub(crate) fn preview_policy(&mut self, config: &ClipboardConfig) {
        self.policy = Some(config.clone());
    }
    pub(crate) fn preview_clear(&mut self) {
        self.confirm = Some(Confirmation::Clear);
    }
    fn controls(&mut self, ui: &mut egui::Ui, service: &mut ClipboardService) {
        let (label, tone) = match service.state {
            State::Off => (
                "未记录",
                if service.failed {
                    Tone::Error
                } else {
                    Tone::Neutral
                },
            ),
            State::Starting => ("正在开启", Tone::Neutral),
            State::Listening => ("记录已开启", Tone::Success),
            State::Stopping => ("正在停止", Tone::Warning),
        };
        ui.horizontal(|ui| {
            status::badge(ui, label, tone);
            ui.label(
                RichText::new("仅本次运行 · 默认关闭 · 退出后清空")
                    .size(12.0)
                    .color(theme::MUTED),
            );
        });
        match service.state {
            State::Off => {
                if actions::primary(ui, Icon::Play, "开始记录新复制内容", true).clicked() {
                    service.start();
                }
            }
            State::Starting | State::Listening => {
                if actions::primary(ui, Icon::Pause, "暂停记录 · 保留历史", true).clicked()
                {
                    service.pause();
                }
            }
            State::Stopping => {
                actions::primary(ui, Icon::Pause, "正在停止 · 不再接收内容", false);
            }
        }
        ui.label(
            RichText::new(
                "开启时不读取当前剪贴板；历史只保存在内存。关闭此页仍会继续记录，停止请用“暂停”。",
            )
            .size(12.0)
            .color(theme::MUTED),
        );
        ui.horizontal_wrapped(|ui| {
            if actions::secondary(ui, Icon::Settings, "容量设置").clicked() {
                self.policy = if self.policy.is_some() {
                    None
                } else {
                    Some(service.config.clone())
                };
            }
            ui.add_enabled_ui(!service.history.items.is_empty(), |ui| {
                if actions::secondary(ui, Icon::Cancel, "清空历史").clicked() {
                    self.confirm = Some(Confirmation::Clear);
                }
            });
            if service.copying() {
                status::badge(ui, "正在复制原始内容", Tone::Neutral);
            }
        });
        if !service.message.is_empty() {
            status::message(
                ui,
                &service.message,
                if service.failed {
                    Tone::Warning
                } else {
                    Tone::Neutral
                },
            );
        }
    }
    fn refresh(&mut self, service: &ClipboardService) {
        let key = (
            service.history.revision(),
            self.query.to_lowercase(),
            self.filter,
        );
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let mut items: Vec<_> = service
            .history
            .items
            .iter()
            .filter(|i| {
                let kind = matches!(
                    (&i.packet.content, self.filter),
                    (_, Filter::All)
                        | (Content::Text(_), Filter::Text)
                        | (Content::Picture(_), Filter::Picture)
                );
                kind && (key.1.is_empty()
                    || match &i.packet.content {
                        Content::Text(t) => t.to_lowercase().contains(&key.1),
                        Content::Picture(_) => i.packet.label().to_lowercase().contains(&key.1),
                    })
            })
            .collect();
        items.sort_by_key(|i| !i.pinned);
        self.filtered = items.into_iter().map(|i| i.id).collect();
        self.key = Some(key);
    }
}
