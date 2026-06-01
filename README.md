# lau-symmetry-engine

**17 wallpaper groups, symmetry detection, vibe field analysis, and geometric pattern generation in Rust.**

Every crystal lattice, every tile mosaic, every Islamic geometric design, every wallpaper pattern — they all belong to one of exactly 17 symmetry types in 2D, classified by Fedorov in 1891. This crate implements the complete taxonomy: all 17 wallpaper groups with their symmetry operations, a scalar field (`VibeField`) that can detect, measure, and enforce symmetry, a `RoomSymmetry` system for depositing and withdrawing energy, and a `SymmetryRegistry` for managing multiple rooms with global statistics.

---

## What This Does

- **17 wallpaper groups** — `P1` through `P6M`, each with its canonical symmetry operations (rotations, reflections, glide reflections)
- **Symmetry detection** — measure how well a field obeys each symmetry type; detect the best-fitting group
- **Symmetry enforcement** — apply symmetry by averaging values across orbits (symmetric copies)
- **VibeField** — a discretized 2D scalar field with bilinear interpolation, gradient computation, energy tracking, and fractal dimension estimation
- **RoomSymmetry** — deposit/withdraw energy at spatial coordinates; analyze and score symmetry
- **SymmetryRegistry** — manage multiple rooms, rank by symmetry, compute global statistics
- **Full serde support** — serialize/deserialize fields, rooms, and registries to JSON

---

## Key Idea

Symmetry is measured as **average absolute deviation under symmetry operations**. For a field $V$ and a symmetry operation $g$:

$$\epsilon(V, g) = \frac{1}{|V|} \sum_{(x,y) \in V} |V(x,y) - V(g(x,y))|$$

A field has symmetry group $G$ if $\epsilon(V, g) \approx 0$ for all $g \in G$. The crate tests rotations (2-fold, 3-fold, 4-fold, 6-fold), reflections, and glide reflections, then picks the group with lowest error.

When symmetry is *enforced* (`apply_symmetry`), values are averaged across each orbit $\{g(p) : g \in G\}$, creating a perfectly symmetric field.

---

## Install

```toml
[dependencies]
lau-symmetry-engine = { git = "https://github.com/SuperInstance/lau-symmetry-engine" }
```

Or from source:

```bash
git clone https://github.com/SuperInstance/lau-symmetry-engine.git
cd lau-symmetry-engine
cargo build
```

Dependencies: `serde` + `serde_json`.

---

## Quick Start

### Symmetry Detection

```rust
use lau_symmetry_engine::{VibeField, SymmetryGroup};

let mut field = VibeField::new(1.0);

// Create a reflection-symmetric pattern
field.set(1, 0, 5.0);
field.set(-1, 0, 5.0);
field.set(0, 1, 3.0);
field.set(0, -1, 3.0);

let groups = field.detect_symmetry();
// Reflection has near-zero error → detected
```

### Enforcing Symmetry

```rust
let mut field = VibeField::new(1.0);
field.set(1, 1, 5.0);   // reflects to (1, -1)
field.set(1, -1, 1.0);  // asymmetric partner

field.apply_symmetry(&SymmetryGroup::Reflection);
// Both (1,1) and (1,-1) now equal (5.0 + 1.0) / 2.0 = 3.0
```

### Room Energy Management

```rust
use lau_symmetry_engine::RoomSymmetry;

let mut room = RoomSymmetry::new("kitchen", 1.0);

room.deposit(1.0, 0.0, 5.0);
room.deposit(-1.0, 0.0, 5.0);  // symmetric deposit

room.analyze();
println!("Symmetry score: {:.2}", room.symmetry_score());
println!("Primary group: {:?}", room.primary_group);
println!("Fractal dim: {:.2}", room.fractal_dimension());
```

### Multi-Room Registry

```rust
use lau_symmetry_engine::SymmetryRegistry;

let mut reg = SymmetryRegistry::new();
reg.register("kitchen", 1.0);
reg.register("bedroom", 1.0);

reg.deposit("kitchen", 1.0, 0.0, 5.0);
reg.deposit("kitchen", -1.0, 0.0, 5.0);
reg.deposit("bedroom", 1.0, 2.0, 3.0);

reg.analyze_all();

let most = reg.most_symmetric().unwrap();
let least = reg.least_symmetric().unwrap();

let stats = reg.registry_stats();
println!("Rooms: {}, avg score: {:.2}", stats.total_rooms, stats.avg_symmetry_score);
```

### Bilinear Interpolation & Gradients

```rust
let mut field = VibeField::new(1.0);
field.set(0, 0, 0.0);
field.set(1, 0, 1.0);
field.set(0, 1, 0.0);
field.set(1, 1, 1.0);

let val = field.interpolate(0.5, 0.5); // → 0.5
let (dx, dy) = field.gradient_at(1, 0);
```

---

## API Reference

### Core Types

| Type | Description |
|---|---|
| `Point2D` | 2D point with `x`, `y` |
| `WallpaperType` | Enum of all 17 wallpaper groups (`P1`, `P2`, `PM`, `PG`, `CM`, `PMM`, `PMG`, `PGG`, `CMM`, `P4`, `P4M`, `P4G`, `P3`, `P3M1`, `P31M`, `P6`, `P6M`) |
| `SymmetryGroup` | Identity, Reflection, Rotation(n), Translation, GlideReflection, Wallpaper(WallpaperType) |
| `SymmetryOperation` | Rotate, Reflect, Translate, GlideReflect — each with an `apply(Point2D) → Point2D` method |

### `VibeField`

A 2D scalar field backed by a `HashMap<(i32, i32), f64>`.

| Method | Description |
|---|---|
| `new(resolution)` | Create field with grid spacing |
| `set(x, y, value)` / `get(x, y)` | Grid cell access |
| `total_energy()` | Sum of all values |
| `symmetry_error(group)` | Mean absolute deviation under symmetry operations |
| `detect_symmetry()` | Test all groups; return sorted by ascending error |
| `apply_symmetry(group)` | Enforce symmetry by averaging orbits |
| `interpolate(x, y)` | Bilinear interpolation at continuous coordinates |
| `gradient_at(x, y)` | Central-difference gradient ∂V/∂x, ∂V/∂y |

### `RoomSymmetry`

| Method | Description |
|---|---|
| `new(room_id, resolution)` | Create a room |
| `deposit(x, y, amount)` | Add energy at position |
| `withdraw(x, y, amount)` → `bool` | Remove energy (fails if insufficient) |
| `analyze()` | Detect symmetry groups, set `primary_group` |
| `symmetry_score()` → `f64` | 0.0 (no symmetry) to ~1.0 (perfect) |
| `fractal_dimension()` → `f64` | Box-counting dimension estimate |
| `is_conserved()` → `bool` | All values finite (no NaN/inf) |

### `SymmetryRegistry`

| Method | Description |
|---|---|
| `new()` / `default()` | Empty registry |
| `register(room_id, resolution)` | Add a room |
| `deposit(room, x, y, amount)` | Deposit into a named room |
| `analyze_all()` | Run analysis on all rooms |
| `most_symmetric()` / `least_symmetric()` | Extreme rooms by score |
| `symmetry_distribution()` → `HashMap<SymmetryGroup, usize>` | Count of rooms per group |
| `registry_stats()` → `SymmetryStats` | Aggregate statistics |

---

## How It Works

### Symmetry Error Computation

For each symmetry operation $g$ in a group, the engine:

1. Takes each grid point $(x, y)$ with value $V(x,y)$
2. Applies the operation: $g(x, y) \to (x', y')$
3. Rounds to nearest grid cell
4. Computes $|V(x,y) - V(x', y')|$
5. Averages over all points and operations

If the mean error is below a threshold (0.5), the symmetry is considered present.

### Symmetry Enforcement (Orbit Averaging)

When enforcing a group:

1. For each point, build its **orbit** — the set of all points reachable by applying symmetry operations
2. Compute the average value across the orbit
3. Set all orbit points to the average

This creates a field that is exactly symmetric under the given group.

### Fractal Dimension (Box-Counting)

Uses the box-counting method:

1. For scale factors $s = 2^0, 2^1, 2^2, 2^3, 2^4$
2. Count how many boxes of size $s$ contain at least one non-zero cell
3. Fit a line to $\log(\text{count})$ vs $\log(s)$ via linear regression
4. The slope is the box-counting (fractal) dimension

### Wallpaper Group Operations

Each of the 17 wallpaper groups maps to a canonical set of symmetry operations:

| Group | Operations |
|---|---|
| `P1` | None (translations only) |
| `P2` | 180° rotation |
| `PM` | Reflection across x-axis |
| `PG` | Glide reflection along x-axis |
| `CM` | Reflection + diagonal glide |
| `PMM` | Two perpendicular reflections + 180° rotation |
| `PMG` | Reflection + perpendicular glide |
| `PGG` | Two perpendicular glides |
| `CMM` | Two perpendicular reflections + 180° rotation (rhombic) |
| `P4` | 90° rotation |
| `P4M` | 90° rotation + mirror |
| `P4G` | 90° rotation + 45° mirror |
| `P3` | 120° rotation |
| `P3M1` | 120° rotation + mirror |
| `P31M` | 120° rotation + 30° mirror |
| `P6` | 60° rotation |
| `P6M` | 60° rotation + mirror |

---

## The Math

### Wallpaper Groups (Fedorov, 1891)

There are exactly **17 distinct wallpaper groups** in 2D — discrete groups of isometries (rotations, reflections, glide reflections, translations) that leave a pattern invariant. This was proven by Evgraf Fedorov in 1891. The 17 groups are classified by their:

- **Rotational order**: 1 (none), 2, 3, 4, or 6 (these are the only compatible orders for 2D lattices — the **crystallographic restriction theorem**)
- **Mirror lines**: present or absent
- **Glide reflections**: reflections composed with half-translations

### Crystallographic Restriction Theorem

A 2D lattice can only support rotations of order $n$ where $\phi(2\cos(2\pi/n))$ is rational, giving $n \in \{1, 2, 3, 4, 6\}$. Five-fold symmetry is impossible in a periodic lattice (which is why quasicrystals were such a surprise).

### Symmetry Error Metric

$$\epsilon(V, G) = \frac{1}{|V| \cdot |G|} \sum_{g \in G} \sum_{p \in V} |V(p) - V(g \cdot p)|$$

where $V(p)$ is the field value at point $p$, $G$ is the symmetry group, and $g \cdot p$ is the image of $p$ under operation $g$.

### Bilinear Interpolation

For continuous coordinate $(x, y)$ between grid cells:

$$V(x, y) \approx V_{00}(1-f_x)(1-f_y) + V_{10} f_x(1-f_y) + V_{01}(1-f_x)f_y + V_{11} f_x f_y$$

where $f_x, f_y$ are the fractional offsets within the cell.

### Central Difference Gradient

$$\frac{\partial V}{\partial x}\bigg|_{(i,j)} \approx \frac{V(i+1, j) - V(i-1, j)}{2h}$$

### Box-Counting Fractal Dimension

$$D = -\lim_{\epsilon \to 0} \frac{\log N(\epsilon)}{\log \epsilon}$$

estimated by linear regression on $\log N$ vs $\log \epsilon$ over 5 scale levels.

### Symmetry Score

$$S = 1 - \min\!\left(1,\; \frac{\epsilon(V, G)}{|E|}\right)$$

where $E = \sum V(x,y)$ is the total energy. Perfect symmetry gives $S = 1$; no symmetry gives $S = 0$.

---

## Testing

```bash
cargo test
```

**48 tests** covering:

- **Point2D** — creation
- **VibeField** — creation, set/get, total energy (populated and empty), symmetry error (identity, reflection symmetric, reflection asymmetric, 2-fold rotation, 4-fold rotation), symmetry enforcement (orbit averaging), symmetry detection (empty field), bilinear interpolation (center, corner, outside grid), gradient (uniform field, zero field), resolution scaling
- **RoomSymmetry** — creation, deposit (single, accumulating), withdraw (success, insufficient, exact), analysis, symmetry score (no analysis), conservation check, fractal dimension (empty, with data)
- **SymmetryRegistry** — creation, register, deposit (valid room, missing room), analyze_all, most/least symmetric, stats (empty, populated), symmetry distribution, default
- **Serde round-trips** — SymmetryGroup, VibeField, RoomSymmetry, SymmetryRegistry (all survive JSON encode → decode)
- **Symmetry operations** — rotate (180° at origin), reflect (x-axis), translate
- **All 17 wallpaper types** — no panics on any group's operations

---

## License

MIT
