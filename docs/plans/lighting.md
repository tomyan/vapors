# Lighting: normals, light nodes and a lit material

Status: plan, 2026-11-19. Asked for by Tom after the Toad Trip video,
ahead of the order in `docs/plans/3d-engine-parity.md` (3.3, rows L1 and
L2), which put lighting after property 9. It follows that plan's design
with two simplifications for a first version, named below, each with
what retires it.

## Goal

A `.wisp` scene can say "this mesh is lit" and place a directional
light, point lights and an ambient level, and the meshes shade by them:
Lambert diffuse and Blinn-Phong specular, in linear light. Enough for a
dusk scene with street lamps and headlights. Shadows, bloom, textures
and PBR are later rows of the parity plan.

## Design

- **Normals.** `Geometry` gains optional per-vertex normals, validated
  when given (one per position, finite, unit length within 1e-3), and
  the vertex layout carries them only when present. A helper builds
  flat-shaded geometry (each triangle its own corners and face normal)
  from positions and indices, so low-poly models get facets without
  hand-written normals. The existing per-vertex `shade` stays: unlit
  meshes use it as today, and a lit mesh multiplies its lit colour by it
  (baked occlusion).
- **Light nodes.** `NodeKind::Light(Light)`, with `Light` a closed enum:
  `Directional { colour, intensity }` shining along the node's -Z, and
  `Point { colour, intensity, range }` at the node's position, falling
  off smoothly to zero at `range`. At most 8 lights per scene; a ninth is
  refused naming the limit. Ambient (`colour`, `intensity`) is a prop of
  the scene root. One lights uniform per scene, written only when a
  light's world version or value changes, or ambient changes.
- **The lit material.** A mesh chooses `Shading::Unlit` (today's
  behaviour, the default) or `Shading::Lit`, a closed enum prop. Lit
  shading converts the sRGB colour to linear, sums ambient, Lambert and
  Blinn-Phong over the lights, and encodes back to sRGB. The lighting
  code is a WGSL chunk of its own (`lighting.wgsl`) so the flat scene's
  mesh pipeline can use the same chunk later (parity plan G4).
- **Refusals.** A lit mesh whose geometry has no normals is refused at
  the frame, naming node and geometry. A scene with a lit mesh, no
  light and zero ambient is refused at mount, naming the fix.

## Simplifications for the first version

1. **No `Materials` store yet** (parity plan M1). `Shading` is a prop on
   the mesh; the store arrives with textures, and `Shading` becomes the
   first two `MaterialKind`s then.
2. **The lit pipeline is made when the first lit mesh mounts**, not from
   a compile-time `SceneUses` (parity plan M3, which needs the build-time
   analysis). Declared RELAXED where it runs; retired by M3. A scene
   without lit meshes makes no lit pipeline and draws exactly as before.

## Slices

| Slice | Content | Accepted when |
|---|---|---|
| LT1 | Normals on `Geometry` (optional, validated); the vertex layout per attribute set; the flat-shading helper | A geometry with normals uploads and draws unchanged when unlit (pixels equal); each validation refuses naming the vertex; the helper's normals match a CPU cross product for a box and a prism |
| LT2 | `NodeKind::Light`, the lights uniform, ambient on the scene root, `<Light>` component, the 8-light limit | The uniform's bytes match the scene for one directional and two point lights; moving a light writes the uniform once and no node slots; a ninth light refused; an idle frame writes nothing |
| LT3 | `Shading::Lit`, `lighting.wgsl`, the lit pipeline, the refusals above, docs | Pixel tests: a box lit from one side, each face within 1/255 per channel of the CPU formula, for a directional light and for a point light; unlit scenes' pixels and draw counts unchanged; `docs/scene3d.md` section with a `.wisp` example |

Each slice is under about 400 lines, keeps every existing pixel test
equal, and pins `FrameCounts::surfaces` for an idle frame and for one
change on a scene without lights, equal to before.
