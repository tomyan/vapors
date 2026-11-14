//! Which scene the judged 60 s run drives (`--scene-run [racer|one-scene]`):
//! the racer (P0's run, `racer_driven.rs`) or the one-scene example, the
//! minimal scene for finish-line property 9 (`one_scene_driven.rs`).
//! Each names its report, its reproduce command, its workload, and the
//! scripted changes of a frame, which a frame over budget is reported
//! with.

use crate::driver::{Scripted, scripted};
use crate::one_scene_driven::DrivenScene;
use crate::racer_driven::DrivenRacer;
use crate::run::Workload;

/// A scene the judged run drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    /// The racer, driven through its HUD and keys.
    Racer,
    /// The one-scene example: HTML over a 3D scene CSS styles.
    OneScene,
}

impl SceneKind {
    /// The scene named `name` on the command line.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "racer" => Some(Self::Racer),
            "one-scene" => Some(Self::OneScene),
            _ => None,
        }
    }

    /// The report's heading.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Racer => "racer, one scene, 60 s",
            Self::OneScene => "one-scene example, 60 s",
        }
    }

    /// How to run it again.
    #[must_use]
    pub fn reproduce(self) -> &'static str {
        match self {
            Self::Racer => "cargo run --release -p vapors-perf -- --scene-run",
            Self::OneScene => "cargo run --release -p vapors-perf -- --scene-run one-scene",
        }
    }

    /// Its driven workload.
    ///
    /// # Errors
    /// It cannot be mounted.
    pub fn workload(self) -> Result<Box<dyn Workload>, String> {
        Ok(match self {
            Self::Racer => Box::new(DrivenRacer::new()),
            Self::OneScene => Box::new(DrivenScene::new()?),
        })
    }

    /// The scripted changes due at `frame`, named.
    #[must_use]
    pub fn changes(self, frame: usize) -> Vec<String> {
        match self {
            Self::Racer => scripted(frame).into_iter().map(racer_change).collect(),
            Self::OneScene => vapors_one_scene::schedule::changes_at(frame),
        }
    }
}

fn racer_change(change: Scripted) -> String {
    match change {
        Scripted::Camera { follow: true } => "camera follows".into(),
        Scripted::Camera { follow: false } => "camera fixed".into(),
        Scripted::Livery => "livery".into(),
    }
}
