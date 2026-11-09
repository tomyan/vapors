# Racer component structure

`ui/hud.wisp` owns the shared UI signals and composes five child components:
RaceStats, RaceMenu, CameraToggle, DrivingControls and CarSetup. Each child owns
its layout styles. HUD supplies the document's base button theme; DrivingControls
styles the reusable HoldButton through inherited CSS custom properties.

DrivingControls mounts four `vapors-components` HoldButtons. Their `pressed`
signals feed the application controls alongside keyboard state. Every pointer
event goes through DocumentInput; the racer does not hit-test pedals or capture
fingers itself. Pause resets the shared held-state signals before a later resume.

Rendering and simulation remain application Rust code. The track renderer draws
first; ScenePainter draws the compiled HUD over it. The 3D scene is not a template
component or a declarative scene graph.

See `../../docs/hold-button.md` for the shared control and
`../../docs/RACER_PLAN.md` for design and validation. Browser checks in `scripts/`
at the repository root exercise driving, capture outside button bounds,
cancellation, camera transitions, short landscape layouts and viewport resizing.
