#![windows_subsystem = "windows"]

mod audio;
mod utils;

use audio::AudioManager;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    list: HashSet<String>,
    is_whitelist: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            list: HashSet::new(),
            is_whitelist: false,
        }
    }
}

struct SilencerApp {
    config: Config,
    is_running: bool,
    audio_manager: AudioManager,
    new_item: String,
    rx: Receiver<()>,
    hook_handle: Option<windows::Win32::UI::Accessibility::HWINEVENTHOOK>,
    active_sessions: Vec<audio::AudioSessionInfo>,
    last_refresh: std::time::Instant,
    last_audio_enforcement: std::time::Instant,
    show_sponsor: bool,
    show_help: bool,
    wechat_qr: Option<egui::TextureHandle>,
    alipay_qr: Option<egui::TextureHandle>,
}

static EVENT_SENDER: OnceLock<Sender<()>> = OnceLock::new();

unsafe extern "system" fn win_event_callback(
    _: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _: u32,
    _: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if let Some(sender) = EVENT_SENDER.get() {
        let _ = sender.send(());
    }
}

impl SilencerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize image loaders
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Set modern visuals
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = 12.0.into();
        visuals.widgets.noninteractive.rounding = 8.0.into();
        visuals.widgets.inactive.rounding = 8.0.into();
        visuals.widgets.hovered.rounding = 8.0.into();
        visuals.widgets.active.rounding = 8.0.into();
        visuals.widgets.open.rounding = 8.0.into();
        cc.egui_ctx.set_visuals(visuals);

        // Load fonts with specific priority
        let mut fonts = egui::FontDefinitions::default();
        
        // Font paths to try, in order of fallback (last one is highest priority in the loop below)
        let font_configs = [
            ("emoji", "C:\\Windows\\Fonts\\seguiemj.ttf"),
            ("symbol", "C:\\Windows\\Fonts\\seguisym.ttf"),
            ("nirmala", "C:\\Windows\\Fonts\\Nirmala.ttf"),
            ("msyh", "C:\\Windows\\Fonts\\msyh.ttc"),
            ("simsun", "C:\\Windows\\Fonts\\simsun.ttc"),   // 恢复宋体
        ];

        for (name, path) in font_configs {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert(name.to_owned(), egui::FontData::from_owned(font_data));
                // 每次插入到索引 0，所以数组中最后的 simsun 会排在最前面
                fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, name.to_owned());
                fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, name.to_owned());
            }
        }
        cc.egui_ctx.set_fonts(fonts);

        let config = cc.storage
            .and_then(|s| s.get_string(eframe::APP_KEY))
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let (tx, rx) = channel();
        let mut hook_handle = None;
        let _ = EVENT_SENDER.set(tx);
        unsafe {
            let handle = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_callback),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if !handle.is_invalid() {
                hook_handle = Some(handle);
            }
        }

        let audio_manager = AudioManager::new().expect("Failed to initialize audio manager");
        let active_sessions = audio_manager.get_active_sessions().unwrap_or_default();

        // Load QR codes manually to ensure they display
        let wechat_qr = {
            let bytes = include_bytes!("../photo/naicha_weixin.png");
            if let Ok(image) = image::load_from_memory(bytes) {
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                Some(cc.egui_ctx.load_texture(
                    "wechat_qr",
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                    Default::default(),
                ))
            } else {
                None
            }
        };

        let alipay_qr = {
            let bytes = include_bytes!("../photo/naicha_zhifubao.png");
            if let Ok(image) = image::load_from_memory(bytes) {
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                Some(cc.egui_ctx.load_texture(
                    "alipay_qr",
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
                    Default::default(),
                ))
            } else {
                None
            }
        };

        Self {
            config,
            is_running: false,
            audio_manager,
            new_item: String::new(),
            rx,
            hook_handle,
            active_sessions,
            last_refresh: std::time::Instant::now(),
            last_audio_enforcement: std::time::Instant::now(),
            show_sponsor: false,
            show_help: false,
            wechat_qr,
            alipay_qr,
        }
    }

    fn refresh_sessions(&mut self) {
        if let Ok(sessions) = self.audio_manager.get_active_sessions() {
            self.active_sessions = sessions;
        }
        self.last_refresh = std::time::Instant::now();
    }

    fn update_audio(&self) {
        let foreground_pid = utils::get_foreground_pid();
        let _ = self.audio_manager.update_mute_status(
            &self.config.list,
            self.config.is_whitelist,
            self.is_running,
            foreground_pid,
        );
    }
}

impl Drop for SilencerApp {
    fn drop(&mut self) {
        if let Some(handle) = self.hook_handle {
            unsafe {
                let _ = windows::Win32::UI::Accessibility::UnhookWinEvent(handle);
            }
        }
        // 在应用退出时尝试将我们修改过的会话恢复到原始静音状态
        let _ = self.audio_manager.restore_saved_states();
    }
}

impl eframe::App for SilencerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Header Bar
        egui::TopBottomPanel::top("header_bar").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("一款不智能的静音软件的软件").strong());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    // Author Info - Collapsible/Clickable
                    ui.menu_button("👤 作者信息", |ui| {
                        ui.set_min_width(180.0);
                        ui.vertical(|ui| {
                            ui.add(egui::Label::new(egui::RichText::new("作者: Pai").strong()).selectable(false));
                            ui.add(egui::Label::new(egui::RichText::new("邮箱: 1421493444@qq.com").size(11.0)).selectable(false));
                        });
                    });
                    ui.separator();
                    // Help Button
                    if ui.button("📖 使用说明").clicked() {
                        self.show_help = !self.show_help;
                    }
                    ui.separator();
                    // Project Link
                    ui.hyperlink_to("项目地址", "https://github.com/Part01-Pai/Silencer-rs/releases");
                    ui.separator();
                    // Sponsor Button (milk tea)
                    if ui.button("请你喝杯奶茶 O◡oಣ").clicked() {
                        self.show_sponsor = !self.show_sponsor;
                    }
                });
            });
            ui.add_space(5.0);
        });

        if self.show_sponsor {
            egui::Window::new("请你喝杯奶茶")
                .open(&mut self.show_sponsor)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.label("如果此项目能帮助到您，我万分荣幸，或者您愿意请我喝杯奶茶 O◡oಣ");
                        ui.add_space(12.0);
                        
                        ui.columns(2, |columns| {
                            columns[0].vertical_centered(|ui| {
                                ui.label("微信奶茶 🍦");
                                if let Some(texture) = &self.wechat_qr {
                                    ui.add(egui::Image::from_texture(texture).max_width(120.0));
                                } else {
                                    ui.label("图片加载失败");
                                }
                            });
                            columns[1].vertical_centered(|ui| {
                                ui.label("支付宝奶茶 🍰");
                                if let Some(texture) = &self.alipay_qr {
                                    ui.add(egui::Image::from_texture(texture).max_width(120.0));
                                } else {
                                    ui.label("图片加载失败");
                                }
                            });
                        });
                        
                        ui.add_space(10.0);
                        ui.label("您的支持是我持续开发的动力！");
                    });
                });
        }

        if self.show_help {
            egui::Window::new("📖 使用操作讲解")
                .open(&mut self.show_help)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.collapsing("💡 核心功能介绍", |ui| {
                            ui.label("本软件可以根据窗口的前后台状态，自动控制音频合成器的静音开关。");
                            ui.label("🚫 黑名单模式：列表中的应用在后台时静音。");
                            ui.label("✅ 白名单模式：除列表和前台应用外，全部静音。");
                        });

                        ui.add_space(10.0);

                        ui.collapsing("🔍 添加应用 vs 添加实例", |ui| {
                            ui.strong("1. 📦 添加应用 (按进程名)");
                            ui.label("🎯 范围：控制该软件的所有窗口。");
                            ui.label("💡 场景：适合普通软件。只要你在用该软件的任何一个窗口，它就不会静音。");
                            ui.label("💾 持久性：重启软件后依然有效。");
                            
                            ui.add_space(5.0);
                            
                            ui.strong("2. 🆔 添加实例 (按 PID)");
                            ui.label("🎯 范围：仅控制当前选中的这一个特定窗口。");
                            ui.label("💡 场景：适合多开游戏。可以实现“大号有声，小号静音”的精准控制。");
                            ui.label("⏳ 持久性：仅本次运行有效（PID 重启会变）。");
                        });
                    });
                });
        }

        if self.last_refresh.elapsed().as_secs() >= 2 {
            self.refresh_sessions();
        }

        let mut event_triggered = false;
        while self.rx.try_recv().is_ok() {
            event_triggered = true;
        }

        // 核心修复：
        // 1. 增加 50ms 的防抖（Debounce），防止极速切屏时的性能抖动
        // 2. 增加 200ms 的周期性强制同步，确保即使错过事件也能恢复正确状态
        let now = std::time::Instant::now();
        if self.is_running {
            let elapsed = now.duration_since(self.last_audio_enforcement).as_millis();
            if (event_triggered && elapsed >= 50) || elapsed >= 200 {
                self.update_audio();
                self.last_audio_enforcement = now;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(5.0);
            // Top Control Bar
            egui::Frame::none()
                .fill(ui.visuals().widgets.noninteractive.bg_fill)
                .rounding(10.0)
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let button_text = if self.is_running { "停止运行" } else { "开始运行" };
                        let button_color = if self.is_running { egui::Color32::from_rgb(200, 50, 50) } else { egui::Color32::from_rgb(50, 150, 50) };
                        
                        if ui.add(egui::Button::new(egui::RichText::new(button_text).color(egui::Color32::WHITE).strong())
                            .fill(button_color)
                            .min_size(egui::vec2(100.0, 35.0))).clicked() {
                            self.is_running = !self.is_running;
                            self.update_audio();
                        }

                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(format!("状态: {}", if self.is_running { "正在运行" } else { "已停止" })).size(16.0));
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 刷新列表").clicked() {
                                self.refresh_sessions();
                            }
                        });
                    });
                });

            ui.add_space(15.0);

            // Mode Selection
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("工作模式:").size(16.0));
                ui.add_space(10.0);
                if ui.selectable_label(!self.config.is_whitelist, "🚫 黑名单模式").clicked() {
                    self.config.is_whitelist = false;
                    self.update_audio();
                }
                ui.add_space(5.0);
                if ui.selectable_label(self.config.is_whitelist, "✅ 白名单模式").clicked() {
                    self.config.is_whitelist = true;
                    self.update_audio();
                }
            });

            ui.add_space(15.0);

            // Main Content Area
            ui.columns(2, |columns| {
                // Left Column: Active Sessions
                columns[0].vertical(|ui| {
                    ui.label(egui::RichText::new("活跃音频会话").strong().size(16.0));
                    ui.add_space(5.0);
                    
                    let mut to_add = None;
                    egui::ScrollArea::vertical()
                        .id_salt("active_sessions")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for session in &self.active_sessions {
                                egui::Frame::none()
                                    .fill(ui.visuals().widgets.inactive.bg_fill)
                                    .rounding(8.0)
                                    .inner_margin(10.0)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&session.display_name).strong());
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.label(egui::RichText::new(format!("PID: {}", session.pid)).size(10.0).color(egui::Color32::GRAY));
                                                });
                                            });
                                            
                                            if !session.window_title.is_empty() {
                                                ui.label(egui::RichText::new(&session.window_title).size(11.0).color(egui::Color32::LIGHT_GRAY));
                                            }
                                            
                                            ui.add_space(5.0);
                                            ui.horizontal(|ui| {
                                                if ui.button("📦 添加应用").clicked() {
                                                    to_add = Some(session.name.clone());
                                                }
                                                if ui.button("🆔 添加实例").clicked() {
                                                    to_add = Some(format!("{} [{}]", session.name, session.pid));
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(8.0);
                            }
                        });
                    if let Some(item) = to_add {
                        self.config.list.insert(item);
                        self.update_audio();
                    }
                });

                // Right Column: Mute List
                columns[1].vertical(|ui| {
                    ui.label(egui::RichText::new("管理列表").strong().size(16.0));
                    ui.add_space(5.0);

                    let mut to_remove = None;
                    egui::ScrollArea::vertical()
                        .id_salt("mute_list")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for item in &self.config.list {
                                ui.horizontal(|ui| {
                                    egui::Frame::none()
                                        .fill(ui.visuals().widgets.inactive.bg_fill)
                                        .rounding(5.0)
                                        .inner_margin(5.0)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.label(item);
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button("🗑").clicked() {
                                                        to_remove = Some(item.clone());
                                                    }
                                                });
                                            });
                                        });
                                });
                                ui.add_space(4.0);
                            }
                        });

                    if let Some(item) = to_remove {
                        self.config.list.remove(&item);
                        self.update_audio();
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);
                    ui.label("手动添加:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_item);
                        if ui.button("添加").clicked() && !self.new_item.is_empty() {
                            self.config.list.insert(self.new_item.clone());
                            self.new_item.clear();
                            self.update_audio();
                        }
                    });
                });
            });
        });
        
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(config_str) = serde_json::to_string(&self.config) {
            storage.set_string(eframe::APP_KEY, config_str);
        }
    }
}

fn main() -> eframe::Result {
    // 构建视口并使用编译时内嵌的 ICO（通过 include_bytes! 保证在可执行文件中存在）
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_min_inner_size([600.0, 450.0]);

    // 使用编译时包含的 ico 数据，确保窗口图标在所有运行环境下一致
    // 如果仓库根目录有 silencer-rs.ico，该文件会在编译时被包含进可执行文件
    const EMBEDDED_ICO: &[u8] = include_bytes!("../../silencer-rs.ico");
    if let Ok(img) = image::load_from_memory_with_format(EMBEDDED_ICO, image::ImageFormat::Ico) {
        let rgba = img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();
        let raw = rgba.into_raw();
        let icon = egui::IconData { rgba: raw, width, height };
        viewport_builder = viewport_builder.with_icon(icon);
    }

    let options = eframe::NativeOptions { viewport: viewport_builder, ..Default::default() };
    eframe::run_native(
        "Silencer-rs",
        options,
        Box::new(|cc| Ok(Box::new(SilencerApp::new(cc)))),
    )
}
