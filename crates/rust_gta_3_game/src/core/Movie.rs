//! re3 reference: references/re-GTA/src/skel/win/win.cpp (startup movies).
//! Original MPEG-1/MP2 data is decoded in-process, then presented by Bevy.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use bevy::{
    asset::RenderAssetUsages,
    audio::AudioSinkPlayback,
    ecs::system::SystemParam,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::widget::NodeImageMode,
    window::WindowFocused,
};
use rust_gta_3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use rust_gta_3_core::core::GameState::{CGameState, GtaAppState};
use rust_gta_3_formats::formats::Mpeg::{MovieFrame, MovieInfo, MpegDecoder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupMovieKind {
    Logo,
    Intro,
}

#[derive(Debug)]
enum DecodedMovie {
    Ready(MovieInfo, Option<Vec<u8>>),
    Frame(MovieFrame),
    End,
    Error(String),
}

#[derive(Debug, Resource, Default)]
pub struct StartupMoviePlayer {
    kind: Option<StartupMovieKind>,
    root: Option<Entity>,
    image: Option<Entity>,
    audio: Option<Entity>,
    audio_handle: Option<Handle<AudioSource>>,
    frame_handle: Option<Handle<Image>>,
    receiver: Option<Mutex<mpsc::Receiver<DecodedMovie>>>,
    cancelled: Option<Arc<AtomicBool>>,
    info: Option<MovieInfo>,
    pending: Option<MovieFrame>,
    elapsed: f64,
    last_frame_end: f64,
    audio_wait: f64,
    started: bool,
    ended: bool,
    failed: bool,
    // Native startupDeactivate spans both startup movies.
    deactivated: bool,
}

#[derive(Component)]
pub struct StartupMovieImage;

pub fn setup_logo_movie(
    mut commands: Commands,
    mut player: ResMut<StartupMoviePlayer>,
    install: Option<Res<OriginalGtaInstall>>,
) {
    setup_movie(
        &mut commands,
        &mut player,
        install.as_deref(),
        StartupMovieKind::Logo,
        "Logo.mpg",
    );
}

pub fn setup_intro_movie(
    mut commands: Commands,
    mut player: ResMut<StartupMoviePlayer>,
    install: Option<Res<OriginalGtaInstall>>,
    game_state: Res<CGameState>,
) {
    let name = if game_state.french_game || game_state.german_game {
        "GTAtitlesGER.mpg"
    } else {
        "GTAtitles.mpg"
    };
    setup_movie(
        &mut commands,
        &mut player,
        install.as_deref(),
        StartupMovieKind::Intro,
        name,
    );
}

#[derive(SystemParam)]
pub struct MoviePlayback<'w, 's> {
    time: Res<'w, Time<Real>>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    gamepads: Query<'w, 's, &'static Gamepad>,
    focus: MessageReader<'w, 's, WindowFocused>,
    nodes: Query<'w, 's, &'static mut ImageNode, With<StartupMovieImage>>,
    sinks: Query<'w, 's, &'static AudioSink>,
    images: ResMut<'w, Assets<Image>>,
    sounds: ResMut<'w, Assets<AudioSource>>,
    commands: Commands<'w, 's>,
    player: ResMut<'w, StartupMoviePlayer>,
}

pub fn update_logo_movie(mut playback: MoviePlayback, mut next: ResMut<NextState<GtaAppState>>) {
    if playback.update() {
        next.set(GtaAppState::IntroMpeg);
    }
}

pub fn update_intro_movie(mut playback: MoviePlayback, mut next: ResMut<NextState<GtaAppState>>) {
    if playback.update() {
        next.set(GtaAppState::LoadingFrontend);
    }
}

fn setup_movie(
    commands: &mut Commands,
    player: &mut StartupMoviePlayer,
    install: Option<&OriginalGtaInstall>,
    kind: StartupMovieKind,
    name: &str,
) {
    clear_movie(commands, player);
    player.kind = Some(kind);
    if player.deactivated {
        player.failed = true;
        return;
    }
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Name::new(format!("StartupMovie{kind:?}")),
        ))
        .id();
    let image = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            StartupMovieImage,
        ))
        .id();
    commands.entity(root).add_child(image);
    player.root = Some(root);
    player.image = Some(image);
    let Some(install) = install else {
        error!("Startup movie: original GTA installation is missing");
        player.failed = true;
        return;
    };
    let path = install.resolve_existing_case_path(format!("movies/{name}"));
    let cancelled = Arc::new(AtomicBool::new(false));
    player.cancelled = Some(cancelled.clone());
    // Three queued RGBA frames, one pending frame and one Image. Backpressure
    // prevents decoding the entire film into RAM or writing a disk cache.
    let (sender, receiver) = mpsc::sync_channel(3);
    player.receiver = Some(Mutex::new(receiver));
    thread::spawn(move || {
        let decode = || -> Result<(), String> {
            let input =
                std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            let mut decoder = MpegDecoder::new(input)?;
            let audio = decoder.soundtrack(|| cancelled.load(Ordering::Relaxed))?;
            if cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            if sender
                .send(DecodedMovie::Ready(decoder.info, audio))
                .is_err()
            {
                return Ok(());
            }
            while !cancelled.load(Ordering::Relaxed) {
                let Some(frame) = decoder.next_frame()? else {
                    break;
                };
                if sender.send(DecodedMovie::Frame(frame)).is_err() {
                    return Ok(());
                }
            }
            if !cancelled.load(Ordering::Relaxed) {
                let _ = sender.send(DecodedMovie::End);
            }
            Ok(())
        };
        if let Err(error) = decode() {
            if !cancelled.load(Ordering::Relaxed) {
                let _ = sender.send(DecodedMovie::Error(error));
            }
        }
    });
}

impl MoviePlayback<'_, '_> {
    fn receive(&mut self) -> Option<DecodedMovie> {
        let receiver = self.player.receiver.as_ref()?;
        match receiver.lock().ok()?.try_recv() {
            Ok(message) => Some(message),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) if self.player.ended => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(DecodedMovie::Error("decoder disconnected".into()))
            }
        }
    }

    fn update(&mut self) -> bool {
        let lost_focus = self.focus.read().any(|event| !event.focused);
        self.player.deactivated |= lost_focus;
        if self.player.deactivated
            || self.keys.just_pressed(KeyCode::Enter)
            || self.keys.just_pressed(KeyCode::Space)
            || self.keys.just_pressed(KeyCode::AltLeft)
            || self.keys.just_pressed(KeyCode::AltRight)
            || self.keys.just_pressed(KeyCode::Tab)
            || self.mouse.just_pressed(MouseButton::Left)
            || self.touches.iter_just_pressed().next().is_some()
            || self
                .gamepads
                .iter()
                .any(|pad| pad.get_just_pressed().next().is_some())
        {
            return true;
        }
        if self.player.failed {
            return true;
        }

        if self.player.info.is_none() {
            match self.receive() {
                Some(DecodedMovie::Ready(info, audio)) => {
                    self.player.info = Some(info);
                    if let Some(bytes) = audio {
                        let handle = self.sounds.add(AudioSource {
                            bytes: bytes.into(),
                        });
                        self.player.audio = Some(
                            self.commands
                                .spawn((
                                    AudioPlayer::new(handle.clone()),
                                    PlaybackSettings::ONCE.paused(),
                                ))
                                .id(),
                        );
                        self.player.audio_handle = Some(handle);
                    }
                    info!(
                        "Startup movie decoded: kind={:?}, {}x{}, {} fps, audio={:?}",
                        self.player.kind, info.width, info.height, info.fps, info.sample_rate
                    );
                }
                Some(DecodedMovie::Error(error)) => {
                    error!("Startup movie decode failed: {error}");
                    return true;
                }
                None => return false,
                _ => {
                    error!("Startup movie: missing decoder metadata");
                    return true;
                }
            }
        }

        if !self.player.started {
            self.player.audio_wait += self.time.delta_secs_f64();
            // An unavailable audio device must not leave startup on a black screen.
            if self
                .player
                .audio
                .is_some_and(|entity| self.sinks.get(entity).is_err())
                && self.player.audio_wait < 3.0
            {
                return false;
            }
            if self.player.pending.is_none() {
                match self.receive() {
                    Some(DecodedMovie::Frame(frame)) => self.player.pending = Some(frame),
                    Some(DecodedMovie::Error(error)) => {
                        error!("Startup movie decode failed: {error}");
                        return true;
                    }
                    Some(DecodedMovie::End) => {
                        error!("Startup movie produced no video frames");
                        return true;
                    }
                    None => return false,
                    _ => return false,
                }
            }
            if let Some(sink) = self
                .player
                .audio
                .and_then(|entity| self.sinks.get(entity).ok())
            {
                sink.play();
            } else if let Some(entity) = self.player.audio.take() {
                warn!("Startup movie audio output unavailable; playing video only");
                self.commands.entity(entity).despawn();
                self.player.audio_handle = None;
            }
            self.player.started = true;
            info!(
                "Startup movie playback started: kind={:?}",
                self.player.kind
            );
        } else {
            self.player.elapsed += self.time.delta_secs_f64();
            if let Some(sink) = self
                .player
                .audio
                .and_then(|entity| self.sinks.get(entity).ok())
            {
                if !sink.empty() {
                    self.player.elapsed = sink.position().as_secs_f64();
                }
            }
        }

        loop {
            if self.player.pending.is_none() {
                match self.receive() {
                    Some(DecodedMovie::Frame(frame)) => self.player.pending = Some(frame),
                    Some(DecodedMovie::End) => {
                        self.player.ended = true;
                        break;
                    }
                    Some(DecodedMovie::Error(error)) => {
                        error!("Startup movie decode failed: {error}");
                        return true;
                    }
                    _ => break,
                }
            }
            if self
                .player
                .pending
                .as_ref()
                .is_some_and(|frame| frame.time > self.player.elapsed)
            {
                break;
            }
            let frame = self.player.pending.take().unwrap();
            let info = self.player.info.unwrap();
            self.player.last_frame_end = frame.time + 1.0 / info.fps;
            if let Some(handle) = self.player.frame_handle.as_ref() {
                if let Some(mut image) = self.images.get_mut(handle) {
                    image.data = Some(frame.rgba);
                }
            } else {
                let handle = self.images.add(Image::new(
                    Extent3d {
                        width: info.width,
                        height: info.height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    frame.rgba,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                ));
                if let Some(entity) = self.player.image {
                    if let Ok(mut node) = self.nodes.get_mut(entity) {
                        node.image = handle.clone();
                    }
                }
                self.player.frame_handle = Some(handle);
            }
        }
        let audio_finished = self
            .player
            .audio
            .and_then(|entity| self.sinks.get(entity).ok())
            .is_none_or(|sink| sink.empty());
        if self.player.ended && self.player.elapsed >= self.player.last_frame_end && audio_finished
        {
            info!(
                "Startup movie playback completed: kind={:?}",
                self.player.kind
            );
            return true;
        }
        false
    }
}

pub fn cleanup_movie(mut commands: Commands, mut player: ResMut<StartupMoviePlayer>) {
    clear_movie(&mut commands, &mut player);
}

impl Drop for StartupMoviePlayer {
    fn drop(&mut self) {
        if let Some(cancelled) = &self.cancelled {
            cancelled.store(true, Ordering::Relaxed);
        }
    }
}

fn clear_movie(commands: &mut Commands, player: &mut StartupMoviePlayer) {
    if let Some(cancelled) = player.cancelled.take() {
        cancelled.store(true, Ordering::Relaxed);
    }
    // Dropping the receiver releases a worker blocked by the bounded frame queue.
    player.receiver = None;
    if let Some(root) = player.root.take() {
        commands.entity(root).despawn();
    }
    if let Some(audio) = player.audio.take() {
        commands.entity(audio).despawn();
    }
    let deactivated = player.deactivated;
    *player = StartupMoviePlayer::default();
    player.deactivated = deactivated;
}
