# lau-symmetry-engine

17 wallpaper groups, vibe field symmetry detection, and geometric pattern generation. Every crystal, every tile pattern, every Islamic geometric design belongs to one of exactly 17 symmetry types in 2D.

## The concept in 60 seconds

A **wallpaper group** is a 2D symmetry classification. There are exactly 17 of them — proven by Fedorov in 1891. Every repeating 2D pattern falls into one:

- **p1:** translation only (no rotation, reflection, or glide)
- **p4m:** the richest — 4-fold rotation + mirrors
- **pg:** glide reflections only (subtle and beautiful)

This crate detects which symmetry group a pattern belongs to, generates patterns from groups, and connects symmetry to the vibe field — symmetry is what makes vibes *coherent*.

## Quick start

```rust
use lau_symmetry_engine::{WallpaperGroup, PatternDetector, PatternGenerator};

// Identify the symmetry group of a pattern
let pattern = vec![/* 2D grid of values */];
let detector = PatternDetector::new();
let group = detector.detect(&pattern);
println!("Symmetry: {:?}", group); // e.g., WallpaperGroup::P4m

// Generate a pattern from a symmetry group
let gen = PatternGenerator::from_group(WallpaperGroup::P6m);
let tiles = gen.generate(10, 10);

// All 17 groups
for group in WallpaperGroup::all() {
    println!("{}: order {}", group.name(), group.order());
}
```

## Contributing

[Open an issue](https://github.com/SuperInstance/lau-symmetry-engine/issues) or PR.
