//! F10-controlled Chrome trace capture for Bevy systems and re3bevy scopes.

use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::{App, AppExit, Update},
    ecs::{message::MessageWriter, resource::Resource, system::Res},
    input::{ButtonInput, keyboard::KeyCode},
    log::{
        BoxedLayer,
        tracing::{
            field::{Field, Visit},
            span::{Attributes, Id},
        },
        tracing_subscriber::{
            layer::{Context, Layer},
            registry::Registry,
        },
    },
};

#[derive(Resource, Clone)]
pub(super) struct ProfileCapture {
    state: Arc<Mutex<CaptureState>>,
    recording: Arc<AtomicBool>,
}

struct CaptureState {
    recording: bool,
    started_at: Option<Instant>,
    output_path: Option<PathBuf>,
    output_dir: PathBuf,
    writer_tx: mpsc::Sender<WriterCommand>,
    active: HashMap<(Id, u64), ActiveSpan>,
}

struct ActiveSpan {
    name: Arc<str>,
    category: &'static str,
    started_at: Instant,
    start_us: u128,
    thread_id: u64,
}

struct ProfileEvent {
    name: Arc<str>,
    category: &'static str,
    start_us: u128,
    duration_us: u128,
    thread_id: u64,
}

enum WriterCommand {
    Start(PathBuf),
    Span(ProfileEvent),
    Stop,
}

struct ProfileCaptureLayer {
    state: Arc<Mutex<CaptureState>>,
    recording: Arc<AtomicBool>,
}

#[derive(Clone)]
struct CapturedSpanName(Arc<str>);

#[derive(Default)]
struct SpanNameVisitor {
    name: Option<Arc<str>>,
}

impl Visit for SpanNameVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "name" {
            self.name = Some(Arc::from(value));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() != "name" || self.name.is_some() {
            return;
        }
        let value = format!("{value:?}");
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(&value);
        self.name = Some(Arc::from(value));
    }
}

thread_local! {
    static PROFILE_THREAD_ID: u64 = {
        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        hasher.finish()
    };
}

#[inline]
fn current_profile_thread_id() -> u64 {
    PROFILE_THREAD_ID.with(|thread_id| *thread_id)
}

#[inline]
fn noisy_executor_span(name: &str) -> bool {
    matches!(
        name,
        "multithreaded executor" | "par_for_each" | "system_commands"
    )
}

#[inline]
fn should_capture_span(target: &str, name: &str) -> bool {
    if noisy_executor_span(name) {
        return false;
    }
    if target.starts_with("re3bevy") || name.starts_with("re3bevy") {
        return true;
    }
    if matches!(
        name,
        "update"
            | "main app"
            | "Main"
            | "First"
            | "PreUpdate"
            | "Update"
            | "PostUpdate"
            | "Last"
            | "FixedMain"
            | "RunFixedMainLoop"
            | "RenderRecovery"
            | "Render"
            | "RenderExtractApp"
            | "ExtractSchedule"
            | "RenderGraph"
            | "Core3d"
            | "Core2d"
    ) {
        return true;
    }
    // Surface acquisition, pipeline compilation, asset preparation and UI
    // layout can dominate a frame too. Omitting their system spans left only
    // an unexplained gap inside Render, hiding the actual source of stalls.
    name.starts_with("bevy_")
        || matches!(name, "present_frames" | "main_render_schedule")
}

impl ProfileCapture {
    fn new(output_dir: PathBuf, writer_tx: mpsc::Sender<WriterCommand>) -> Self {
        Self {
            state: Arc::new(Mutex::new(CaptureState {
                recording: false,
                started_at: None,
                output_path: None,
                output_dir,
                writer_tx,
                active: HashMap::new(),
            })),
            recording: Arc::new(AtomicBool::new(false)),
        }
    }

    fn toggle(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.recording {
            self.recording.store(false, Ordering::Release);
            let stopped_at = Instant::now();
            let active = std::mem::take(&mut state.active);
            for (_, span) in active {
                let duration_us = stopped_at
                    .saturating_duration_since(span.started_at)
                    .as_micros();
                let _ = state.writer_tx.send(WriterCommand::Span(ProfileEvent {
                    name: span.name,
                    category: span.category,
                    start_us: span.start_us,
                    duration_us,
                    thread_id: span.thread_id,
                }));
            }
            state.recording = false;
            let _ = state.writer_tx.send(WriterCommand::Stop);
            let path = state.output_path.take();
            drop(state);
            if let Some(path) = path {
                eprintln!(
                    "F10 profiling stopped; saving {} in background.",
                    path.display()
                );
            }
            return;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros();
        let path = state
            .output_dir
            .join(format!("re3bevy-f10-{timestamp}.json"));
        if state
            .writer_tx
            .send(WriterCommand::Start(path.clone()))
            .is_err()
        {
            eprintln!("F10 profiling writer thread is unavailable.");
            return;
        }
        state.output_path = Some(path.clone());
        state.started_at = Some(Instant::now());
        state.active.clear();
        state.recording = true;
        self.recording.store(true, Ordering::Release);
        drop(state);
        eprintln!("F10 profiling started; press F10 again to save the capture.");
    }
}

fn profile_writer(rx: mpsc::Receiver<WriterCommand>) {
    let mut output: Option<(BufWriter<File>, PathBuf, bool)> = None;
    while let Ok(command) = rx.recv() {
        match command {
            WriterCommand::Start(path) => {
                let file =
                    fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new(".")))
                        .and_then(|()| File::create(&path));
                match file {
                    Ok(file) => {
                        let mut writer = BufWriter::new(file);
                        if writer.write_all(br#"{"traceEvents":["#).is_ok() {
                            output = Some((writer, path, false));
                        } else {
                            eprintln!("F10 profiling could not initialize the trace file.");
                        }
                    }
                    Err(error) => {
                        eprintln!("F10 profiling could not create {}: {error}", path.display())
                    }
                }
            }
            WriterCommand::Span(span) => {
                let Some((writer, _, has_event)) = output.as_mut() else {
                    continue;
                };
                if *has_event && writer.write_all(b",").is_err() {
                    continue;
                }
                let event = serde_json::json!({
                    "name": span.name.as_ref(),
                    "cat": span.category,
                    "ph": "X",
                    "ts": span.start_us,
                    "dur": span.duration_us,
                    "pid": std::process::id(),
                    "tid": span.thread_id,
                });
                if serde_json::to_writer(writer, &event).is_ok() {
                    *has_event = true;
                }
            }
            WriterCommand::Stop => {
                if let Some((mut writer, path, _)) = output.take() {
                    let _ = writer.write_all(b"]}");
                    let _ = writer.flush();
                    eprintln!("F10 profiling saved: {}", path.display());
                }
            }
        }
    }
}

impl Layer<Registry> for ProfileCaptureLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, Registry>) {
        let mut visitor = SpanNameVisitor::default();
        attrs.record(&mut visitor);
        let Some(name) = visitor.name else {
            return;
        };
        let Some(span_ref) = ctx.span(id) else {
            return;
        };
        span_ref.extensions_mut().insert(CapturedSpanName(name));
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, Registry>) {
        if !self.recording.load(Ordering::Acquire) {
            return;
        }
        let Some(span_ref) = ctx.span(id) else {
            return;
        };
        let metadata = span_ref.metadata();
        if noisy_executor_span(metadata.name()) {
            return;
        }
        let extensions = span_ref.extensions();
        let name = extensions
            .get::<CapturedSpanName>()
            .map(|name| name.0.as_ref())
            .unwrap_or_else(|| metadata.name());
        if !should_capture_span(metadata.target(), name) {
            return;
        }
        let name = Arc::<str>::from(name);
        drop(extensions);
        drop(span_ref);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.recording {
            return;
        }
        let Some(started_at) = state.started_at else {
            return;
        };
        let thread_id = current_profile_thread_id();
        let span = ActiveSpan {
            name,
            category: "re3bevy",
            started_at: Instant::now(),
            start_us: started_at.elapsed().as_micros(),
            thread_id,
        };
        state.active.insert((id.clone(), thread_id), span);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, Registry>) {
        if !self.recording.load(Ordering::Acquire) {
            return;
        }
        let Some(span_ref) = ctx.span(id) else {
            return;
        };
        let metadata = span_ref.metadata();
        if noisy_executor_span(metadata.name()) {
            return;
        }
        let extensions = span_ref.extensions();
        let name = extensions
            .get::<CapturedSpanName>()
            .map(|name| name.0.as_ref())
            .unwrap_or_else(|| metadata.name());
        if !should_capture_span(metadata.target(), name) {
            return;
        }
        drop(extensions);
        drop(span_ref);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let thread_id = current_profile_thread_id();
        if let Some(span) = state.active.remove(&(id.clone(), thread_id)) {
            let duration_us = span.started_at.elapsed().as_micros();
            let _ = state.writer_tx.send(WriterCommand::Span(ProfileEvent {
                name: span.name,
                category: span.category,
                start_us: span.start_us,
                duration_us,
                thread_id: span.thread_id,
            }));
        }
    }
}

pub(super) fn profile_capture_layer(app: &mut App) -> Option<BoxedLayer> {
    let (writer_tx, writer_rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("re3bevy-profile-writer".to_owned())
        .spawn(move || profile_writer(writer_rx))
        .ok()?;
    let output_dir = re3bevy_assets::install::RuntimePaths::diagnostic_root().to_path_buf();
    let capture = ProfileCapture::new(output_dir, writer_tx);
    app.insert_resource(capture.clone());
    if std::env::var_os("RE3BEVY_PROFILE_AUTO_FRAMES").is_some() {
        app.add_systems(Update, auto_profile_capture);
    }
    Some(Box::new(ProfileCaptureLayer {
        state: capture.state,
        recording: capture.recording,
    }))
}

fn auto_profile_capture(
    capture: Res<ProfileCapture>,
    mut frame: bevy::ecs::system::Local<u32>,
    mut stopped_at: bevy::ecs::system::Local<Option<u32>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(limit) = std::env::var("RE3BEVY_PROFILE_AUTO_FRAMES")
        .unwrap_or_default()
        .parse::<u32>()
    else {
        return;
    };
    if limit == 0 {
        return;
    }

    *frame += 1;
    if *frame == 1 {
        capture.toggle();
        return;
    }
    if *frame == limit {
        capture.toggle();
        *stopped_at = Some(*frame);
        return;
    }
    if stopped_at.is_some_and(|stopped| (*frame).saturating_sub(stopped) >= 120) {
        exit.write(AppExit::Success);
    }
}

pub(super) fn toggle_profile_capture(
    keys: Res<ButtonInput<KeyCode>>,
    capture: Res<ProfileCapture>,
) {
    if keys.just_pressed(KeyCode::F10) {
        capture.toggle();
    }
}
