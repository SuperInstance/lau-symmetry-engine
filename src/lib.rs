use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Helper module for serde of HashMap<(i32,i32), f64> as JSON-compatible vec
mod serde_field_map {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct Entry {
        x: i32,
        y: i32,
        v: f64,
    }

    pub fn serialize<S: Serializer>(
        map: &HashMap<(i32, i32), f64>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let entries: Vec<Entry> = map
            .iter()
            .map(|(&(x, y), &v)| Entry { x, y, v })
            .collect();
        entries.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<HashMap<(i32, i32), f64>, D::Error> {
        let entries: Vec<Entry> = Vec::deserialize(d)?;
        let mut map = HashMap::new();
        for e in entries {
            map.insert((e.x, e.y), e.v);
        }
        Ok(map)
    }
}

// ── Wallpaper group types (17 groups) ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WallpaperType {
    P1,
    P2,
    PM,
    PG,
    CM,
    PMM,
    PMG,
    PGG,
    CMM,
    P4,
    P4M,
    P4G,
    P3,
    P3M1,
    P31M,
    P6,
    P6M,
}

// ── Symmetry group ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SymmetryGroup {
    Identity,
    Reflection,
    Rotation(u32),
    Translation,
    GlideReflection,
    Wallpaper(WallpaperType),
}

impl std::hash::Hash for SymmetryGroup {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        if let SymmetryGroup::Rotation(n) = self {
            n.hash(state);
        }
        if let SymmetryGroup::Wallpaper(wt) = self {
            std::mem::discriminant(wt).hash(state);
        }
    }
}

impl Eq for SymmetryGroup {}

// ── Symmetry operations ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SymmetryOperation {
    Rotate { angle: f64, center: (f64, f64) },
    Reflect { axis: f64 },
    Translate { dx: f64, dy: f64 },
    GlideReflect { axis: f64, distance: f64 },
}

impl SymmetryOperation {
    /// Apply this operation to a point, returning the transformed point.
    pub fn apply(&self, p: Point2D) -> Point2D {
        match self {
            SymmetryOperation::Rotate { angle, center } => {
                let dx = p.x - center.0;
                let dy = p.y - center.1;
                let c = angle.cos();
                let s = angle.sin();
                Point2D {
                    x: center.0 + dx * c - dy * s,
                    y: center.1 + dx * s + dy * c,
                }
            }
            SymmetryOperation::Reflect { axis } => {
                // Reflect across a line through origin at given angle
                let c = (2.0 * axis).cos();
                let s = (2.0 * axis).sin();
                Point2D {
                    x: p.x * c + p.y * s,
                    y: p.x * s - p.y * c,
                }
            }
            SymmetryOperation::Translate { dx, dy } => Point2D {
                x: p.x + dx,
                y: p.y + dy,
            },
            SymmetryOperation::GlideReflect { axis, distance } => {
                let reflected = SymmetryOperation::Reflect { axis: *axis }.apply(p);
                let dx = distance * axis.cos();
                let dy = distance * axis.sin();
                Point2D {
                    x: reflected.x + dx,
                    y: reflected.y + dy,
                }
            }
        }
    }
}

// ── Point2D ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

// ── VibeField ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeField {
    #[serde(with = "serde_field_map")]
    pub values: HashMap<(i32, i32), f64>,
    pub symmetry: Option<SymmetryGroup>,
    pub resolution: f64,
}

impl VibeField {
    pub fn new(resolution: f64) -> Self {
        Self {
            values: HashMap::new(),
            symmetry: None,
            resolution,
        }
    }

    pub fn set(&mut self, x: i32, y: i32, value: f64) {
        self.values.insert((x, y), value);
    }

    pub fn get(&self, x: i32, y: i32) -> f64 {
        self.values.get(&(x, y)).copied().unwrap_or(0.0)
    }

    pub fn total_energy(&self) -> f64 {
        self.values.values().sum()
    }

    pub fn apply_symmetry(&mut self, group: &SymmetryGroup) {
        if self.values.is_empty() {
            self.symmetry = Some(group.clone());
            return;
        }

        let ops = symmetry_ops_for_group(group);
        if ops.is_empty() {
            self.symmetry = Some(group.clone());
            return;
        }

        // Collect all points including their symmetric counterparts
        let original: Vec<((i32, i32), f64)> = self.values.iter().map(|(&k, &v)| (k, v)).collect();

        for op in &ops {
            for &((ix, iy), _val) in &original {
                let p = op.apply(Point2D {
                    x: ix as f64 * self.resolution,
                    y: iy as f64 * self.resolution,
                });
                let gx = (p.x / self.resolution).round() as i32;
                let gy = (p.y / self.resolution).round() as i32;
                self.values.entry((gx, gy)).or_insert(0.0);
            }
        }

        // Average values across orbits
        let points: Vec<(i32, i32)> = self.values.keys().copied().collect();
        let mut visited = HashMap::new();

        for &(ix, iy) in &points {
            if visited.contains_key(&(ix, iy)) {
                continue;
            }

            // Build orbit
            let mut orbit = vec![(ix, iy)];
            for op in &ops {
                let p = op.apply(Point2D {
                    x: ix as f64 * self.resolution,
                    y: iy as f64 * self.resolution,
                });
                let gx = (p.x / self.resolution).round() as i32;
                let gy = (p.y / self.resolution).round() as i32;
                if !orbit.contains(&(gx, gy)) {
                    orbit.push((gx, gy));
                }
            }

            // Compute average
            let avg: f64 = orbit
                .iter()
                .map(|&(ox, oy)| self.values.get(&(ox, oy)).copied().unwrap_or(0.0))
                .sum::<f64>()
                / orbit.len() as f64;

            for &pt in &orbit {
                self.values.insert(pt, avg);
                visited.insert(pt, true);
            }
        }

        self.symmetry = Some(group.clone());
    }

    pub fn symmetry_error(&self, group: &SymmetryGroup) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        let ops = symmetry_ops_for_group(group);
        if ops.is_empty() {
            return 0.0;
        }

        let mut total_error = 0.0;
        let mut count = 0;

        for (&(ix, iy), &val) in &self.values {
            let p = Point2D {
                x: ix as f64 * self.resolution,
                y: iy as f64 * self.resolution,
            };

            for op in &ops {
                let tp = op.apply(p);
                let gx = (tp.x / self.resolution).round() as i32;
                let gy = (tp.y / self.resolution).round() as i32;
                let other = self.values.get(&(gx, gy)).copied().unwrap_or(0.0);
                total_error += (val - other).abs();
                count += 1;
            }
        }

        if count == 0 {
            0.0
        } else {
            total_error / count as f64
        }
    }

    pub fn detect_symmetry(&self) -> Vec<SymmetryGroup> {
        let mut results = Vec::new();

        // Test rotations
        for n in [2u32, 3, 4, 6] {
            let group = SymmetryGroup::Rotation(n);
            if self.symmetry_error(&group) < 0.5 {
                results.push(group);
            }
        }

        // Test reflection
        if self.symmetry_error(&SymmetryGroup::Reflection) < 0.5 {
            results.push(SymmetryGroup::Reflection);
        }

        // Test glide reflection
        if self.symmetry_error(&SymmetryGroup::GlideReflection) < 0.5 {
            results.push(SymmetryGroup::GlideReflection);
        }

        // Always include identity as fallback
        if self.symmetry_error(&SymmetryGroup::Identity) < 0.5 {
            results.push(SymmetryGroup::Identity);
        }

        // Sort by lowest error
        results.sort_by(|a, b| {
            self.symmetry_error(a)
                .partial_cmp(&self.symmetry_error(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    pub fn interpolate(&self, x: f64, y: f64) -> f64 {
        let gx = x / self.resolution;
        let gy = y / self.resolution;

        let x0 = gx.floor() as i32;
        let y0 = gy.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let fx = gx - x0 as f64;
        let fy = gy - y0 as f64;

        let v00 = self.get(x0, y0);
        let v10 = self.get(x1, y0);
        let v01 = self.get(x0, y1);
        let v11 = self.get(x1, y1);

        v00 * (1.0 - fx) * (1.0 - fy)
            + v10 * fx * (1.0 - fy)
            + v01 * (1.0 - fx) * fy
            + v11 * fx * fy
    }

    pub fn gradient_at(&self, x: i32, y: i32) -> (f64, f64) {
        let h = self.resolution;
        let dx = (self.get(x + 1, y) - self.get(x - 1, y)) / (2.0 * h);
        let dy = (self.get(x, y + 1) - self.get(x, y - 1)) / (2.0 * h);
        (dx, dy)
    }
}

// ── RoomSymmetry ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSymmetry {
    pub room_id: String,
    pub field: VibeField,
    pub detected_groups: Vec<SymmetryGroup>,
    pub primary_group: Option<SymmetryGroup>,
}

impl RoomSymmetry {
    pub fn new(room_id: &str, resolution: f64) -> Self {
        Self {
            room_id: room_id.to_string(),
            field: VibeField::new(resolution),
            detected_groups: Vec::new(),
            primary_group: None,
        }
    }

    pub fn deposit(&mut self, x: f64, y: f64, amount: f64) {
        let gx = (x / self.field.resolution).round() as i32;
        let gy = (y / self.field.resolution).round() as i32;
        let current = self.field.get(gx, gy);
        self.field.set(gx, gy, current + amount);
    }

    pub fn withdraw(&mut self, x: f64, y: f64, amount: f64) -> bool {
        let gx = (x / self.field.resolution).round() as i32;
        let gy = (y / self.field.resolution).round() as i32;
        let current = self.field.get(gx, gy);
        if current >= amount {
            self.field.set(gx, gy, current - amount);
            true
        } else {
            false
        }
    }

    pub fn analyze(&mut self) {
        self.detected_groups = self.field.detect_symmetry();
        self.primary_group = self.detected_groups.first().cloned();
    }

    pub fn symmetry_score(&self) -> f64 {
        match &self.primary_group {
            None => 0.0,
            Some(group) => {
                if matches!(group, SymmetryGroup::Identity) {
                    return 0.0;
                }
                let error = self.field.symmetry_error(group);
                let energy = self.field.total_energy().abs();
                if energy == 0.0 {
                    return 1.0;
                }
                (1.0 - (error / energy).min(1.0)).max(0.0)
            }
        }
    }

    pub fn fractal_dimension(&self) -> f64 {
        if self.field.values.is_empty() {
            return 0.0;
        }

        // Box-counting dimension estimate
        let mut counts = Vec::new();
        for scale_exp in 0..5i32 {
            let scale = 2i32.pow(scale_exp as u32);
            let mut boxes = std::collections::HashSet::new();
            for &(x, y) in self.field.values.keys() {
                boxes.insert((x / scale, y / scale));
            }
            counts.push((scale as f64, boxes.len() as f64));
        }

        // Linear regression on log-log
        let n = counts.len() as f64;
        if n < 2.0 {
            return 0.0;
        }

        let sum_x: f64 = counts.iter().map(|(s, _)| s.ln()).sum();
        let sum_y: f64 = counts.iter().map(|(_, c)| c.ln()).sum();
        let sum_xy: f64 = counts.iter().map(|(s, c)| s.ln() * c.ln()).sum();
        let sum_x2: f64 = counts.iter().map(|(s, _)| s.ln().powi(2)).sum();

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom == 0.0 {
            return 0.0;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        slope.abs()
    }

    pub fn is_conserved(&self) -> bool {
        // Energy is conserved if no values are NaN or infinite
        self.field
            .values
            .values()
            .all(|v| v.is_finite())
    }
}

// ── SymmetryStats ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymmetryStats {
    pub total_rooms: usize,
    pub avg_symmetry_score: f64,
    pub most_common_group: Option<SymmetryGroup>,
    pub avg_fractal_dimension: f64,
}

// ── SymmetryRegistry ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymmetryRegistry {
    pub rooms: HashMap<String, RoomSymmetry>,
}

impl SymmetryRegistry {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    pub fn register(&mut self, room_id: &str, resolution: f64) {
        self.rooms
            .insert(room_id.to_string(), RoomSymmetry::new(room_id, resolution));
    }

    pub fn deposit(&mut self, room: &str, x: f64, y: f64, amount: f64) {
        if let Some(r) = self.rooms.get_mut(room) {
            r.deposit(x, y, amount);
        }
    }

    pub fn analyze_all(&mut self) {
        for room in self.rooms.values_mut() {
            room.analyze();
        }
    }

    pub fn most_symmetric(&self) -> Option<&RoomSymmetry> {
        self.rooms.values().max_by(|a, b| {
            a.symmetry_score()
                .partial_cmp(&b.symmetry_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn least_symmetric(&self) -> Option<&RoomSymmetry> {
        self.rooms.values().min_by(|a, b| {
            a.symmetry_score()
                .partial_cmp(&b.symmetry_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    pub fn symmetry_distribution(&self) -> HashMap<SymmetryGroup, usize> {
        let mut dist = HashMap::new();
        for room in self.rooms.values() {
            if let Some(ref group) = room.primary_group {
                *dist.entry(group.clone()).or_insert(0) += 1;
            }
        }
        dist
    }

    pub fn registry_stats(&self) -> SymmetryStats {
        let n = self.rooms.len();
        if n == 0 {
            return SymmetryStats {
                total_rooms: 0,
                avg_symmetry_score: 0.0,
                most_common_group: None,
                avg_fractal_dimension: 0.0,
            };
        }

        let avg_score = self.rooms.values().map(|r| r.symmetry_score()).sum::<f64>() / n as f64;
        let avg_fd = self
            .rooms
            .values()
            .map(|r| r.fractal_dimension())
            .sum::<f64>()
            / n as f64;

        let dist = self.symmetry_distribution();
        let most_common = dist
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(group, _)| group);

        SymmetryStats {
            total_rooms: n,
            avg_symmetry_score: avg_score,
            most_common_group: most_common,
            avg_fractal_dimension: avg_fd,
        }
    }
}

impl Default for SymmetryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn symmetry_ops_for_group(group: &SymmetryGroup) -> Vec<SymmetryOperation> {
    match group {
        SymmetryGroup::Identity => vec![],
        SymmetryGroup::Reflection => {
            // Reflect across x-axis (axis = 0)
            vec![SymmetryOperation::Reflect { axis: 0.0 }]
        }
        SymmetryGroup::Rotation(n) => {
            let angle = 2.0 * std::f64::consts::PI / *n as f64;
            vec![SymmetryOperation::Rotate {
                angle,
                center: (0.0, 0.0),
            }]
        }
        SymmetryGroup::Translation => {
            vec![SymmetryOperation::Translate { dx: 1.0, dy: 0.0 }]
        }
        SymmetryGroup::GlideReflection => {
            vec![SymmetryOperation::GlideReflect {
                axis: 0.0,
                distance: 1.0,
            }]
        }
        SymmetryGroup::Wallpaper(wt) => wallpaper_ops(wt),
    }
}

fn wallpaper_ops(wt: &WallpaperType) -> Vec<SymmetryOperation> {
    match wt {
        WallpaperType::P1 => vec![], // No symmetry, only translations
        WallpaperType::P2 => vec![SymmetryOperation::Rotate {
            angle: std::f64::consts::PI,
            center: (0.0, 0.0),
        }],
        WallpaperType::PM => vec![SymmetryOperation::Reflect { axis: 0.0 }],
        WallpaperType::PG => vec![SymmetryOperation::GlideReflect {
            axis: 0.0,
            distance: 1.0,
        }],
        WallpaperType::CM => vec![
            SymmetryOperation::Reflect { axis: 0.0 },
            SymmetryOperation::GlideReflect {
                axis: std::f64::consts::FRAC_PI_4,
                distance: 1.0,
            },
        ],
        WallpaperType::PMM => vec![
            SymmetryOperation::Reflect { axis: 0.0 },
            SymmetryOperation::Reflect {
                axis: std::f64::consts::FRAC_PI_2,
            },
            SymmetryOperation::Rotate {
                angle: std::f64::consts::PI,
                center: (0.0, 0.0),
            },
        ],
        WallpaperType::PMG => vec![
            SymmetryOperation::Reflect { axis: 0.0 },
            SymmetryOperation::GlideReflect {
                axis: std::f64::consts::FRAC_PI_2,
                distance: 1.0,
            },
        ],
        WallpaperType::PGG => vec![
            SymmetryOperation::GlideReflect {
                axis: 0.0,
                distance: 1.0,
            },
            SymmetryOperation::GlideReflect {
                axis: std::f64::consts::FRAC_PI_2,
                distance: 1.0,
            },
        ],
        WallpaperType::CMM => vec![
            SymmetryOperation::Reflect { axis: 0.0 },
            SymmetryOperation::Reflect {
                axis: std::f64::consts::FRAC_PI_2,
            },
            SymmetryOperation::Rotate {
                angle: std::f64::consts::PI,
                center: (0.0, 0.0),
            },
        ],
        WallpaperType::P4 => vec![SymmetryOperation::Rotate {
            angle: std::f64::consts::FRAC_PI_2,
            center: (0.0, 0.0),
        }],
        WallpaperType::P4M => vec![
            SymmetryOperation::Rotate {
                angle: std::f64::consts::FRAC_PI_2,
                center: (0.0, 0.0),
            },
            SymmetryOperation::Reflect { axis: 0.0 },
        ],
        WallpaperType::P4G => vec![
            SymmetryOperation::Rotate {
                angle: std::f64::consts::FRAC_PI_2,
                center: (0.0, 0.0),
            },
            SymmetryOperation::Reflect {
                axis: std::f64::consts::FRAC_PI_4,
            },
        ],
        WallpaperType::P3 => vec![SymmetryOperation::Rotate {
            angle: 2.0 * std::f64::consts::PI / 3.0,
            center: (0.0, 0.0),
        }],
        WallpaperType::P3M1 => vec![
            SymmetryOperation::Rotate {
                angle: 2.0 * std::f64::consts::PI / 3.0,
                center: (0.0, 0.0),
            },
            SymmetryOperation::Reflect { axis: 0.0 },
        ],
        WallpaperType::P31M => vec![
            SymmetryOperation::Rotate {
                angle: 2.0 * std::f64::consts::PI / 3.0,
                center: (0.0, 0.0),
            },
            SymmetryOperation::Reflect {
                axis: std::f64::consts::FRAC_PI_6,
            },
        ],
        WallpaperType::P6 => vec![SymmetryOperation::Rotate {
            angle: std::f64::consts::PI / 3.0,
            center: (0.0, 0.0),
        }],
        WallpaperType::P6M => vec![
            SymmetryOperation::Rotate {
                angle: std::f64::consts::PI / 3.0,
                center: (0.0, 0.0),
            },
            SymmetryOperation::Reflect { axis: 0.0 },
        ],
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point2d_creation() {
        let p = Point2D { x: 1.0, y: 2.0 };
        assert_eq!(p.x, 1.0);
        assert_eq!(p.y, 2.0);
    }

    #[test]
    fn test_vibe_field_new() {
        let field = VibeField::new(1.0);
        assert_eq!(field.resolution, 1.0);
        assert!(field.values.is_empty());
    }

    #[test]
    fn test_vibe_field_set_get() {
        let mut field = VibeField::new(1.0);
        field.set(3, 4, 7.5);
        assert_eq!(field.get(3, 4), 7.5);
        assert_eq!(field.get(0, 0), 0.0);
    }

    #[test]
    fn test_total_energy() {
        let mut field = VibeField::new(1.0);
        field.set(0, 0, 3.0);
        field.set(1, 0, 2.0);
        field.set(0, 1, 5.0);
        assert_eq!(field.total_energy(), 10.0);
    }

    #[test]
    fn test_total_energy_empty() {
        let field = VibeField::new(1.0);
        assert_eq!(field.total_energy(), 0.0);
    }

    #[test]
    fn test_symmetry_identity_error() {
        let mut field = VibeField::new(1.0);
        field.set(0, 0, 1.0);
        assert_eq!(field.symmetry_error(&SymmetryGroup::Identity), 0.0);
    }

    #[test]
    fn test_reflection_error_symmetric() {
        let mut field = VibeField::new(1.0);
        field.set(1, 0, 5.0);
        field.set(-1, 0, 5.0);
        let err = field.symmetry_error(&SymmetryGroup::Reflection);
        assert!(err < 0.01, "reflection error should be near 0, got {err}");
    }

    #[test]
    fn test_reflection_error_asymmetric() {
        let mut field = VibeField::new(1.0);
        field.set(1, 1, 5.0);  // (1,1) reflects to (1,-1)
        field.set(1, -1, 1.0); // asymmetric partner
        let err = field.symmetry_error(&SymmetryGroup::Reflection);
        assert!(err > 0.0, "reflection error should be > 0, got {err}");
    }

    #[test]
    fn test_rotation_2_error_symmetric() {
        let mut field = VibeField::new(1.0);
        field.set(1, 0, 3.0);
        field.set(-1, 0, 3.0);
        let err = field.symmetry_error(&SymmetryGroup::Rotation(2));
        assert!(err < 0.01, "2-fold rotation error should be near 0, got {err}");
    }

    #[test]
    fn test_rotation_4_error() {
        let mut field = VibeField::new(1.0);
        field.set(1, 0, 2.0);
        field.set(0, 1, 2.0);
        field.set(-1, 0, 2.0);
        field.set(0, -1, 2.0);
        let err = field.symmetry_error(&SymmetryGroup::Rotation(4));
        assert!(err < 0.01, "4-fold rotation error should be near 0, got {err}");
    }

    #[test]
    fn test_apply_symmetry_reflection() {
        let mut field = VibeField::new(1.0);
        field.set(1, 1, 5.0);   // reflects to (1,-1)
        field.set(1, -1, 1.0);  // asymmetric partner
        field.apply_symmetry(&SymmetryGroup::Reflection);
        let avg = (5.0 + 1.0) / 2.0;
        assert!((field.get(1, 1) - avg).abs() < 0.01, "got {}", field.get(1, 1));
        assert!((field.get(1, -1) - avg).abs() < 0.01);
    }

    #[test]
    fn test_detect_symmetry_empty() {
        let field = VibeField::new(1.0);
        let groups = field.detect_symmetry();
        assert!(groups.contains(&SymmetryGroup::Identity));
    }

    #[test]
    fn test_interpolate_bilinear() {
        let mut field = VibeField::new(1.0);
        field.set(0, 0, 0.0);
        field.set(1, 0, 1.0);
        field.set(0, 1, 0.0);
        field.set(1, 1, 1.0);
        // At center should be 0.5
        let val = field.interpolate(0.5, 0.5);
        assert!((val - 0.5).abs() < 0.01, "interpolate center: {val}");
    }

    #[test]
    fn test_interpolate_corner() {
        let mut field = VibeField::new(1.0);
        field.set(0, 0, 3.0);
        let val = field.interpolate(0.0, 0.0);
        assert!((val - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_gradient_uniform() {
        let mut field = VibeField::new(1.0);
        field.set(0, 0, 0.0);
        field.set(1, 0, 1.0);
        field.set(2, 0, 2.0);
        let (dx, _dy) = field.gradient_at(1, 0);
        assert!((dx - 1.0).abs() < 0.01, "gradient dx: {dx}");
    }

    #[test]
    fn test_gradient_zero_field() {
        let field = VibeField::new(1.0);
        let (dx, dy) = field.gradient_at(0, 0);
        assert_eq!(dx, 0.0);
        assert_eq!(dy, 0.0);
    }

    #[test]
    fn test_room_symmetry_new() {
        let room = RoomSymmetry::new("test-room", 0.5);
        assert_eq!(room.room_id, "test-room");
        assert_eq!(room.field.resolution, 0.5);
        assert!(room.primary_group.is_none());
    }

    #[test]
    fn test_room_deposit() {
        let mut room = RoomSymmetry::new("room1", 1.0);
        room.deposit(1.0, 2.0, 5.0);
        assert_eq!(room.field.get(1, 2), 5.0);
    }

    #[test]
    fn test_room_deposit_accumulates() {
        let mut room = RoomSymmetry::new("room1", 1.0);
        room.deposit(1.0, 1.0, 3.0);
        room.deposit(1.0, 1.0, 2.0);
        assert_eq!(room.field.get(1, 1), 5.0);
    }

    #[test]
    fn test_room_withdraw_success() {
        let mut room = RoomSymmetry::new("room1", 1.0);
        room.deposit(1.0, 1.0, 5.0);
        assert!(room.withdraw(1.0, 1.0, 3.0));
        assert_eq!(room.field.get(1, 1), 2.0);
    }

    #[test]
    fn test_room_withdraw_insufficient() {
        let mut room = RoomSymmetry::new("room1", 1.0);
        room.deposit(1.0, 1.0, 2.0);
        assert!(!room.withdraw(1.0, 1.0, 5.0));
        assert_eq!(room.field.get(1, 1), 2.0);
    }

    #[test]
    fn test_room_analyze() {
        let mut room = RoomSymmetry::new("room1", 1.0);
        room.deposit(1.0, 0.0, 5.0);
        room.deposit(-1.0, 0.0, 5.0);
        room.analyze();
        assert!(!room.detected_groups.is_empty());
        assert!(room.primary_group.is_some());
    }

    #[test]
    fn test_symmetry_score_no_analysis() {
        let room = RoomSymmetry::new("room1", 1.0);
        assert_eq!(room.symmetry_score(), 0.0);
    }

    #[test]
    fn test_is_conserved() {
        let mut room = RoomSymmetry::new("room1", 1.0);
        room.deposit(0.0, 0.0, 5.0);
        assert!(room.is_conserved());
    }

    #[test]
    fn test_fractal_dimension_empty() {
        let room = RoomSymmetry::new("room1", 1.0);
        assert_eq!(room.fractal_dimension(), 0.0);
    }

    #[test]
    fn test_registry_new() {
        let reg = SymmetryRegistry::new();
        assert!(reg.rooms.is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut reg = SymmetryRegistry::new();
        reg.register("room1", 1.0);
        assert!(reg.rooms.contains_key("room1"));
    }

    #[test]
    fn test_registry_deposit() {
        let mut reg = SymmetryRegistry::new();
        reg.register("room1", 1.0);
        reg.deposit("room1", 1.0, 1.0, 5.0);
        assert_eq!(reg.rooms["room1"].field.get(1, 1), 5.0);
    }

    #[test]
    fn test_registry_deposit_missing_room() {
        let mut reg = SymmetryRegistry::new();
        reg.deposit("ghost", 1.0, 1.0, 5.0); // should not panic
    }

    #[test]
    fn test_registry_analyze_all() {
        let mut reg = SymmetryRegistry::new();
        reg.register("r1", 1.0);
        reg.register("r2", 1.0);
        reg.deposit("r1", 1.0, 0.0, 5.0);
        reg.deposit("r1", -1.0, 0.0, 5.0);
        reg.deposit("r2", 0.0, 0.0, 3.0);
        reg.analyze_all();
        assert!(reg.rooms["r1"].primary_group.is_some());
        assert!(reg.rooms["r2"].primary_group.is_some());
    }

    #[test]
    fn test_registry_most_symmetric() {
        let mut reg = SymmetryRegistry::new();
        reg.register("sym", 1.0);
        reg.register("asym", 1.0);
        // Symmetric room
        reg.deposit("sym", 1.0, 0.0, 5.0);
        reg.deposit("sym", -1.0, 0.0, 5.0);
        // Asymmetric room
        reg.deposit("asym", 1.0, 0.0, 5.0);
        reg.deposit("asym", 2.0, 3.0, 1.0);
        reg.analyze_all();
        let most = reg.most_symmetric().unwrap();
        assert_eq!(most.room_id, "sym");
    }

    #[test]
    fn test_registry_least_symmetric() {
        let mut reg = SymmetryRegistry::new();
        reg.register("sym", 1.0);
        reg.register("asym", 1.0);
        reg.deposit("sym", 1.0, 0.0, 5.0);
        reg.deposit("sym", -1.0, 0.0, 5.0);
        reg.deposit("asym", 1.0, 0.0, 5.0);
        reg.deposit("asym", 2.0, 3.0, 1.0);
        reg.analyze_all();
        let least = reg.least_symmetric().unwrap();
        assert_eq!(least.room_id, "asym");
    }

    #[test]
    fn test_registry_stats_empty() {
        let reg = SymmetryRegistry::new();
        let stats = reg.registry_stats();
        assert_eq!(stats.total_rooms, 0);
    }

    #[test]
    fn test_registry_stats() {
        let mut reg = SymmetryRegistry::new();
        reg.register("r1", 1.0);
        reg.deposit("r1", 1.0, 0.0, 5.0);
        reg.deposit("r1", -1.0, 0.0, 5.0);
        reg.analyze_all();
        let stats = reg.registry_stats();
        assert_eq!(stats.total_rooms, 1);
        assert!(stats.avg_symmetry_score >= 0.0);
    }

    #[test]
    fn test_symmetry_distribution() {
        let mut reg = SymmetryRegistry::new();
        reg.register("r1", 1.0);
        reg.register("r2", 1.0);
        reg.deposit("r1", 1.0, 0.0, 5.0);
        reg.deposit("r1", -1.0, 0.0, 5.0);
        reg.deposit("r2", 1.0, 0.0, 5.0);
        reg.deposit("r2", -1.0, 0.0, 5.0);
        reg.analyze_all();
        let dist = reg.symmetry_distribution();
        assert!(!dist.is_empty());
    }

    #[test]
    fn test_serde_roundtrip_symmetry_group() {
        let groups = vec![
            SymmetryGroup::Identity,
            SymmetryGroup::Reflection,
            SymmetryGroup::Rotation(4),
            SymmetryGroup::Translation,
            SymmetryGroup::GlideReflection,
            SymmetryGroup::Wallpaper(WallpaperType::P6M),
        ];
        let json = serde_json::to_string(&groups).unwrap();
        let decoded: Vec<SymmetryGroup> = serde_json::from_str(&json).unwrap();
        assert_eq!(groups, decoded);
    }

    #[test]
    fn test_serde_roundtrip_vibe_field() {
        let mut field = VibeField::new(0.5);
        field.set(1, 2, 3.14);
        field.symmetry = Some(SymmetryGroup::Rotation(3));
        let json = serde_json::to_string(&field).unwrap();
        let decoded: VibeField = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.resolution, 0.5);
        assert_eq!(decoded.get(1, 2), 3.14);
    }

    #[test]
    fn test_serde_roundtrip_room_symmetry() {
        let mut room = RoomSymmetry::new("test", 1.0);
        room.deposit(1.0, 1.0, 7.0);
        let json = serde_json::to_string(&room).unwrap();
        let decoded: RoomSymmetry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.room_id, "test");
        assert_eq!(decoded.field.get(1, 1), 7.0);
    }

    #[test]
    fn test_serde_roundtrip_registry() {
        let mut reg = SymmetryRegistry::new();
        reg.register("r1", 1.0);
        reg.deposit("r1", 0.0, 0.0, 42.0);
        let json = serde_json::to_string(&reg).unwrap();
        let decoded: SymmetryRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.rooms.len(), 1);
        assert_eq!(decoded.rooms["r1"].field.get(0, 0), 42.0);
    }

    #[test]
    fn test_operation_rotate() {
        let op = SymmetryOperation::Rotate {
            angle: std::f64::consts::PI,
            center: (0.0, 0.0),
        };
        let p = Point2D { x: 1.0, y: 0.0 };
        let result = op.apply(p);
        assert!((result.x - (-1.0)).abs() < 0.001);
        assert!(result.y.abs() < 0.001);
    }

    #[test]
    fn test_operation_reflect() {
        let op = SymmetryOperation::Reflect { axis: 0.0 };
        let p = Point2D { x: 1.0, y: 1.0 };
        let result = op.apply(p);
        // Reflect across x-axis: (x, y) -> (x, -y)
        assert!((result.x - 1.0).abs() < 0.001);
        assert!((result.y - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_operation_translate() {
        let op = SymmetryOperation::Translate { dx: 3.0, dy: 4.0 };
        let p = Point2D { x: 1.0, y: 2.0 };
        let result = op.apply(p);
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_all_wallpaper_types() {
        let all_types = [
            WallpaperType::P1, WallpaperType::P2, WallpaperType::PM,
            WallpaperType::PG, WallpaperType::CM, WallpaperType::PMM,
            WallpaperType::PMG, WallpaperType::PGG, WallpaperType::CMM,
            WallpaperType::P4, WallpaperType::P4M, WallpaperType::P4G,
            WallpaperType::P3, WallpaperType::P3M1, WallpaperType::P31M,
            WallpaperType::P6, WallpaperType::P6M,
        ];
        for wt in all_types {
            let ops = wallpaper_ops(&wt);
            // Each wallpaper type should have valid operations (P1 has none)
            let _ = ops; // just ensure no panic
        }
    }

    #[test]
    fn test_registry_default() {
        let reg = SymmetryRegistry::default();
        assert!(reg.rooms.is_empty());
    }

    #[test]
    fn test_interpolate_outside_grid() {
        let field = VibeField::new(1.0);
        let val = field.interpolate(100.0, 100.0);
        assert_eq!(val, 0.0);
    }

    #[test]
    fn test_vibe_field_resolution() {
        let mut field = VibeField::new(0.5);
        field.set(2, 2, 10.0);
        // Point (2,2) at resolution 0.5 maps to world coords (1.0, 1.0)
        let val = field.interpolate(1.0, 1.0);
        assert!((val - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_room_withdraw_exact() {
        let mut room = RoomSymmetry::new("r", 1.0);
        room.deposit(0.0, 0.0, 5.0);
        assert!(room.withdraw(0.0, 0.0, 5.0));
        assert_eq!(room.field.get(0, 0), 0.0);
    }

    #[test]
    fn test_fractal_dimension_with_data() {
        let mut room = RoomSymmetry::new("r", 1.0);
        // Create some spread data
        for i in 0..10i32 {
            for j in 0..10i32 {
                room.deposit(i as f64, j as f64, 1.0);
            }
        }
        let dim = room.fractal_dimension();
        assert!(dim > 0.0, "fractal dimension should be > 0, got {dim}");
        assert!(dim <= 3.0, "fractal dimension should be reasonable, got {dim}");
    }
}
