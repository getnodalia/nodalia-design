//! The update windows for egui apps (feature `egui`).
//!
//! One [`Updater`] per app. The app calls [`Updater::check_at_start`] once it is up,
//! [`Updater::check_now`] from its "Check for Updates…" command, and [`Updater::show`] every
//! frame; `show` returns what the app must do: open the release page, or exit so the new version
//! can take over. Checks, downloads and installs run on their own threads.
//!
//! Nothing is installed without a click, and never while `safe_to_close` is false (unsaved work,
//! a dialog, a job running): the downloaded package then waits until it is. Installs that a
//! package manager owns (Flatpak, `.deb`, `.rpm`, the AUR, a dev build), portable copies and
//! copies in a folder this user cannot write are pointed at the release page instead. An automatic check stays silent unless there is
//! something to install.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{App, Check, Failure, Language, Release};

/// An automatic check is skipped if one ran less than this long ago.
const EVERY: u64 = 24 * 60 * 60;

/// How long after start-up the automatic check waits, so it never competes with the first frames.
const STARTUP_DELAY: Duration = Duration::from_secs(20);

const WIDTH: f32 = 420.0;

/// The notes scroll past this, so the buttons under them are always on screen.
const NOTES_HEIGHT: f32 = 260.0;

/// What the app must do after [`Updater::show`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// Open this page in the browser.
    OpenUrl(String),
    /// The new version is installed and starting, or a helper waits for this process to end:
    /// exit now with `std::process::exit(0)`, not through a close that something may veto.
    Exit,
}

/// Where the flow is.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    /// Looking for a release; `user` when someone asked, which shows a window.
    Checking {
        user: bool,
    },
    UpToDate,
    CheckFailed(String),
    /// The question: install this release?
    Offer(Release),
    Downloading {
        release: Release,
        received: u64,
        total: Option<u64>,
    },
    /// Downloaded and verified, waiting for the app to be safe to close.
    Ready {
        release: Release,
        package: PathBuf,
    },
    Installing(Release),
    InstallFailed(String),
    /// The last macOS install did not go through, read at start.
    LastFailed(Failure),
}

enum Msg {
    Checked {
        user: bool,
        result: Result<Check, String>,
        /// The typed error behind a failed check from GitHub, worded in the window's language.
        error: Option<crate::Error>,
    },
    Progress(u64, Option<u64>),
    /// The version downloaded, so a result only completes the download that asked for it.
    Downloaded(String, Result<PathBuf, String>, Option<crate::Error>),
    Installed(Result<(), String>, Option<crate::Error>),
}

/// Where a check gets its answer: GitHub, unless an app's tests stand in for it.
pub type Source = Arc<dyn Fn() -> Result<Check, String> + Send + Sync>;

/// The update flow of one egui app.
pub struct Updater {
    app: App,
    status: Status,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    started: bool,
    accent: Option<egui::Color32>,
    /// Where the time of the last automatic check is kept; `None` keeps it nowhere (tests).
    stamp: Option<PathBuf>,
    /// Stands in for GitHub (tests); `None` asks GitHub.
    source: Option<Source>,
    startup_delay: Duration,
    /// The typed error behind the failure on screen, if it came from this crate: it is worded in
    /// the language the window is drawn in, which the threads do not know.
    error: Option<crate::Error>,
}

impl Updater {
    pub fn new(app: App) -> Self {
        let (tx, rx) = mpsc::channel();
        let stamp = Some(crate::update_dir(&app).join("last-check"));
        Self {
            app,
            status: Status::Idle,
            tx,
            rx,
            started: false,
            accent: None,
            stamp,
            source: None,
            startup_delay: STARTUP_DELAY,
            error: None,
        }
    }

    /// Colours the main button in the app's accent instead of the theme's selection colour.
    pub fn with_accent(mut self, rgb: [u8; 3]) -> Self {
        self.accent = Some(egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        self
    }

    /// Keeps the time of the last automatic check in this file instead of the update folder.
    pub fn with_stamp(mut self, path: Option<PathBuf>) -> Self {
        self.stamp = path;
        self
    }

    /// Answers checks from `source` instead of GitHub, for an app's tests. The check at start then
    /// runs whatever the install, since a test does not run from an AppImage.
    pub fn with_source(mut self, source: Source) -> Self {
        self.source = Some(source);
        self
    }

    /// How long the check at start waits after the first frame (20 s unless changed).
    pub fn with_startup_delay(mut self, delay: Duration) -> Self {
        self.startup_delay = delay;
        self
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Whether a window is up. They are modal: the app should keep its keys from the document.
    pub fn is_open(&self) -> bool {
        !matches!(self.status, Status::Idle | Status::Checking { user: false })
    }

    /// Shows a release as if a check had found it. A download or install under way goes on.
    pub fn offer(&mut self, release: Release) {
        if matches!(
            self.status,
            Status::Downloading { .. } | Status::Installing(_)
        ) {
            return;
        }
        self.status = Status::Offer(release);
    }

    /// Puts the flow in this status without doing its work: for an app's tests and previews.
    pub fn set_status(&mut self, status: Status) {
        self.error = None;
        self.status = status;
    }

    /// Once per run, whenever it is called first: reports a failed macOS install, then checks for
    /// a release after a short wait, at most once a day, and only where the app can replace itself
    /// (for the rest the package manager says when there is something new).
    pub fn check_at_start(&mut self, ctx: &egui::Context) {
        if std::mem::replace(&mut self.started, true) {
            return;
        }
        if let Some(failure) = crate::take_last_failure(&self.app) {
            // The failure is what this run shows; the next start checks for a release.
            self.status = Status::LastFailed(failure);
            return;
        }
        let checks = self.source.is_some() || crate::install_kind(&self.app).checks_at_start();
        if !checks || !due(self.last_check(), now()) {
            return;
        }
        self.spawn_check(ctx, false, self.startup_delay);
    }

    /// Looks for a release now and says what it found, whatever it is.
    pub fn check_now(&mut self, ctx: &egui::Context) {
        if matches!(
            self.status,
            Status::Downloading { .. } | Status::Installing(_) | Status::Checking { user: true }
        ) {
            return;
        }
        self.status = Status::Checking { user: true };
        self.spawn_check(ctx, true, Duration::ZERO);
    }

    fn spawn_check(&mut self, ctx: &egui::Context, user: bool, delay: Duration) {
        if !user {
            self.status = Status::Checking { user: false };
        }
        let (app, tx, ctx) = (self.app.clone(), self.tx.clone(), ctx.clone());
        let source = self.source.clone();
        // An automatic check counts once it starts: one the app closed before is not skipped.
        let stamp = if user { None } else { self.stamp.clone() };
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            if let Some(stamp) = stamp {
                write_stamp(&stamp);
            }
            let (result, error) = match source {
                Some(source) => (source(), None),
                None => match crate::check(&app) {
                    Ok(check) => (Ok(check), None),
                    Err(e) => (Err(e.to_string()), Some(e)),
                },
            };
            let _ = tx.send(Msg::Checked {
                user,
                result,
                error,
            });
            ctx.request_repaint();
        });
    }

    fn spawn_download(&mut self, ctx: &egui::Context, release: Release) {
        let (app, tx, ctx) = (self.app.clone(), self.tx.clone(), ctx.clone());
        self.status = Status::Downloading {
            release: release.clone(),
            received: 0,
            total: None,
        };
        std::thread::spawn(move || {
            let mut last = u64::MAX;
            let result = crate::download(&app, &release, |received, total| {
                // Whole percent when the size is known, whole MiB when not.
                let step = match total {
                    Some(total) if total > 0 => received.saturating_mul(100) / total,
                    _ => received >> 20,
                };
                if step != last {
                    last = step;
                    let _ = tx.send(Msg::Progress(received, total));
                    ctx.request_repaint();
                }
            });
            let error = result.as_ref().err().cloned();
            let _ = tx.send(Msg::Downloaded(
                release.version.clone(),
                result.map_err(|e| e.to_string()),
                error,
            ));
            ctx.request_repaint();
        });
    }

    fn spawn_install(&mut self, ctx: &egui::Context, release: Release, package: PathBuf) {
        let (app, tx, ctx) = (self.app.clone(), self.tx.clone(), ctx.clone());
        self.status = Status::Installing(release);
        std::thread::spawn(move || {
            let result = crate::apply(&app, &package, || Ok(()));
            let error = result.as_ref().err().cloned();
            let _ = tx.send(Msg::Installed(result.map_err(|e| e.to_string()), error));
            ctx.request_repaint();
        });
    }

    /// Takes what the threads sent. [`Updater::show`] does it too; this is for apps that ask
    /// [`Updater::is_open`] before drawing.
    #[must_use = "an `Action::Exit` dropped here leaves the window on \"Installing…\" for good"]
    pub fn poll(&mut self) -> Action {
        let mut action = Action::None;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Checked {
                    user,
                    result,
                    error,
                } => {
                    // Only the check still pending gets an answer: a late one must not replace a
                    // download or a window the person has moved on to.
                    if self.status != (Status::Checking { user }) {
                        continue;
                    }
                    self.error = error;
                    self.status = match result {
                        Ok(Check::Available(release)) => Status::Offer(release),
                        Ok(Check::UpToDate) if user => Status::UpToDate,
                        Err(why) if user => Status::CheckFailed(why),
                        // An automatic check that found nothing, or failed, says nothing.
                        _ if matches!(self.status, Status::Checking { user: false }) => {
                            Status::Idle
                        }
                        _ => continue,
                    };
                }
                Msg::Progress(received, total) => {
                    if let Status::Downloading {
                        received: r,
                        total: t,
                        ..
                    } = &mut self.status
                    {
                        *r = received;
                        *t = total;
                    }
                }
                Msg::Downloaded(version, result, error) => {
                    let release = match &self.status {
                        Status::Downloading { release, .. } if release.version == version => {
                            release.clone()
                        }
                        _ => continue,
                    };
                    self.error = error;
                    self.status = match result {
                        Ok(package) => Status::Ready { release, package },
                        Err(why) => Status::InstallFailed(why),
                    };
                }
                Msg::Installed(result, error) => match result {
                    Ok(()) => action = Action::Exit,
                    Err(why) => {
                        self.error = error;
                        self.status = Status::InstallFailed(why);
                    }
                },
            }
        }
        action
    }

    /// Draws the window for the current status and returns what the app must do.
    ///
    /// `safe_to_close` is whether the app could quit now without losing anything: a downloaded
    /// update installs only then, and waits with an explanation otherwise.
    #[must_use = "the app must open the URL or exit as the action says"]
    pub fn show(&mut self, ctx: &egui::Context, language: Language, safe_to_close: bool) -> Action {
        let polled = self.poll();
        if polled != Action::None {
            return polled;
        }
        let words = Words::new(language);
        let kind = crate::install_kind(&self.app);
        let in_place = kind.can_apply_in_place();
        let mut next: Option<Status> = None;
        let mut action = Action::None;
        let mut install: Option<(Release, PathBuf)> = None;
        let mut download: Option<Release> = None;
        let mut again = false;

        let (title, closable) = match &self.status {
            Status::Idle | Status::Checking { user: false } => return Action::None,
            Status::Checking { user: true } => (words.checking.to_owned(), true),
            Status::UpToDate => (words.up_to_date_title.to_owned(), true),
            Status::CheckFailed(_) => (words.check_failed_title.to_owned(), true),
            Status::Offer(release) => (words.available(self.app.name, &release.version), true),
            Status::Downloading { release, .. } => (words.downloading(&release.version), false),
            Status::Ready { release, .. } => (words.ready(&release.version), true),
            Status::Installing(release) => (words.installing(&release.version), false),
            Status::InstallFailed(_) | Status::LastFailed(_) => {
                (words.install_failed_title.to_owned(), true)
            }
        };

        let modal =
            egui::Modal::new(egui::Id::unique(("nodalia-update", self.app.repo))).show(ctx, |ui| {
                ui.set_width(WIDTH);
                ui.heading(&title);
                ui.add_space(8.0);
                match &self.status {
                    Status::Checking { .. } => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(words.checking_body);
                        });
                    }
                    Status::UpToDate => {
                        ui.label(words.up_to_date(self.app.name, self.app.version));
                    }
                    Status::CheckFailed(why) | Status::InstallFailed(why) => match &self.error {
                        Some(error) => {
                            ui.label(words.error(error));
                        }
                        None => {
                            ui.label(why.as_str());
                        }
                    },
                    Status::LastFailed(failure) => {
                        ui.label(words.last_failed(*failure));
                    }
                    Status::Offer(release) => {
                        let blocks = crate::notes_blocks(&release.notes, language);
                        egui::ScrollArea::vertical()
                            .max_height(NOTES_HEIGHT)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                if blocks.is_empty() {
                                    ui.label(words.no_notes);
                                }
                                for (i, block) in blocks.iter().enumerate() {
                                    draw_block(ui, block, i == 0);
                                }
                            });
                        if !in_place {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(words.elsewhere(&kind)).weak());
                        } else if release.asset.is_none() {
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(words.page_only).weak());
                        }
                    }
                    Status::Downloading {
                        received, total, ..
                    } => {
                        let bar = match total {
                            Some(total) if *total > 0 => {
                                egui::ProgressBar::new((*received as f32 / *total as f32).min(1.0))
                                    .show_percentage()
                            }
                            _ => egui::ProgressBar::new(0.0).animate(true),
                        };
                        ui.add(bar);
                    }
                    Status::Ready { .. } => {
                        ui.label(if safe_to_close {
                            words.ready_body
                        } else {
                            words.wait_body
                        });
                    }
                    Status::Installing(_) => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(words.installing_body);
                        });
                    }
                    Status::Idle => {}
                }
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let accent = self.accent;
                    let primary = |ui: &mut egui::Ui, text: &str, enabled: bool| {
                        let mut button = egui::Button::new(text);
                        if let Some(fill) = accent {
                            button = button.fill(fill);
                        }
                        ui.add_enabled(enabled, button).clicked()
                    };
                    match &self.status {
                        Status::Offer(release) => {
                            // Known only by its page (GitHub's limit was spent): no package to
                            // install, so the page.
                            if in_place && release.asset.is_some() {
                                if primary(ui, words.install, true) {
                                    download = Some(release.clone());
                                }
                            } else if primary(ui, words.open_page, true) {
                                action = Action::OpenUrl(release.page.clone());
                                next = Some(Status::Idle);
                            }
                            if ui.button(words.later).clicked() {
                                next = Some(Status::Idle);
                            }
                        }
                        Status::Ready { release, package } => {
                            if primary(ui, words.restart, safe_to_close) {
                                install = Some((release.clone(), package.clone()));
                            }
                            if ui.button(words.later).clicked() {
                                next = Some(Status::Idle);
                            }
                        }
                        Status::CheckFailed(_) => {
                            if primary(ui, words.close, true) {
                                next = Some(Status::Idle);
                            }
                            if ui.button(words.try_again).clicked() {
                                again = true;
                            }
                        }
                        Status::Downloading { .. } | Status::Installing(_) => {}
                        _ => {
                            if primary(ui, words.close, true) {
                                next = Some(Status::Idle);
                            }
                        }
                    }
                });
            });
        if closable
            && modal.should_close()
            && next.is_none()
            && install.is_none()
            && download.is_none()
        {
            next = Some(Status::Idle);
        }

        if let Some(release) = download {
            self.spawn_download(ctx, release);
        } else if let Some((release, package)) = install {
            self.spawn_install(ctx, release, package);
        } else if again {
            self.check_now(ctx);
        } else if let Some(status) = next {
            self.status = status;
        }
        action
    }

    fn last_check(&self) -> u64 {
        self.stamp
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0)
    }
}

/// One block of release notes, left-aligned, with its emphasis.
fn draw_block(ui: &mut egui::Ui, block: &crate::Block, first: bool) {
    let (spans, prefix, heading) = match block {
        crate::Block::Heading(spans) => (spans, "", true),
        crate::Block::Bullet(spans) => (spans, "•  ", false),
        crate::Block::Paragraph(spans) => (spans, "", false),
    };
    if heading && !first {
        ui.add_space(6.0);
    }
    let style = ui.style().clone();
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let mut push = |text: egui::RichText| {
        text.append_to(
            &mut job,
            &style,
            egui::FontSelection::Default,
            egui::Align::Center,
        );
    };
    if !prefix.is_empty() {
        push(egui::RichText::new(prefix));
    }
    for span in spans {
        let mut text = egui::RichText::new(&span.text);
        if span.bold || heading {
            text = text.strong();
        }
        if span.italic {
            text = text.italics();
        }
        if span.code {
            text = text.code();
        }
        push(text);
    }
    ui.add(egui::Label::new(job).wrap());
}

fn write_stamp(path: &std::path::Path) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, now().to_string());
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Whether an automatic check is due.
fn due(last: u64, now: u64) -> bool {
    now.saturating_sub(last) >= EVERY
}

/// The windows' words, in English and Spanish.
struct Words {
    spanish: bool,
    checking: &'static str,
    checking_body: &'static str,
    up_to_date_title: &'static str,
    check_failed_title: &'static str,
    install_failed_title: &'static str,
    no_notes: &'static str,
    managed: &'static str,
    portable: &'static str,
    not_writable: &'static str,
    ready_body: &'static str,
    wait_body: &'static str,
    installing_body: &'static str,
    install: &'static str,
    open_page: &'static str,
    later: &'static str,
    restart: &'static str,
    close: &'static str,
    try_again: &'static str,
    page_only: &'static str,
}

impl Words {
    fn new(language: Language) -> Self {
        match language {
            Language::Spanish => Self {
                spanish: true,
                checking: "Buscando actualizaciones",
                checking_body: "Consultando la última versión…",
                up_to_date_title: "No hay actualizaciones",
                check_failed_title: "No se pudo buscar actualizaciones",
                install_failed_title: "No se pudo instalar la actualización",
                no_notes: "Hay una versión nueva disponible.",
                managed: "Esta instalación la actualiza el gestor de paquetes del sistema. Descarga la versión nueva desde la página de la versión o actualiza con tu gestor.",
                portable: "Esta copia es portable. Descarga la versión nueva desde la página de la versión y sustituye la carpeta.",
                not_writable: "La app no puede sustituirse donde está (una carpeta sin permiso de escritura, la imagen de disco o una copia que macOS abre desde otra ubicación). Descarga la versión nueva desde la página de la versión; en macOS, arrastra la app a Aplicaciones.",
                ready_body: "La versión nueva está descargada y verificada. La app se cerrará y se abrirá de nuevo.",
                wait_body: "La versión nueva está descargada y verificada. Guarda tu trabajo, cierra los diálogos y espera a que terminen las tareas en curso para reiniciar.",
                installing_body: "Instalando y reiniciando…",
                install: "Instalar y reiniciar",
                open_page: "Abrir la página de la versión",
                later: "Más tarde",
                restart: "Reiniciar ahora",
                close: "Cerrar",
                try_again: "Reintentar",
                page_only: "GitHub limita las consultas desde tu red durante un rato, así que no se ha podido preparar la instalación automática. Descarga la versión nueva desde su página, o vuelve a buscar más tarde para instalarla desde aquí.",
            },
            Language::English => Self {
                spanish: false,
                checking: "Checking for Updates",
                checking_body: "Looking for the latest version…",
                up_to_date_title: "No Updates",
                check_failed_title: "Could Not Check for Updates",
                install_failed_title: "Could Not Install the Update",
                no_notes: "A new version is available.",
                managed: "Your system's package manager updates this installation. Download the new version from its release page, or update with your package manager.",
                portable: "This is a portable copy. Download the new version from its release page and replace the folder.",
                not_writable: "The app cannot replace itself where it is (a folder you cannot write to, the disk image, or a copy macOS opens from elsewhere). Download the new version from its release page; on macOS, drag the app to Applications.",
                ready_body: "The new version is downloaded and verified. The app will close and open again.",
                wait_body: "The new version is downloaded and verified. Save your work, close any dialogs and let running jobs finish to restart.",
                installing_body: "Installing and restarting…",
                install: "Install and Restart",
                open_page: "Open Release Page",
                later: "Later",
                restart: "Restart Now",
                close: "Close",
                try_again: "Try Again",
                page_only: "GitHub is limiting requests from your network for a while, so the automatic install could not be prepared. Download the new version from its page, or check again later to install it from here.",
            },
        }
    }

    fn available(&self, app: &str, version: &str) -> String {
        if self.spanish {
            format!("{app} {version} está disponible")
        } else {
            format!("{app} {version} Is Available")
        }
    }

    fn up_to_date(&self, app: &str, version: &str) -> String {
        if self.spanish {
            format!("{app} {version} es la versión más reciente.")
        } else {
            format!("{app} {version} is the latest version.")
        }
    }

    fn downloading(&self, version: &str) -> String {
        if self.spanish {
            format!("Descargando {version}")
        } else {
            format!("Downloading {version}")
        }
    }

    fn ready(&self, version: &str) -> String {
        if self.spanish {
            format!("{version} está lista para instalarse")
        } else {
            format!("{version} Is Ready to Install")
        }
    }

    fn installing(&self, version: &str) -> String {
        if self.spanish {
            format!("Instalando {version}")
        } else {
            format!("Installing {version}")
        }
    }

    /// A failure from this crate, in the window's language: what happened and what to do.
    fn error(&self, error: &crate::Error) -> String {
        use crate::Error as E;
        let es = self.spanish;
        match error {
            E::Network(_) if es => "No se pudo conectar con GitHub. Comprueba la conexión a Internet y vuelve a intentarlo.".into(),
            E::Network(_) => "Could not connect to GitHub. Check your internet connection and try again.".into(),
            E::RateLimited { reset } => {
                let minutes = reset.map(|r| r.saturating_sub(now()).div_ceil(60).max(1));
                match (es, minutes) {
                    (true, Some(1)) => "GitHub limita cuántas veces se pueden buscar actualizaciones desde tu red, y el límite se ha agotado por ahora. Vuelve a intentarlo dentro de un minuto.".into(),
                    (true, Some(m)) => format!("GitHub limita cuántas veces se pueden buscar actualizaciones desde tu red, y el límite se ha agotado por ahora. Vuelve a intentarlo dentro de {m} minutos."),
                    (true, None) => "GitHub limita cuántas veces se pueden buscar actualizaciones desde tu red, y el límite se ha agotado por ahora. Vuelve a intentarlo dentro de un rato.".into(),
                    (false, Some(1)) => "GitHub limits how often updates can be checked from your network, and the limit is spent for now. Try again in a minute.".into(),
                    (false, Some(m)) => format!("GitHub limits how often updates can be checked from your network, and the limit is spent for now. Try again in {m} minutes."),
                    (false, None) => "GitHub limits how often updates can be checked from your network, and the limit is spent for now. Try again in a while.".into(),
                }
            }
            E::BadResponse if es => "GitHub respondió algo inesperado. Vuelve a intentarlo más tarde.".into(),
            E::BadResponse => "GitHub sent an unexpected answer. Try again later.".into(),
            E::NoVersion if es => "La última versión publicada no tiene número de versión.".into(),
            E::NoVersion => "The latest release has no version number.".into(),
            E::NoPackage(v) if es => format!("Hay una versión nueva ({v}), pero aún no tiene paquete para este sistema. Vuelve a buscar en un rato."),
            E::NoPackage(v) => format!("A new version ({v}) is out, but it has no package for this system yet. Check again in a while."),
            E::NoDigest if es => "La versión no publica la huella SHA-256 del paquete, así que no se instala sin verificar.".into(),
            E::NoDigest => "The release does not publish the package's SHA-256, so it is not installed unverified.".into(),
            E::Checksum if es => "La descarga no coincide con su huella SHA-256 y se ha descartado. Vuelve a intentarlo.".into(),
            E::Checksum => "The download does not match its SHA-256 and was discarded. Try again.".into(),
            E::NotUpdatable if es => "Esta instalación no puede actualizarse desde la app. Descarga la versión nueva desde su página.".into(),
            E::NotUpdatable => "This installation cannot be updated from the app. Download the new version from its page.".into(),
            E::Refused(why) | E::Io(why) | E::Signature(why) => why.clone(),
        }
    }

    fn elsewhere(&self, kind: &crate::InstallKind) -> &'static str {
        match kind {
            crate::InstallKind::Portable(_) => self.portable,
            crate::InstallKind::NotWritable(_) => self.not_writable,
            _ => self.managed,
        }
    }

    fn last_failed(&self, failure: Failure) -> &'static str {
        match (self.spanish, failure) {
            (true, Failure::StillRunning) => "La app no se cerró a tiempo; no se ha cambiado nada.",
            (true, Failure::Mount) => "No se pudo abrir la imagen de disco de la actualización.",
            (true, Failure::Contents) => "La imagen de disco no contiene esta app.",
            (true, Failure::Copy) => "No se pudo copiar la versión nueva junto a la instalada.",
            (true, Failure::Replace) => "macOS no permitió sustituir la app. Permítelo en Ajustes del Sistema → Privacidad y seguridad → Gestión de apps.",
            (true, Failure::Install) => "El instalador no pudo instalar la versión nueva; sigue la versión anterior.",
            (true, Failure::Cancelled) => "Se canceló la instalación; no se ha cambiado nada.",
            (true, _) => "La última actualización no se instaló.",
            (false, Failure::StillRunning) => "The app did not close in time; nothing was changed.",
            (false, Failure::Mount) => "The update's disk image could not be opened.",
            (false, Failure::Contents) => "The disk image does not contain this app.",
            (false, Failure::Copy) => "The new version could not be copied next to the installed one.",
            (false, Failure::Replace) => "macOS did not allow replacing the app. Allow it in System Settings → Privacy & Security → App Management.",
            (false, Failure::Install) => "The installer could not install the new version; the previous one stays.",
            (false, Failure::Cancelled) => "The installation was cancelled; nothing was changed.",
            (false, _) => "The last update was not installed.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: App = App {
        name: "Prueba",
        repo: "danielmigueltejedor/prueba",
        bundle: "Prueba.app",
        version: "1.0.0",
        edition: None,
    };

    fn release() -> Release {
        Release {
            tag: "v2.0.0".into(),
            version: "2.0.0".into(),
            notes: "**Español**\n\n- Nuevo.\n\n**English**\n\n- New.".into(),
            page: "https://github.com/danielmigueltejedor/prueba/releases/tag/v2.0.0".into(),
            asset: None,
        }
    }

    fn frame(updater: &mut Updater, ctx: &egui::Context, events: Vec<egui::Event>) -> Action {
        let mut action = Action::None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                action = updater.show(ui.ctx(), Language::Spanish, true);
            },
        );
        output.textures_delta.clear();
        action
    }

    fn escape() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn a_stand_in_source_answers_the_check_at_start() {
        let source: Source = Arc::new(|| Ok(Check::Available(release())));
        let mut updater = Updater::new(APP)
            .with_stamp(None)
            .with_source(source)
            .with_startup_delay(Duration::ZERO);
        let ctx = egui::Context::default();
        updater.check_at_start(&ctx);
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while updater.status() != &Status::Offer(release()) {
            assert!(
                std::time::Instant::now() < deadline,
                "{:?}",
                updater.status()
            );
            let _ = updater.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        updater.check_at_start(&ctx);
        assert_eq!(updater.status(), &Status::Offer(release()), "once per run");
    }

    #[test]
    fn a_check_is_due_once_a_day() {
        assert!(due(0, 1_000_000));
        assert!(!due(1_000_000, 1_000_000 + EVERY - 1));
        assert!(due(1_000_000, 1_000_000 + EVERY));
        assert!(
            !due(5, 1),
            "a clock that went backwards waits rather than checking"
        );
    }

    #[test]
    fn idle_shows_nothing_and_holds_nothing() {
        let mut updater = Updater::new(APP).with_stamp(None);
        let ctx = egui::Context::default();
        assert!(!updater.is_open());
        assert_eq!(frame(&mut updater, &ctx, vec![]), Action::None);
        assert_eq!(updater.status(), &Status::Idle);
    }

    #[test]
    fn the_offer_is_modal_and_escape_closes_it() {
        let mut updater = Updater::new(APP).with_stamp(None);
        let ctx = egui::Context::default();
        updater.offer(release());
        assert!(updater.is_open());
        assert_eq!(frame(&mut updater, &ctx, vec![]), Action::None);
        assert!(updater.is_open());
        frame(&mut updater, &ctx, vec![escape()]);
        assert_eq!(updater.status(), &Status::Idle);
    }

    #[test]
    fn a_download_cannot_be_dismissed() {
        let mut updater = Updater::new(APP).with_stamp(None);
        let ctx = egui::Context::default();
        updater.status = Status::Downloading {
            release: release(),
            received: 5,
            total: Some(10),
        };
        frame(&mut updater, &ctx, vec![escape()]);
        assert!(matches!(updater.status(), Status::Downloading { .. }));
    }

    #[test]
    fn an_automatic_check_that_finds_nothing_or_fails_stays_silent() {
        let mut updater = Updater::new(APP).with_stamp(None);
        for result in [Ok(Check::UpToDate), Err("offline".to_owned())] {
            updater.status = Status::Checking { user: false };
            let _ = updater.tx.send(Msg::Checked {
                user: false,
                result,
                error: None,
            });
            assert_eq!(updater.poll(), Action::None);
            assert_eq!(updater.status(), &Status::Idle);
        }
    }

    #[test]
    fn a_check_someone_asked_for_answers_every_outcome() {
        let mut updater = Updater::new(APP).with_stamp(None);
        updater.status = Status::Checking { user: true };
        let _ = updater.tx.send(Msg::Checked {
            user: true,
            result: Ok(Check::UpToDate),
            error: None,
        });
        let _ = updater.poll();
        assert_eq!(updater.status(), &Status::UpToDate);

        updater.status = Status::Checking { user: true };
        let _ = updater.tx.send(Msg::Checked {
            user: true,
            result: Err("offline".into()),
            error: None,
        });
        let _ = updater.poll();
        assert_eq!(updater.status(), &Status::CheckFailed("offline".into()));

        updater.status = Status::Checking { user: true };
        let _ = updater.tx.send(Msg::Checked {
            user: true,
            result: Ok(Check::Available(release())),
            error: None,
        });
        let _ = updater.poll();
        assert_eq!(updater.status(), &Status::Offer(release()));
    }

    #[test]
    fn a_late_check_does_not_replace_a_download() {
        let mut updater = Updater::new(APP).with_stamp(None);
        let downloading = Status::Downloading {
            release: release(),
            received: 1,
            total: Some(10),
        };
        updater.status = downloading.clone();
        let _ = updater.tx.send(Msg::Checked {
            user: false,
            result: Ok(Check::Available(release())),
            error: None,
        });
        let _ = updater.poll();
        assert_eq!(updater.status(), &downloading);
        updater.offer(release());
        assert_eq!(
            updater.status(),
            &downloading,
            "an offer waits for the download"
        );
    }

    #[test]
    fn a_download_completes_only_the_release_that_asked_for_it() {
        let mut updater = Updater::new(APP).with_stamp(None);
        updater.status = Status::Downloading {
            release: release(),
            received: 1,
            total: None,
        };
        let _ = updater.tx.send(Msg::Downloaded(
            "0.0.1".into(),
            Ok(PathBuf::from("old")),
            None,
        ));
        let _ = updater.poll();
        assert!(matches!(updater.status(), Status::Downloading { .. }));
        let _ = updater.tx.send(Msg::Downloaded(
            release().version,
            Ok(PathBuf::from("new")),
            None,
        ));
        let _ = updater.poll();
        assert_eq!(
            updater.status(),
            &Status::Ready {
                release: release(),
                package: PathBuf::from("new")
            }
        );
    }

    #[test]
    fn the_day_counts_from_when_the_check_starts() {
        let dir = std::env::temp_dir().join(format!("nodalia-update-stamp-{}", std::process::id()));
        let stamp = dir.join("last-check");
        let _ = std::fs::remove_dir_all(&dir);
        let source: Source = Arc::new(|| Ok(Check::UpToDate));
        let mut updater = Updater::new(APP)
            .with_stamp(Some(stamp.clone()))
            .with_source(source)
            .with_startup_delay(Duration::from_secs(3600));
        updater.check_at_start(&egui::Context::default());
        assert!(!stamp.exists(), "nothing is recorded before the wait ends");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_finished_install_asks_the_app_to_exit_and_a_failed_one_says_why() {
        let mut updater = Updater::new(APP).with_stamp(None);
        updater.status = Status::Installing(release());
        let _ = updater.tx.send(Msg::Installed(Ok(()), None));
        assert_eq!(updater.poll(), Action::Exit);

        updater.status = Status::Installing(release());
        let _ = updater.tx.send(Msg::Installed(Err("locked".into()), None));
        assert_eq!(updater.poll(), Action::None);
        assert_eq!(updater.status(), &Status::InstallFailed("locked".into()));
    }

    #[test]
    fn a_downloaded_update_waits_while_the_app_is_not_safe_to_close() {
        let mut updater = Updater::new(APP).with_stamp(None);
        let ctx = egui::Context::default();
        updater.status = Status::Ready {
            release: release(),
            package: PathBuf::from("/nowhere"),
        };
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert_eq!(
                updater.show(ui.ctx(), Language::English, false),
                Action::None
            );
        });
        output.textures_delta.clear();
        assert!(matches!(updater.status(), Status::Ready { .. }));
    }

    #[test]
    fn a_failure_from_github_is_worded_in_the_window_s_language() {
        let es = Words::new(Language::Spanish);
        let en = Words::new(Language::English);
        let limited = crate::Error::RateLimited {
            reset: Some(now() + 600),
        };
        assert!(
            es.error(&limited).contains("dentro de 10 minutos"),
            "{}",
            es.error(&limited)
        );
        assert!(en.error(&limited).contains("in 10 minutes"));
        let soon = crate::Error::RateLimited { reset: Some(now()) };
        assert!(es.error(&soon).contains("dentro de un minuto"));
        let network = crate::Error::Network("http status: 500".into());
        assert!(es
            .error(&network)
            .starts_with("No se pudo conectar con GitHub"));
        assert!(!es.error(&network).contains("http status"), "no raw status");

        // The window shows the worded error, not the English one the status carries.
        let mut updater = Updater::new(APP).with_stamp(None);
        updater.status = Status::Checking { user: true };
        let _ = updater.tx.send(Msg::Checked {
            user: true,
            result: Err(limited.to_string()),
            error: Some(limited.clone()),
        });
        let _ = updater.poll();
        assert!(matches!(updater.status(), Status::CheckFailed(_)));
        assert_eq!(updater.error, Some(limited));
        // A status set by the app drops it.
        updater.set_status(Status::CheckFailed("from the app".into()));
        assert_eq!(updater.error, None);
    }
}
