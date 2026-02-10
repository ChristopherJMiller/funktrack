---
name: bevy-ui-design
description: Create distinctive, polished Bevy UI interfaces with high design quality. Use this skill when building game UI components, HUDs, menus, or any Bevy UI elements. Generates creative, intentional UI code that avoids generic defaults and commits to a cohesive aesthetic vision.
---

This skill guides creation of distinctive, polished Bevy UI interfaces that avoid generic "placeholder" aesthetics. Implement real working code with exceptional attention to visual details and intentional design choices within Bevy's styling constraints.

The user provides UI requirements: a component, screen, HUD element, or interface to build. They may include context about the game's aesthetic, the UI's purpose, or technical constraints.

## Design Thinking

Before coding, understand the context and commit to a BOLD aesthetic direction:

- **Purpose**: What game state does this UI communicate? What's the player's mental state when viewing it (combat stress, menu browsing, victory celebration)?
- **Tone**: Commit to a distinct direction: dark/moody fantasy, clean esports readability, whimsical/playful, retro-futuristic, brutalist/raw, ethereal/mystical, industrial/gritty, minimalist/elegant, maximalist/ornate. Use these as inspiration—the final design should feel singular, with every detail serving one cohesive direction.
- **Asset vs Code**: What requires external assets (decorative frames, icons, textures) vs what can be achieved programmatically (colors, layout, dynamic state indicators)?
- **Differentiation**: What makes this UI memorable? What's the one visual choice someone will remember?

**CRITICAL**: Choose a clear conceptual direction and execute it vigorously. The key is intentionality—every color, every spacing value, every border radius should serve the aesthetic vision.

Then implement working Bevy UI code that is:

- Functional and performant within the ECS paradigm
- Visually striking and memorable within Bevy's constraints
- Cohesive with a clear aesthetic point-of-view
- Meticulously refined in spacing, color, and hierarchy

## Bevy UI Capabilities Reference

### What You Can Style Programmatically

**Layout (Full Flexbox)**
- `flex_direction`: Column, Row
- `justify_content`: FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly
- `align_items` / `align_self`: FlexStart, FlexEnd, Center, Stretch
- `flex_wrap`: NoWrap, Wrap
- `flex_grow`, `flex_shrink`, `flex_basis`
- `column_gap`, `row_gap`

**Sizing**
- `Val::Px(f32)` - Absolute pixels
- `Val::Percent(f32)` - Percentage of parent
- `Val::Vh(f32)`, `Val::Vw(f32)` - Viewport units
- `Val::VMin(f32)`, `Val::VMax(f32)` - Min/max viewport dimension
- `Val::Auto` - Automatic sizing

**Spacing**
- `margin`, `padding`: Use `UiRect` for per-side control
- `UiRect::all(Val)`, `UiRect::horizontal(Val)`, `UiRect::vertical(Val)`
- `UiRect { top, bottom, left, right }`

**Positioning**
- `position_type`: Relative (default), Absolute
- `top`, `bottom`, `left`, `right`: Position offsets for absolute elements

**Colors**
- `BackgroundColor(Color::srgb(r, g, b))` or `Color::srgba(r, g, b, a)`
- `BorderColor` with per-side colors
- `TextColor` for text elements
- Full alpha transparency support for layering and depth

**Borders**
- `border`: Width via `UiRect`
- `BorderRadius::all(Val::Px(f32))` or per-corner control
- `BorderColor` for color

**Text**
- `Text::new(string)` for content
- `TextFont { font_size, .. }` for sizing
- `TextColor` for color
- `TextLayout { justify: Justify::Center/Left/Right }` for alignment

**Interaction States**
- `Interaction` component: `Pressed`, `Hovered`, `None`
- Update colors/styles in systems based on interaction state

### What Requires Assets or Workarounds

- **Gradients**: Not supported—use layered semi-transparent nodes or asset images
- **Shadows**: Not supported—fake with offset darker nodes or asset images
- **Complex shapes**: Not supported—use asset images or border-radius creativity
- **Textures/Patterns**: Requires `ImageNode` with loaded assets
- **Icons**: Requires loaded image assets
- **Custom fonts**: Requires font assets loaded via asset system
- **Animations**: No CSS animations—implement via systems modifying Transform or style properties over time

## Bevy UI Aesthetic Guidelines

### Color & Theme

Commit to a cohesive palette. Define it explicitly in code:

```rust
// Example: Dark fantasy palette
const PRIMARY: Color = Color::srgb(0.6, 0.2, 0.8);      // Deep purple
const PRIMARY_HOVER: Color = Color::srgb(0.7, 0.3, 0.9);
const ACCENT: Color = Color::srgb(0.9, 0.7, 0.2);        // Gold accent
const SURFACE: Color = Color::srgb(0.08, 0.06, 0.12);    // Near-black purple
const SURFACE_ELEVATED: Color = Color::srgb(0.12, 0.1, 0.18);
const TEXT_PRIMARY: Color = Color::srgb(0.95, 0.93, 0.98);
const TEXT_MUTED: Color = Color::srgb(0.6, 0.55, 0.65);
const ERROR: Color = Color::srgb(0.9, 0.3, 0.3);
const SUCCESS: Color = Color::srgb(0.3, 0.8, 0.4);
```

- Lead with a dominant color, punctuate with sharp accents
- Use alpha transparency (`Color::srgba`) for atmospheric depth and layering
- Define all interaction states: normal, hover, pressed, disabled
- Avoid pure white (`1.0, 1.0, 1.0`) and pure black (`0.0, 0.0, 0.0`)—they feel flat

### Spatial Composition

- **Intentional spacing**: Every margin and padding value should be deliberate. Use a spacing scale (4, 8, 12, 16, 24, 32, 48) rather than arbitrary numbers.
- **Visual hierarchy through size**: Important elements should be noticeably larger. Create dramatic scale differences, not subtle ones.
- **Asymmetry**: Don't default to centered everything. Offset layouts create visual interest.
- **Negative space**: Generous padding around important elements draws attention. Cramped layouts feel generic.
- **Overlapping elements**: Use absolute positioning to create depth—elements that overlap feel more dynamic than flat grids.

### Typography & Fonts

**Loading Custom Fonts**

Bevy's default font is functional but generic. Custom fonts elevate UI significantly:

```rust
// Load fonts in a startup system or via asset loader
fn setup_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(UiFonts {
        heading: asset_server.load("fonts/cinzel_bold.ttf"),
        body: asset_server.load("fonts/lato_regular.ttf"),
        mono: asset_server.load("fonts/jetbrains_mono.ttf"),
    });
}

#[derive(Resource)]
struct UiFonts {
    heading: Handle<Font>,
    body: Handle<Font>,
    mono: Handle<Font>,
}

// Use in UI spawning
commands.spawn((
    Text::new("Game Title"),
    TextFont {
        font: ui_fonts.heading.clone(),
        font_size: 48.0,
        ..default()
    },
    TextColor(TEXT_PRIMARY),
));
```

**How Many Fonts?**

Keep it tight—fonts are a commitment:

| Count | Use Case |
|-------|----------|
| **1 font** | Minimalist designs. Vary weight/size for hierarchy. |
| **2 fonts** | Most games. One display/heading + one readable body. |
| **3 fonts** | Maximum recommended. Heading + body + monospace (for numbers/stats). |

More than 3 fonts creates visual chaos. If you need variety, use weights (Regular, Bold, Light) of the same family.

**Font Pairing Guidelines**

- **Contrast is key**: Pair a decorative display font with a clean readable font
- **Match the tone**: Fantasy game? Serif or blackletter headings. Sci-fi? Geometric sans.
- **Readability for body**: Body text must be legible at 14-18px. Save expressive fonts for headings.
- **Numbers matter**: For stats/health/mana, consider fonts with tabular (monospace) numerals so values don't jump around

**Font Suggestions by Aesthetic**

| Tone | Heading Options | Body Options |
|------|-----------------|--------------|
| Dark Fantasy | Cinzel, Cormorant, EB Garamond | Lato, Source Sans, Crimson Text |
| Sci-Fi | Orbitron, Exo 2, Rajdhani | Inter, IBM Plex Sans, Roboto |
| Clean Esports | Montserrat, Oswald, Bebas Neue | Open Sans, Nunito, Work Sans |
| Whimsical | Fredoka, Baloo, Bubblegum | Quicksand, Comfortaa, Varela Round |
| Retro | Press Start 2P, VT323, Silkscreen | Share Tech Mono, IBM Plex Mono |

**Size Hierarchy**

Establish clear differentiation—subtle differences are invisible:

```rust
// Define as constants for consistency
const FONT_DISPLAY: f32 = 64.0;    // Splash screens, major titles
const FONT_TITLE: f32 = 48.0;      // Screen titles
const FONT_HEADING: f32 = 28.0;    // Section headers
const FONT_SUBHEAD: f32 = 22.0;    // Subsections
const FONT_BODY: f32 = 18.0;       // Primary readable text
const FONT_CAPTION: f32 = 14.0;    // Secondary info, labels
const FONT_TINY: f32 = 11.0;       // Fine print, badges
```

- **Minimum 1.25x ratio** between adjacent levels for clear hierarchy
- Use color AND size together—muted small text vs bright large text
- Consider text alignment for visual flow—left-aligned body, centered titles
- Letter spacing and line height aren't available, so compensate with surrounding padding

### Dynamic State Indicators

Game UI lives and breathes. Make state changes visually interesting:

**Progress Bars with Personality**
```rust
// Don't just change width—add visual interest
// Container with subtle inner glow effect via layered backgrounds
parent.spawn((
    Node {
        width: Val::Px(200.0),
        height: Val::Px(16.0),
        padding: UiRect::all(Val::Px(2.0)),
        ..default()
    },
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
    BorderRadius::all(Val::Px(8.0)),
)).with_children(|bar| {
    // Inner track
    bar.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.3, 0.3, 0.2)), // Subtle colored glow
        BorderRadius::all(Val::Px(6.0)),
    )).with_children(|track| {
        // Actual fill - updated dynamically
        track.spawn((
            HealthBarFill,
            Node {
                width: Val::Percent(75.0), // Dynamic
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.9, 0.3, 0.3)),
            BorderRadius::all(Val::Px(6.0)),
        ));
    });
});
```

**Cooldown Overlays**
- Fill from bottom or top, not just left-to-right
- Use semi-transparent dark overlay with the remaining time
- Consider a subtle border glow when ability becomes available

**Interaction Feedback**
```rust
fn update_button_style(
    mut query: Query<(&Interaction, &mut BackgroundColor, &ButtonStyle), Changed<Interaction>>,
) {
    for (interaction, mut bg, style) in &mut query {
        *bg = BackgroundColor(match *interaction {
            Interaction::Pressed => style.pressed,
            Interaction::Hovered => style.hover,
            Interaction::None => style.normal,
        });
    }
}
```

### Layering & Depth

Bevy UI is flat by default. Create depth through:

- **Absolute positioned overlays**: Status effects, tooltips, modal backgrounds
- **Alpha transparency**: Background panels at 0.8-0.95 alpha feel more atmospheric than solid colors
- **Border radius variation**: Sharp corners feel harsh/industrial, rounded corners feel soft/friendly
- **Nested containers**: Group related elements with subtle background color differences
- **Z-ordering via spawn order**: Later children render on top

```rust
// Create depth with layered container
parent.spawn((
    Node {
        padding: UiRect::all(Val::Px(16.0)),
        ..default()
    },
    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.9)),
    BorderRadius::all(Val::Px(12.0)),
)).with_children(|panel| {
    // Inner content area with slightly different bg
    panel.spawn((
        Node {
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.5)),
        BorderRadius::all(Val::Px(8.0)),
    ));
});
```

## Asset Integration Patterns

### When to Use Assets

- **Decorative frames**: Ornate borders around health bars, skill slots, portraits
- **Icons**: Spell icons, item images, status effect indicators
- **Background textures**: Parchment, metal, stone textures for panels
- **Complex shapes**: Anything beyond rectangles with rounded corners

### Integration Pattern: Programmatic Core + Asset Frame

```rust
// Asset frame as container
parent.spawn((
    ImageNode::new(asset_server.load("ui/frame_ornate.png")),
    Node {
        width: Val::Px(220.0),
        height: Val::Px(40.0),
        padding: UiRect::all(Val::Px(8.0)), // Account for frame border
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    },
)).with_children(|frame| {
    // Programmatic health bar inside
    frame.spawn((/* dynamic bar code */));
});
```

### Integration Pattern: Asset Background + Dynamic Overlay

```rust
// Spell slot with icon background and cooldown overlay
parent.spawn((
    Node {
        width: Val::Px(64.0),
        height: Val::Px(64.0),
        ..default()
    },
)).with_children(|slot| {
    // Spell icon (asset)
    slot.spawn((
        ImageNode::new(spell_icon_handle),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
    ));

    // Cooldown overlay (programmatic, fills from bottom)
    slot.spawn((
        CooldownOverlay,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(0.0), // Updated dynamically
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    ));
});
```

## Anti-Patterns to Avoid

**NEVER** fall into these generic traps:

- **Default grays**: `Color::srgb(0.5, 0.5, 0.5)` backgrounds scream placeholder
- **Arbitrary spacing**: Random padding values (13px, 27px) instead of a consistent scale
- **Missing interaction states**: Buttons that don't respond to hover/press feel broken
- **Pure black/white**: `(0.0, 0.0, 0.0)` and `(1.0, 1.0, 1.0)` are harsh—add subtle color tints
- **Flat single-layer layouts**: Everything at the same visual depth feels like a spreadsheet
- **Uniform border radius**: Same radius on everything is boring—vary it intentionally
- **Ignoring alpha**: Solid backgrounds everywhere miss the atmospheric potential of transparency
- **Centered everything**: Default centering is lazy—consider asymmetric, intentional placement
- **Inconsistent sizing**: Similar elements at different sizes without clear hierarchy reason

**INSTEAD**: Every value should be intentional. If you can't explain why a color, spacing, or radius has that specific value, reconsider it.

## Implementation Checklist

Before considering UI work complete:

- [ ] Color palette is defined and cohesive (not arbitrary values scattered in code)
- [ ] Spacing follows a consistent scale
- [ ] All interactive elements have hover and pressed states
- [ ] Text hierarchy is clear (sizes and colors differentiate importance)
- [ ] Alpha transparency is used for depth where appropriate
- [ ] Border radii are intentional and consistent with aesthetic direction
- [ ] Dynamic state indicators (bars, cooldowns) update smoothly
- [ ] The UI would be recognizable without any assets—the programmatic styling alone has character

Remember: Bevy UI's constraints are real, but within those constraints, excellent, memorable design is absolutely achievable. Commit to a vision and execute it with precision.
