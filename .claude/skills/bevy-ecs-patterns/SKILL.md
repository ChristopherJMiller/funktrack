---
name: bevy-ecs-patterns
description: Guide Bevy systems, components, and plugins following established codebase patterns. Use when creating game systems, adding ECS components, implementing state-based execution, or building plugins.
---

This skill guides creation of Bevy ECS code following established patterns in the Conduit MOBA codebase. It covers system ordering, component design, plugin composition, and server/client authority patterns.

## When to Use This Skill

- Creating new game systems (movement, damage, effects)
- Adding ECS components with replication
- Implementing state-based execution with `run_if`
- Building plugins that follow the SharedGamePlugin pattern
- Choosing between FixedUpdate and Update schedules
- Designing event-driven vs polling systems

## ECS Thinking

Before writing code, understand the context:

- **Authority**: Is this logic server-authoritative or client-only?
- **Timing**: Does this need deterministic fixed timestep or can it run every frame?
- **Dependencies**: What other systems must run before/after this one?
- **Replication**: Does this component need to sync across the network?

## Plugin Architecture

### The SharedGamePlugin Pattern

The codebase follows a strict client/server/shared separation:

```rust
// game/shared/src/lib.rs
pub struct SharedGamePlugin {
    pub schedule: SimulationSchedule, // Update (client) or FixedUpdate (server)
}

impl Plugin for SharedGamePlugin {
    fn build(&self, app: &mut App) {
        // Register components, events, and shared systems
        app.add_plugins(ComponentsPlugin);
        app.add_plugins(SystemsPlugin { schedule: self.schedule });
    }
}
```

**Server imports:**
```rust
app.add_plugins(SharedGamePlugin { schedule: SimulationSchedule::FixedUpdate });
```

**Client imports:**
```rust
app.add_plugins(SharedGamePlugin { schedule: SimulationSchedule::Update });
```

### Plugin Composition

Break features into focused plugins:

```rust
// Good: Focused plugin
pub struct HarvestingPlugin;

impl Plugin for HarvestingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HarvestStartEvent>();
        app.add_event::<HarvestCompleteEvent>();
        // Register systems...
    }
}

// Bad: Monolithic plugin
pub struct GameplayPlugin; // Does movement, combat, harvesting, respawn...
```

## System Patterns

### Run Conditions

Use `run_if` to conditionally execute systems based on server/client authority:

```rust
/// Run condition: returns true if this instance has server authority
pub fn has_server_authority(authority: Option<Res<ServerAuthority>>) -> bool {
    authority.is_some()
}

/// Run condition: returns true if this is a client (no server authority)
pub fn is_client(authority: Option<Res<ServerAuthority>>) -> bool {
    authority.is_none()
}

// Usage
app.add_systems(Update, damage_system.run_if(has_server_authority));
app.add_systems(Update, visual_effects_system.run_if(is_client));
```

### System Ordering with chain()

Use `chain()` when systems must execute in order:

```rust
app.add_systems(
    FixedUpdate,
    (
        projectile_movement_system,
        projectile_homing_system,
        projectile_collision_system,
        projectile_hit_handler_system,
    )
        .chain()  // Executes in this exact order
        .run_if(has_server_authority),
);
```

### System Grouping Without Order

Systems without dependencies can run in parallel:

```rust
app.add_systems(
    FixedUpdate,
    (
        damage_effect_system,
        heal_effect_system,
        teleport_effect_system,
        slow_effect_system,
    )
        .run_if(has_server_authority),
);
```

### Schedule Selection Decision Tree

```
Need deterministic simulation for rollback? → FixedUpdate (64Hz)
├── Server game logic (movement, combat, spawning) → FixedUpdate
├── Client prediction systems → FixedUpdate (run_if is_client)
└── Physics simulation → FixedUpdate

Runs on frame timing? → Update
├── Visual effects (particles, animations) → Update
├── UI updates → Update
├── Camera systems → Update
└── Input detection (then buffer to FixedPreUpdate) → Update

After rendering decisions? → PostUpdate
└── Sync Position to Transform for rendering → PostUpdate
```

## Component Design

### Server-Authoritative Components

Components that represent game state should be server-authoritative:

```rust
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct Dead;

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct RespawnTimer {
    pub remaining: Timer,
}
```

### Replicated Components

Components that need network replication require specific traits:

```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
pub struct SpellCooldowns {
    pub q: Timer,
    pub w: Timer,
    pub e: Timer,
    pub r: Timer,
}
```

**Required traits for Lightyear replication:**
- `Serialize`, `Deserialize` - Network serialization
- `Clone` - Required by Lightyear
- `Reflect` (optional) - For editor integration

### Marker Components

Use empty marker components for entity categorization:

```rust
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Projectile;

#[derive(Component)]
pub struct ManaNode;
```

### Newtype Components

Wrap primitives in newtypes for type safety:

```rust
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct MovementSpeed(pub f32);

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct PlayerId(pub u64);  // Use client_id, not Entity
```

## Event-Driven vs Polling

### Event-Driven Pattern

Use events for discrete occurrences:

```rust
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: Option<Entity>,
}

// Producer
fn spell_hit_system(mut damage_events: EventWriter<DamageEvent>) {
    damage_events.write(DamageEvent { target, amount: 50.0, source: Some(caster) });
}

// Consumer
fn apply_damage_system(
    mut events: EventReader<DamageEvent>,
    mut health_query: Query<&mut Health>,
) {
    for event in events.read() {
        if let Ok(mut health) = health_query.get_mut(event.target) {
            health.current -= event.amount;
        }
    }
}
```

### Polling Pattern

Use polling for continuous state checks:

```rust
fn respawn_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut RespawnTimer)>,
) {
    for (entity, mut timer) in &mut query {
        timer.remaining.tick(time.delta());
        if timer.remaining.finished() {
            commands.entity(entity).remove::<Dead>();
            commands.entity(entity).remove::<RespawnTimer>();
        }
    }
}
```

## Resource Patterns

### Singleton Resources

Use resources for global game state:

```rust
#[derive(Resource)]
pub struct MatchState {
    pub phase: MatchPhase,
    pub timer: Timer,
    pub team_scores: [u32; 2],
}

// Insert on startup
commands.insert_resource(MatchState::default());
```

### Replicated Resources via Component

For replicated state, use a singleton entity with a component:

```rust
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct ReplicatedMatchState {
    pub phase: MatchPhase,
    pub remaining_secs: f32,
}

// Spawn singleton on server
commands.spawn((ReplicatedMatchState::default(), Replicate::to_clients(NetworkTarget::All)));

// Sync to local resource on client
fn sync_component_to_match_state(
    query: Query<&ReplicatedMatchState>,
    mut match_state: ResMut<MatchState>,
) {
    if let Ok(replicated) = query.single() {
        match_state.phase = replicated.phase;
        // ...
    }
}
```

## Anti-Patterns to Avoid

**DON'T use Entity IDs for cross-network references:**
```rust
// Bad: Entity IDs are local to each Bevy instance
pub struct ControlledBy(pub Entity);

// Good: Use consistent identifiers
pub struct PlayerId(pub u64);  // client_id from netcode
```

**DON'T run game logic on client without prediction setup:**
```rust
// Bad: Client runs authoritative logic
app.add_systems(Update, damage_system);

// Good: Only server runs, or client runs with prediction
app.add_systems(Update, damage_system.run_if(has_server_authority));
```

**DON'T modify Transform directly for replicated positions:**
```rust
// Bad: Transform doesn't implement Ease for interpolation
transform.translation = new_pos;

// Good: Use Position component, sync to Transform for rendering
position.0 = new_pos;
```

**DON'T forget system ordering for dependent systems:**
```rust
// Bad: Harvest tick might run before harvester count updates
app.add_systems(Update, (harvest_tick_system, harvester_count_system));

// Good: Chain dependent systems
app.add_systems(Update, (harvester_count_system, harvest_tick_system).chain());
```

## Implementation Checklist

Before completing ECS work:

- [ ] Components have correct derive traits for replication
- [ ] Systems use appropriate `run_if` conditions for authority
- [ ] System ordering is explicit with `chain()` where needed
- [ ] Events are used for discrete occurrences, polling for continuous state
- [ ] Resources are used for global state, not singleton entities (unless replicated)
- [ ] Entity references use stable IDs (PlayerId), not Entity
- [ ] Position is used for replicated coordinates, synced to Transform on client
- [ ] Plugin follows SharedGamePlugin pattern for server/client code sharing

## Reference Files

- `game/shared/src/systems/mod.rs` - System registration patterns
- `game/shared/src/protocol.rs` - Component replication registration
- `game/shared/src/components/` - Component definitions
- `game/shared/src/resources.rs` - Resource patterns and authority checks
