# Graphite desktop UI

## Smoky / Дымчатый

The second theme is a separate warm night-room identity: charcoal brown surfaces, milk text, muted peach, one calm smoky-gray cat with a cream muzzle and dark ears. Keep the original typeface, product hierarchy and logo. New imagery is a quiet window onto a voxel valley, with a clear sill reserved for the animated SVG character. Motion is small and purposeful: sleep/breathe, look, watch, stretch, alert ears. Avoid repeated meowing, floating hearts, slogans or oversized anime eyes. Theme selection lives in Appearance and persists through Rust before startup, without replacing Graphite.

Five generated atlases provide 80 semantic replacements in one material/camera palette; slots keep the existing native keys. Clip exact measured bounds using transparent gutters, clean only faint alpha haze at render time and preserve palette values. Do not overwrite source images or custom instance icons. The live character/startup stays code-native for controllable motion and reduced-motion support. Art-direction prompts are kept in design/.

## Operation motion

0.10.3: action geometry has explicit gaps and dimension-preserving rounded edges; scanner overlays never intersect blocks and the update shell has a real central cavity. Recycle tunnel frames and route signals while invisible, not at a visible endpoint. Keep one canvas and an accumulated visual clock through modal transitions and visibility changes. Completion slows from the current pose, followed by a short fade. Reserve a separate clipped viewport above readable status text. Startup logo animation is excluded from these changes.

Use one compact cinematic surface, not six competing overlays. Matte beveled graphite and warm pale planes carry structure; coral joints indicate activity. Portal frames travel in depth over the retained basalt artwork. Server endpoints use a thin spatial path and travelling signal. Pack plates assemble, mod plates join in sequence, a translucent scanner sweeps the structure, and a version-change receipt closes shells around a replaced core. Keep actual status text readable beneath the scene. Never translate a server ping into a successful connection or hold back a completed operation for the animation. A changed installed version triggers the core scene; normal launches retain the approved logo cinematic. Hide controls must remain interactive inside modal dialogs; z-index or top-layer placement alone does not escape modal inertness.

## Startup cinematic

Use real extruded silhouettes of the approved logo 06, not generic cubes or a replacement identity. Four graphite pieces and the coral joint assemble with staggered translation and restrained rotation. The camera pulls away from a macro crop; the final frontal view crossfades into the original SVG and travels into the header. PBR matte materials, beveled edges, studio reflections and soft contact shadows carry the depth. Avoid fake loading percentages and secondary slogans. Workspace mounts concurrently; the overlay is skippable, reduced-motion uses no WebGL, and the scene does not keep running behind the library.

## 0.10 instance tools

Preflight, crash isolation, session history, disk usage and download queue share the existing modal shell and warm graphite palette. A single status illustration anchors each tool; readable rows carry the actual findings. Server cards keep the server address, live metrics and selected instance in a clear hierarchy. Use icons by their catalog meaning, not arbitrary sheet numbers. Surfaces have shallow matte depth, with short transform/opacity entrances and an optional small 3D rotation on the server emblem. Reduced-motion disables these transitions; no continuous 3D scene, blur loop or decorative GPU workload is introduced. Preserve the existing Minecraft artwork, logo and typeface.

## Approved direction for the whole launcher

Application identity: the user selected candidate 06 from `outputs/Graphite-logo-options-01.png`. `public/graphite.svg` is its editable vector adaptation: four chamfered graphite blocks, a warm-gray L-shaped face and a coral inner joint. Do not revert to the old orange wireframe cube. Keep this application logo consistent in the header and native Windows/installer icons; content-category pictograms and user-selected instance images retain their own identities.

The user's chosen style master is `outputs/icon-concepts/02-instance-categories.png`. It applies to the entire future launcher design, not just icons: substantial geometric forms, shallow matte depth, charcoal/graphite surfaces, warm-gray front planes and restrained coral edges, seams and active details. Keep the material, geometry, corner treatment and lighting coherent across icons, buttons, panels, navigation and states. Sheet 01 and the old monoline sheet 03 are not style references.

Keep typography crisp and controls readable; translate the reference's material character into interface surfaces rather than turning text into rendered objects. Preserve the restored Minecraft artwork. Version 0.9 integrates the 16 category icons plus 64 action/state icons from the three approved sheets, using their exact original image regions. All 80 are available in the instance icon picker. Keep simple arrows, plus and close/minimize controls as compact utility glyphs instead of forcing unrelated pictograms onto them.

## Current implementation notes

Preserve the launcher's visual character: a scan-friendly instance list and a cinematic Minecraft artwork area with a prominent instance title. The user explicitly rejected removing the artwork and replacing it with a plain table. Keep operational controls on a separate readable surface below the art; do not interpret anti-slop work as blanket removal of visual detail.

- Use self-hosted IBM Plex Sans Variable with Cyrillic support: 14px navigation, 15px instance names, 12px supporting text and 30–34px headings. The system Segoe UI version was rejected by the user as too plain.
- Use layered graphite surfaces, a clear selected instance, grouped actions and restrained coral accents. Avoid stripping the workspace into a featureless table; the user explicitly rejected that simplification.
- Use the approved material icons for navigation, instance identities and grouped actions. Keep small utility glyphs (search, check, spinner, play, arrows) crisp vectors. Preserve custom instance images and the Graphite application mark.
- Show real version, loader, playtime and last launch. `lastLaunchTime` is Unix milliseconds (Prism `BaseInstance::setLastLaunch`); do not multiply it by 1000. Empty values must say that the instance has not been played/launched.
- At short window heights, hide secondary quick-start/breadcrumb/footer blocks and keep instance actions and accounts reachable. Test at the supported minimum 860×620, with long Cyrillic names.
- Keep modal controls consistent with the main window. Retain keyboard focus, error, loading, empty and reduced-motion states.

Research used for the 0.8.1 review:

- [Raycast: A fresh look and feel](https://www.raycast.com/blog/a-fresh-look-and-feel): consistent icon geometry, contextual actions and compact mode.
- [InterfaceKit: What makes a website look AI-generated?](https://blog.interfacekit.io/what-makes-a-website-look-ai-generated): decorative treatment is not a substitute for hierarchy; review second screens and awkward states.

These are design references, not evidence that any particular font, icon or color was generated by AI.

## Shared controls, 0.11.1

Both themes use controls.css for interaction geometry; palettes, artwork and icon assets remain theme-specific. Do not restore bordered tiles for every action. Preserve disabled states, danger text, visible keyboard focus and reduced motion.

| Before | After | Why |
| --- | --- | --- |
| Every action has the same bevel and border | Filled main action, quiet secondary surface, unboxed instance tools | Action importance is visible without repeating cards |
| Squared 5–7px controls and readiness badge | 11–15px controls, rounded create action, readiness badge removed | Softer interaction shapes and less visual clutter |
| Hover lifting and inconsistent button feedback | Short color feedback and 90ms press response, no card lifting | Repeated desktop actions stay stable and responsive |

## Integration review, 0.9

| Before | After | Why |
| --- | --- | --- |
| Dark rectangles around sheet regions | SVG alpha filter removes the near-black sheet background; original PNGs are unchanged | Icons sit on artwork and controls without visible image tiles |
| Oversized material pictograms and tiny shaded utilities | 25px navigation, 24px actions, 34px instance/picker icons; simple small utilities | Equal visual weight and clear loading/confirmation states |
| Blue-gray panels, warmer icons and several coral shades | Warm graphite surfaces and shared coral `#e8937c` | One palette across the workspace, settings and catalog |

Icon placement audit: use `VersionIcon` for release/snapshot/alpha/beta and quick start; `ContentIcon` for mod/modpack/resource/shader categories and empty states; `AccountIcon` for local versus connected profiles. Command palette entries reuse the instance's actual icon. `ProjectIcon` keeps a category fallback visible until a remote image loads and after a failure. Keep the app logo in the window header, and preserve third-party project artwork and custom instance images.

Artwork treatment: preserve `basalt.png`, crop slightly closer around the lava arch, reduce saturation and brightness, and blend the edges and title area into warm graphite with static overlays. Keep the image readable; do not replace it with an empty panel or add continuous decorative animation.
