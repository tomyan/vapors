//! The scene property set (docs/plans/css-for-3d.md, sections 3 and
//! 4.2): which longhands a node of each scene tag reads, and what to use
//! instead of each one a tag does not. One answer for the compiler's
//! build-time check and the elements runtime. Every longhand is in
//! exactly one group; a test holds the groups and the property table
//! together, so a new longhand fails until it is placed here.
//! Custom properties are not longhands: every scene tag reads them.

use std::sync::LazyLock;

use super::PropertyId;

/// The tag of a scene component's anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneTag {
    /// `<Scene>`: the root of a scene.
    Scene,
    /// `<Group>`: a node that holds others.
    Group,
    /// `<Mesh>`: geometry drawn in one colour.
    Mesh,
    /// `<Camera>`: a view of the scene.
    Camera,
    /// `<Decal>` (docs/plans/one-scene.md S8): paths on a mesh.
    Decal,
}

impl SceneTag {
    /// Every scene tag.
    pub const ALL: [Self; 5] = [
        Self::Scene,
        Self::Group,
        Self::Mesh,
        Self::Camera,
        Self::Decal,
    ];

    /// The scene tag named `tag`; `None` for any other element.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|scene| scene.name() == tag)
    }

    /// The element tag.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Group => "group",
            Self::Mesh => "mesh",
            Self::Camera => "camera",
            Self::Decal => "decal",
        }
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    /// Whether a node of this tag reads `id`: a declaration of any
    /// longhand it does not read is refused, naming [`instead`].
    #[must_use]
    pub fn reads(self, id: PropertyId) -> bool {
        group_of(id).readers & self.bit() != 0
    }
}

/// What to use on a scene node instead of `id`, where its tag does not
/// read it (section 3's table).
#[must_use]
pub fn instead(id: PropertyId) -> &'static str {
    group_of(id).instead
}

/// Longhands that the same tags read and that name the same
/// alternative.
struct Group {
    readers: u8,
    instead: &'static str,
    /// The longhands' names, separated by whitespace.
    longhands: &'static str,
}

const MESH: u8 = SceneTag::Mesh.bit();
/// The tags whose nodes are drawn or hit: all but `camera`.
const DRAWN: u8 =
    SceneTag::Scene.bit() | SceneTag::Group.bit() | SceneTag::Mesh.bit() | SceneTag::Decal.bit();
const NONE: u8 = 0;

/// Section 3's rows, and section 4.2's set as the readers of each.
const GROUPS: &[Group] = &[
    Group {
        readers: MESH,
        instead: "set `color` on each `mesh`, or set a custom property here and read it there with `color: var(--name)`",
        longhands: "color",
    },
    Group {
        readers: MESH,
        instead: "set `opacity` on each `mesh`: a group's opacity is not composited as one layer (docs/plans/css-for-3d.md, D3)",
        longhands: "opacity",
    },
    Group {
        readers: MESH,
        instead: "set `cursor` on each `mesh`",
        longhands: "cursor",
    },
    Group {
        readers: DRAWN,
        instead: "nothing: a camera is neither drawn nor hit",
        longhands: "visibility pointer-events",
    },
    Group {
        readers: MESH,
        instead: "transition or animate the `color` or `opacity` of each `mesh`",
        longhands: MOTION,
    },
    Group {
        readers: NONE,
        instead: "a scene node has no box: its shape is `<Mesh geometry>`, and the surface's box is the `gpu-surface` element's",
        longhands: BOX_MODEL,
    },
    Group {
        readers: NONE,
        instead: "a scene node is placed by its `transform` prop",
        longhands: "position top right bottom left",
    },
    Group {
        readers: NONE,
        instead: "depth orders scene nodes (the depth buffer); there is no paint order to index into",
        longhands: "z-index",
    },
    Group {
        readers: NONE,
        instead: "hide the node with `visibility: hidden`, or remove it by putting its component in an `{if}`",
        longhands: "display",
    },
    Group {
        readers: NONE,
        instead: "the `transform` prop moves a node; a parent `<Group>` gives a pivot; `<Camera projection>` gives the perspective",
        longhands: "transform",
    },
    Group {
        readers: NONE,
        instead: "nothing: a post-process on a scene is not planned (docs/briefs/GPU_SURFACE_3D_BRIEF.md, Deferred work)",
        longhands: "filter",
    },
    Group {
        readers: NONE,
        instead: "a scene anchor has no box, so it is no container; use `@media`",
        longhands: "container-name container-type",
    },
];

/// The transitions and animations a mesh will read (C7).
const MOTION: &str = "
    animation-delay animation-direction animation-duration animation-fill-mode
    animation-iteration-count animation-name animation-play-state
    animation-timing-function transition-delay transition-duration
    transition-property transition-timing-function";

/// The box model, text and layout longhands (section 3, first row).
const BOX_MODEL: &str = "
    align-content align-items align-self aspect-ratio background-color
    border-bottom-color border-bottom-left-radius border-bottom-right-radius
    border-bottom-style border-bottom-width border-left-color border-left-style
    border-left-width border-right-color border-right-style border-right-width
    border-top-color border-top-left-radius border-top-right-radius
    border-top-style border-top-width box-shadow box-sizing column-gap
    flex-basis flex-direction flex-grow flex-shrink flex-wrap font-family
    font-feature-settings font-size font-style font-variation-settings
    font-weight grid-auto-columns grid-auto-flow grid-auto-rows grid-column-end
    grid-column-start grid-row-end grid-row-start grid-template-columns
    grid-template-rows height justify-content justify-items justify-self
    letter-spacing line-height margin-bottom margin-left margin-right margin-top
    max-height max-width min-height min-width outline-color outline-offset
    outline-style outline-width overflow-x overflow-y padding-bottom
    padding-left padding-right padding-top row-gap scrollbar-color
    scrollbar-width text-align text-decoration user-select white-space width";

/// Each longhand's group, by its index; built once.
static GROUP_OF: LazyLock<Vec<&'static Group>> = LazyLock::new(|| {
    PropertyId::all()
        .map(|id| {
            GROUPS
                .iter()
                .find(|group| group.names().any(|name| name == id.name()))
                .unwrap_or_else(|| panic!("`{}` is in no group of crates/css/src/property/scene.rs; place it in the group whose alternative fits (cargo test -p vapors-css --lib every_longhand_is_in_exactly_one_scene_group)", id.name()))
        })
        .collect()
});

impl Group {
    fn names(&self) -> impl Iterator<Item = &'static str> {
        self.longhands.split_whitespace()
    }
}

fn group_of(id: PropertyId) -> &'static Group {
    GROUP_OF[id.index()]
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
