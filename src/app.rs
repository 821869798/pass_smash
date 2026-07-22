//! Main application view and UI state.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState},
    progress::Progress,
    v_flex, *,
};
use parking_lot::Mutex;

use crate::crack::{
    CharsetOptions, CrackEngine, CrackJob, CrackProgress, EngineControl, FileKind, JobStatus,
    TargetFile,
};
use crate::i18n::{self, Locale, Msg};

/// Messages sent from background crack threads → UI.
#[derive(Clone)]
enum UiMsg {
    Progress(CrackProgress),
    BatchFinished,
}

pub struct PassSmashApp {
    jobs: Vec<CrackJob>,
    next_id: u64,

    // Charset options
    digits: bool,
    lowercase: bool,
    uppercase: bool,
    symbols: bool,
    min_len_input: Entity<InputState>,
    max_len_input: Entity<InputState>,
    custom_input: Entity<InputState>,

    // Runtime
    running: bool,
    dialog_open: bool,
    control: EngineControl,
    status_text: SharedString,
    search_space_hint: SharedString,
    auto_threads: usize,
    locale: Locale,

    // Cross-thread mailbox
    inbox: Arc<Mutex<Vec<UiMsg>>>,
}

impl PassSmashApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let locale = i18n::current();
        let min_len_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(Msg::MinLen.get(locale))
                .default_value("1")
        });
        let max_len_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(Msg::MaxLen.get(locale))
                .default_value("4")
        });
        let custom_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(Msg::CustomChars.get(locale))
        });

        let auto_threads = CrackEngine::auto_threads();
        let inbox: Arc<Mutex<Vec<UiMsg>>> = Arc::new(Mutex::new(Vec::new()));
        let inbox_poll = Arc::clone(&inbox);

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                let msgs: Vec<UiMsg> = {
                    let mut guard = inbox_poll.lock();
                    if guard.is_empty() {
                        Vec::new()
                    } else {
                        std::mem::take(&mut *guard)
                    }
                };
                if msgs.is_empty() {
                    continue;
                }
                let result = this.update(cx, |this, cx| {
                    for msg in msgs {
                        this.handle_msg(msg, cx);
                    }
                    cx.notify();
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            jobs: Vec::new(),
            next_id: 1,
            digits: true,
            lowercase: false,
            uppercase: false,
            symbols: false,
            min_len_input,
            max_len_input,
            custom_input,
            running: false,
            dialog_open: false,
            control: EngineControl::new(),
            status_text: Msg::Ready.get(locale).into(),
            search_space_hint: SharedString::default(),
            auto_threads,
            locale,
            inbox,
        }
    }

    fn set_locale(&mut self, locale: Locale, window: &mut Window, cx: &mut Context<Self>) {
        self.locale = locale;
        i18n::set(locale);
        self.min_len_input.update(cx, |input, cx| {
            input.set_placeholder(Msg::MinLen.get(locale), window, cx);
        });
        self.max_len_input.update(cx, |input, cx| {
            input.set_placeholder(Msg::MaxLen.get(locale), window, cx);
        });
        self.custom_input.update(cx, |input, cx| {
            input.set_placeholder(Msg::CustomChars.get(locale), window, cx);
        });
        if !self.running {
            self.status_text = Msg::Ready.get(locale).into();
        }
        self.refresh_search_hint(cx);
        cx.notify();
    }

    fn handle_msg(&mut self, msg: UiMsg, _cx: &mut Context<Self>) {
        let locale = self.locale;
        match msg {
            UiMsg::Progress(p) => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == p.job_id) {
                    job.tried = p.tried;
                    job.total = p.total.max(job.total);
                    job.rate = p.rate;
                    job.elapsed = p.elapsed;
                    job.status = p.status;
                    if let Some(pw) = p.found.clone() {
                        job.password = Some(pw);
                    }
                    if !p.message.is_empty() {
                        job.message = p.message.clone();
                    } else if !p.current_password.is_empty() && p.status == JobStatus::Running {
                        job.message = i18n::format_trying(locale, &p.current_password);
                    }
                }
                if p.finished {
                    if let Some(job) = self.jobs.iter().find(|j| j.id == p.job_id) {
                        self.status_text = i18n::format_job_finished(
                            locale,
                            job.id,
                            job.status.label(locale),
                            &job.message,
                        )
                        .into();
                    }
                } else {
                    self.status_text =
                        i18n::format_running(locale, p.rate, p.tried, p.total).into();
                }
            }
            UiMsg::BatchFinished => {
                self.running = false;
                let found = self
                    .jobs
                    .iter()
                    .filter(|j| j.status == JobStatus::Found)
                    .count();
                let total = self.jobs.len();
                self.status_text = i18n::format_batch_done(locale, found, total).into();
            }
        }
    }

    fn charset_options(&self, cx: &App) -> CharsetOptions {
        let min_len = self
            .min_len_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(1)
            .clamp(1, 12);
        let max_len = self
            .max_len_input
            .read(cx)
            .value()
            .parse::<usize>()
            .unwrap_or(4)
            .clamp(1, 12);
        CharsetOptions {
            min_len,
            max_len: max_len.max(min_len),
            digits: self.digits,
            lowercase: self.lowercase,
            uppercase: self.uppercase,
            symbols: self.symbols,
            custom: self.custom_input.read(cx).value().to_string(),
        }
    }

    fn refresh_search_hint(&mut self, cx: &App) {
        let opts = self.charset_options(cx);
        match opts.validate() {
            Ok(()) => {
                let total = opts.total_candidates();
                let charset_len = opts.build_charset().chars().count();
                let total_fmt = format_count(total);
                self.search_space_hint = i18n::format_search_hint(
                    self.locale,
                    charset_len,
                    &total_fmt,
                    opts.min_len,
                    opts.max_len,
                    self.auto_threads,
                )
                .into();
            }
            Err(e) => {
                self.search_space_hint = e.into();
            }
        }
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let mut added = 0;
        for path in paths {
            if !path.is_file() {
                continue;
            }
            if self.jobs.iter().any(|j| j.file.path == path) {
                continue;
            }
            let file = TargetFile::from_path(path);
            let id = self.next_id;
            self.next_id += 1;
            self.jobs.push(CrackJob::new(id, file));
            added += 1;
        }
        self.status_text =
            i18n::format_added(self.locale, added, self.jobs.len()).into();
        self.refresh_search_hint(cx);
        cx.notify();
    }

    fn open_files(&mut self, cx: &mut Context<Self>) {
        if self.dialog_open {
            return;
        }

        let locale = self.locale;
        let dialog = rfd::AsyncFileDialog::new()
            .set_title(i18n::open_dialog_title(locale))
            .add_filter(i18n::filter_supported(locale), &["zip", "7z", "rar", "pdf", "docx", "docm", "xlsx", "xlsm", "pptx", "pptm", "doc"])
            .add_filter(i18n::filter_all(locale), &["*"])
            .pick_files();

        self.dialog_open = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let paths = dialog.await.map_or_else(Vec::new, |files| {
                files
                    .into_iter()
                    .map(|file| file.path().to_path_buf())
                    .collect()
            });
            let _ = this.update(cx, |this, cx| {
                this.dialog_open = false;
                if paths.is_empty() {
                    cx.notify();
                } else {
                    this.add_paths(paths, cx);
                }
            });
        })
        .detach();
    }

    fn clear_jobs(&mut self, cx: &mut Context<Self>) {
        if self.running {
            self.status_text = Msg::CannotClearWhileRunning.get(self.locale).into();
            cx.notify();
            return;
        }
        self.jobs.clear();
        self.status_text = Msg::Cleared.get(self.locale).into();
        cx.notify();
    }

    fn remove_job(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        self.jobs.retain(|j| j.id != id);
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.control.cancel();
        self.status_text = Msg::Stop.get(self.locale).into();
        cx.notify();
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        if self.jobs.is_empty() {
            self.status_text = Msg::PleaseAddFiles.get(self.locale).into();
            cx.notify();
            return;
        }

        let opts = self.charset_options(cx);
        if let Err(e) = opts.validate() {
            self.status_text = e.into();
            cx.notify();
            return;
        }

        let threads = self.auto_threads;
        self.control = EngineControl::new();
        self.running = true;
        self.status_text = i18n::format_starting(self.locale, threads).into();

        for job in &mut self.jobs {
            if matches!(
                job.status,
                JobStatus::Pending
                    | JobStatus::Failed
                    | JobStatus::Cancelled
                    | JobStatus::Exhausted
            ) || (job.status == JobStatus::Found && job.password.is_none())
            {
                job.status = JobStatus::Pending;
                job.password = None;
                job.tried = 0;
                job.total = opts.total_candidates();
                job.rate = 0.0;
                job.message.clear();
            }
        }

        let queue: Vec<(u64, PathBuf, FileKind)> = self
            .jobs
            .iter()
            .filter(|j| j.status != JobStatus::Found)
            .filter(|j| j.file.kind.is_supported())
            .map(|j| (j.id, j.file.path.clone(), j.file.kind))
            .collect();

        let locale = self.locale;
        for job in &mut self.jobs {
            if !job.file.kind.is_supported() {
                job.status = JobStatus::Skipped;
                job.message = Msg::UnsupportedType.get(locale).to_string();
            } else if queue.iter().any(|(id, _, _)| *id == job.id) {
                job.status = JobStatus::Running;
                job.message = Msg::Queued.get(locale).to_string();
            }
        }

        let control = self.control.clone();
        let inbox = Arc::clone(&self.inbox);
        let opts = opts.clone();

        thread::spawn(move || {
            let engine = CrackEngine::new(control.clone(), threads);
            for (job_id, path, kind) in queue {
                if control.is_cancelled() {
                    break;
                }
                let inbox_p = Arc::clone(&inbox);
                let _result = engine.crack_file(job_id, path, kind, &opts, move |p| {
                    inbox_p.lock().push(UiMsg::Progress(p));
                });
            }
            inbox.lock().push(UiMsg::BatchFinished);
        });

        self.refresh_search_hint(cx);
        cx.notify();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.locale;
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .p_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().font_semibold().child("Pass Smash"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(Msg::AppSubtitle.get(locale)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("btn-lang")
                            .outline()
                            .label(Msg::LangSwitch.get(locale))
                            .disabled(self.running || self.dialog_open)
                            .on_click(cx.listener(|this, _, window, cx| {
                                let next = this.locale.toggle();
                                this.set_locale(next, window, cx);
                            })),
                    )
                    .child(
                        Button::new("btn-add")
                            .primary()
                            .label(Msg::AddFiles.get(locale))
                            .disabled(self.running || self.dialog_open)
                            .on_click(cx.listener(|this, _, _window, cx| this.open_files(cx))),
                    )
                    .child(
                        Button::new("btn-clear")
                            .outline()
                            .label(Msg::Clear.get(locale))
                            .disabled(self.running || self.dialog_open || self.jobs.is_empty())
                            .on_click(cx.listener(|this, _, _window, cx| this.clear_jobs(cx))),
                    ),
            )
    }

    fn render_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.locale;
        v_flex()
            .gap_3()
            .p_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().font_semibold().child(Msg::CrackParams.get(locale)))
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .flex_wrap()
                    .child(
                        Checkbox::new("chk-digits")
                            .label(Msg::Digits.get(locale))
                            .checked(self.digits)
                            .disabled(self.running)
                            .on_click(cx.listener(|this, checked, _window, cx| {
                                this.digits = *checked;
                                this.refresh_search_hint(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("chk-lower")
                            .label(Msg::Lowercase.get(locale))
                            .checked(self.lowercase)
                            .disabled(self.running)
                            .on_click(cx.listener(|this, checked, _window, cx| {
                                this.lowercase = *checked;
                                this.refresh_search_hint(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("chk-upper")
                            .label(Msg::Uppercase.get(locale))
                            .checked(self.uppercase)
                            .disabled(self.running)
                            .on_click(cx.listener(|this, checked, _window, cx| {
                                this.uppercase = *checked;
                                this.refresh_search_hint(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("chk-symbols")
                            .label(Msg::Symbols.get(locale))
                            .checked(self.symbols)
                            .disabled(self.running)
                            .on_click(cx.listener(|this, checked, _window, cx| {
                                this.symbols = *checked;
                                this.refresh_search_hint(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(div().text_sm().child(Msg::MinLen.get(locale)))
                    .child(
                        div()
                            .w(px(72.))
                            .child(Input::new(&self.min_len_input).disabled(self.running)),
                    )
                    .child(div().text_sm().child(Msg::MaxLen.get(locale)))
                    .child(
                        div()
                            .w(px(72.))
                            .child(Input::new(&self.max_len_input).disabled(self.running)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.custom_input).disabled(self.running)),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.search_space_hint.clone()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("btn-start")
                            .primary()
                            .label(if self.running {
                                Msg::Cracking.get(locale)
                            } else {
                                Msg::Start.get(locale)
                            })
                            .disabled(self.running || self.dialog_open || self.jobs.is_empty())
                            .on_click(cx.listener(|this, _, _window, cx| this.start(cx))),
                    )
                    .child(
                        Button::new("btn-stop")
                            .danger()
                            .label(Msg::Stop.get(locale))
                            .disabled(!self.running)
                            .on_click(cx.listener(|this, _, _window, cx| this.stop(cx))),
                    ),
            )
    }

    fn render_job_row(&self, job: &CrackJob, cx: &mut Context<Self>) -> impl IntoElement {
        let id = job.id;
        let locale = self.locale;
        let progress_pct = (job.progress_ratio() * 100.0).clamp(0.0, 100.0);
        let indeterminate = job.status == JobStatus::Running && job.tried == 0;
        let status_color = match job.status {
            JobStatus::Found => cx.theme().success,
            JobStatus::Failed | JobStatus::Skipped => cx.theme().danger,
            JobStatus::Running => cx.theme().primary,
            JobStatus::Cancelled => cx.theme().warning,
            _ => cx.theme().muted_foreground,
        };

        let password_text = job
            .password
            .as_ref()
            .map(|p| i18n::format_password(locale, p))
            .unwrap_or_else(|| job.message.clone());

        let progress_id = format!("pg-{id}-{}", job.tried);

        h_flex()
            .w_full()
            .gap_3()
            .p_3()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().font_semibold().child(job.file.name()))
                            .child(
                                div()
                                    .text_xs()
                                    .px_1()
                                    .rounded(px(4.))
                                    .bg(cx.theme().muted)
                                    .child(job.file.kind.label()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(status_color)
                                    .child(job.status.label(locale)),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(job.file.path.display().to_string()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div().flex_1().h(px(10.)).child(
                                    Progress::new(SharedString::from(progress_id))
                                        .value(if indeterminate { 0.0 } else { progress_pct })
                                        .loading(indeterminate)
                                        .color(cx.theme().primary),
                                ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{}/{}  {:.0}/s",
                                        job.tried, job.total, job.rate
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(if job.status == JobStatus::Found {
                                cx.theme().success
                            } else {
                                cx.theme().foreground
                            })
                            .child(password_text),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("rm-{id}")))
                    .ghost()
                    .label(Msg::Remove.get(locale))
                    .disabled(self.running)
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.remove_job(id, cx);
                    })),
            )
    }

    fn render_file_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.locale;
        let list = if self.jobs.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_3()
                .p_8()
                .child(
                    div()
                        .text_lg()
                        .text_color(cx.theme().muted_foreground)
                        .child(Msg::EmptyTitle.get(locale)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(Msg::EmptyHint.get(locale)),
                )
                .into_any_element()
        } else {
            v_flex()
                .w_full()
                .children(
                    self.jobs
                        .iter()
                        .map(|job| self.render_job_row(job, cx).into_any_element())
                        .collect::<Vec<_>>(),
                )
                .into_any_element()
        };

        v_flex()
            .flex_1()
            .w_full()
            .child(
                h_flex()
                    .w_full()
                    .px_4()
                    .py_2()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .font_semibold()
                            .child(i18n::format_file_list_title(locale, self.jobs.len())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(Msg::SupportedFormats.get(locale)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .id("job-list")
                    .overflow_y_scroll()
                    .child(list),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .px_4()
            .py_2()
            .items_center()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted)
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.status_text.clone()),
            )
    }
}

impl Render for PassSmashApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh_search_hint(cx);

        v_flex()
            .id("pass-smash-root")
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                if this.running {
                    this.status_text = Msg::CannotAddWhileRunning.get(this.locale).into();
                    cx.notify();
                    return;
                }
                if this.dialog_open {
                    return;
                }
                let list: Vec<PathBuf> = paths.paths().to_vec();
                this.add_paths(list, cx);
            }))
            .child(self.render_header(cx))
            .child(self.render_options(cx))
            .child(self.render_file_list(cx))
            .child(self.render_status_bar(cx))
    }
}

fn format_count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}
