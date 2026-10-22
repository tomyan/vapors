//! Transitions and `@keyframes` animations: specs read from a computed
//! style, timing functions, interpolation (lengths, numbers, colours in
//! Oklab), and pure `sample(t)` samplers. No clock lives here: the scene
//! layer keeps the start times and asks for values.

mod interpolate;
mod keyframes;
mod sampler;
mod spec;
mod timing;
mod timing_curve;
mod transition;

pub use interpolate::interpolate;
pub use keyframes::{ResolvedKeyframes, Stop};
pub use sampler::{Animation, AnimationSample};
pub use spec::{
    AnimationSpec, Direction, FillMode, IterationCount, PlayState, TransitionProperty,
    TransitionSpec, animations, transitions,
};
pub use timing::{StepPosition, TimingFunction};
pub use transition::Transition;
