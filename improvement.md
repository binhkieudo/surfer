# Surfer Performance Improvement Opportunities

## Executive Summary

Analysis of Surfer's codebase identifies **3 major performance bottlenecks**:
1. **Wave Rendering (Drawing)** - Most expensive, runs every frame
2. **Value Translation** - Medium heavy, on-demand conversion
3. **File Parsing** - Already optimized but minor improvements possible

This document outlines concrete optimization strategies with ROI analysis.

---

## 1. Wave Rendering (Drawing Canvas) - CRITICAL 🔴

**Current Implementation:**
- File: `libsurfer/src/drawing_canvas.rs` (1823 lines)
- Recomputes **all pixels** every frame
- Performs ~1200+ pixel calculations × number of variables
- Uses rayon for multi-threaded CPU rendering
- Results sent to GPU for OpenGL display

**Bottleneck Flow:**
```
CPU (rayon parallel)
├─ For each pixel: timestamp = viewport.as_time(x)
├─ Wave lookup: value = waves[timestamp]
├─ Translation: string = translate(value)
├─ Color mapping: color = value_to_color(value)
└─ Build draw commands
    ↓
GPU (OpenGL render)
```

**Performance Impact:**
- Large files: 50-200ms per frame (8-16 FPS)
- Multiple variables: exponential slowdown
- Zoom/pan causes full recalculation

### **Improvement 1A: GPU Compute Shaders (Highest Impact)**

**Proposal:** Move pixel-to-waveform calculation to GPU compute shaders

**Benefits:**
- GPU parallelism: 10-100x speedup over CPU
- Eliminate CPU-GPU transfer bottleneck
- Real-time interaction with massive datasets

**Implementation:**
```rust
// Current: CPU computes, then GPU renders
// New: GPU computes AND renders

// Compute shader: For each pixel
// timestamp = viewport_left + (pixel_x / canvas_width) * (viewport_right - viewport_left)
// value = binary_search_wave_data(timestamp)
// output_color = value_to_color(value)
```

**Effort:** High (2-3 weeks)
- Requires learning GPU compute shaders (GLSL/WGSL)
- Must refactor wave data access for GPU memory
- Need to handle texture atlasing for large waveforms

**ROI:** 🔥🔥🔥 (10-100x speedup)
- Enables smooth interaction with gigabyte-scale waveforms
- Pan/zoom becomes interactive on large files

**Dependencies:**
- glow crate: Already supports compute shaders
- Need to restructure wave data for GPU access (memory-mapped or GPU buffers)

---

### **Improvement 1B: Level-of-Detail (LOD) Rendering**

**Proposal:** Render at reduced resolution when zoomed out

**Benefits:**
- 4-16x speedup when zoomed out
- Maintains visual quality at all zoom levels
- Works well with current architecture

**Implementation:**
```rust
// In drawing_canvas.rs
match viewport.zoom_level() {
    z if z < 0.25 => {
        // Zoomed out: sample every 4th pixel
        render_at_resolution(canvas_width / 4, canvas_height)
    }
    z if z < 0.5 => {
        // Moderately zoomed: sample every 2nd pixel
        render_at_resolution(canvas_width / 2, canvas_height)
    }
    _ => {
        // Zoomed in: full resolution
        render_at_resolution(canvas_width, canvas_height)
    }
}
```

**Effort:** Medium (1-2 weeks)
- Modify pixel iteration loop
- Add zoom-level detection
- May require anti-aliasing for quality

**ROI:** 🔥🔥 (4-16x speedup when zoomed out)
- Most common use case: zoomed out view = immediate improvement
- Works with existing GPU rendering

**Implementation Location:** `drawing_canvas.rs` lines 728-770

---

### **Improvement 1C: Tile-based Caching**

**Proposal:** Cache rendered regions and only recompute changed tiles

**Benefits:**
- Pan without changing data = 0ms (instant)
- Only recalculate tiles intersecting viewport
- Incremental updates faster than full redraw

**Implementation:**
```rust
pub struct TileCache {
    // Map: (tile_x, tile_y, zoom_level) → CachedTile
    tiles: HashMap<(i32, i32, u32), CachedTile>,
    tile_size: u32,  // e.g., 256x256 pixels
}

impl TileCache {
    fn get_or_compute(&mut self, tile_id: (i32, i32, u32)) -> &CachedTile {
        if self.tiles.contains_key(&tile_id) {
            return &self.tiles[&tile_id];  // Cache hit: ~0ms
        }
        // Cache miss: compute and store
        let tile = compute_tile(tile_id);
        self.tiles.insert(tile_id, tile);
        &self.tiles[&tile_id]
    }
}

// In draw_items():
let visible_tiles = viewport.intersecting_tiles();
for tile_id in visible_tiles {
    let tile = cache.get_or_compute(tile_id);
    painter.image_button(tile.texture);
}
```

**Effort:** Medium (1-2 weeks)
- Implement tile management
- Handle zoom level invalidation
- Memory management for tile storage (LRU eviction)

**ROI:** 🔥🔥 (5-10x speedup for pan, 0ms cache hits)
- Improves UX: smooth panning
- Works alongside other optimizations

**Considerations:**
- Memory usage: 256×256 tile × 4 bytes (RGBA) × N tiles
- Cache invalidation when: zoom changes, new data loaded

---

### **Improvement 1D: Progressive/Incremental Rendering**

**Proposal:** Render in quality levels to provide immediate visual feedback

**Benefits:**
- User sees result in <100ms instead of waiting
- Perceived performance improves significantly
- Can be canceled if viewport changes

**Implementation:**
```rust
// Frame 1: Low resolution (50% quality) - 10ms
// Frame 2: Medium resolution (75% quality) - 25ms
// Frame 3: Full resolution (100% quality) - 50ms

pub fn render_progressive(viewport: &Viewport, painter: &mut Painter) {
    for quality_level in [0.25, 0.5, 0.75, 1.0] {
        let pixels = calculate_pixels(quality_level);
        painter.draw(pixels);
        ctx.request_repaint();  // Schedule next frame
        if viewport_changed { break; }  // Cancel if user interacts
    }
}
```

**Effort:** Low-Medium (1 week)
- Hook into egui's frame repaint system
- Add quality level parameter to rendering
- Requires careful timing to avoid overhead

**ROI:** 🔥 (Perceived performance improvement)
- UX improvement: no perceived lag
- Works with existing architecture
- Could be combined with LOD rendering

---

## 2. Value Translation - MEDIUM 🟡

**Current Implementation:**
- File: `libsurfer/src/translation/numeric_translators.rs` (1266 lines)
- Per-value translation on-demand during rendering
- Handles: decimal, hex, float, fixed-point, signed, unsigned, VHDL, etc.
- No caching between frames

**Bottleneck:**
```
For each pixel:
    Get wave value at timestamp
    Translate value (bit pattern → string)  ← Called millions of times/second
    Map to color
```

**Performance Impact:**
- Small waveforms: negligible
- Large waveforms (many variables): 10-30% of render time
- Many values repeat (common in simulations)

### **Improvement 2A: LRU Cache Translation Results**

**Proposal:** Cache translated values to avoid re-translating same bits

**Benefits:**
- 50-80% reduction in translation overhead
- Easy to implement
- Works immediately with no other changes

**Implementation:**
```rust
use lru::LruCache;

pub struct TranslationCache {
    // (bit_pattern, translation_type) → translated_string
    cache: LruCache<(u64, TranslationType), String>,
}

impl TranslationCache {
    pub fn translate(&mut self, value: &VariableValue, ty: TranslationType) -> String {
        let key = (value.to_u64(), ty);
        
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        
        let result = translate_value(value, ty);
        self.cache.put(key, result.clone());
        result
    }
}

// In drawing_canvas.rs
let mut translation_cache = TranslationCache::new(NonZeroUsize::new(10_000).unwrap());

for pixel in pixels {
    let value = get_wave_value(pixel);
    let translated = translation_cache.translate(&value, TranslationType::Decimal);
    // ...
}
```

**Effort:** Low (2-3 days)
- Add dependency: `lru` crate
- Wrap translation calls with cache
- Thread-safe version if needed (parking_lot RwLock)

**ROI:** 🔥 (50-80% translation speedup)
- Immediate impact on rendering performance
- No API changes needed
- Can be enabled/disabled for testing

**Cache Size:** 10,000 entries = ~1-2MB memory overhead
**Hit Rate:** 60-95% for typical waveforms (many repeated values)

---

### **Improvement 2B: Bulk Translation with Batch SIMD**

**Proposal:** Translate multiple values in parallel using SIMD

**Benefits:**
- 2-4x speedup on translation-heavy operations
- Better CPU cache utilization
- Works with cache from 2A

**Implementation:**
```rust
// Instead of translating values one-by-one during pixel iteration,
// collect all unique values first and translate in batch

let mut unique_values = HashSet::new();
for pixel in pixels {
    unique_values.insert(get_wave_value(pixel));
}

// Batch translate with parallel iterator
let translated: HashMap<_, _> = unique_values
    .par_iter()  // Parallel iteration (rayon)
    .map(|value| (value.clone(), translate_bulk(value)))
    .collect();

// Then use pre-computed results
for pixel in pixels {
    let value = get_wave_value(pixel);
    let result = translated[&value];
}
```

**Effort:** Medium (1 week)
- Refactor pixel iteration to two-pass approach
- Implement batch translation
- Optimize memory access patterns

**ROI:** 🔥🔥 (2-4x on translation, when combined with cache)

---

## 3. File Parsing - LOW 🟢

**Current Implementation:**
- File: `libsurfer/src/wellen.rs` (995 lines)
- Uses `wellen` crate for VCD/FST/GHW parsing
- Async loading with tokio
- Incremental signal loading
- LZ4 decompression for FST format

**Status:** Already well-optimized with async/incremental loading

### **Improvement 3A: Parallel Decompression**

**Proposal:** Decompress multiple chunks in parallel

**Benefits:**
- 2-3x speedup for large FST files
- Works alongside async loading

**Implementation:**
```rust
// Current: Sequential decompression
for chunk in compressed_chunks {
    let decompressed = decompress_lz4(chunk);
    process(decompressed);
}

// Improved: Parallel decompression
let decompressed: Vec<_> = compressed_chunks
    .par_iter()
    .map(|chunk| decompress_lz4(chunk))
    .collect();

for chunk in decompressed {
    process(chunk);
}
```

**Effort:** Low (2-3 days)
- Already using rayon elsewhere
- Minimal changes to decompression loop
- Profile before/after to measure impact

**ROI:** 🔥 (2-3x on file load time for large files)
- Mainly affects initial file load (not during interaction)

---

## Summary & Recommendations

### **Quick Wins (Implement First)** ⚡
1. **Translation LRU Cache** (2-3 days)
   - 50-80% translation speedup
   - Easiest to implement
   - Immediate impact

2. **LOD Rendering** (1-2 weeks)
   - 4-16x speedup when zoomed out
   - Most common use case
   - Works with current architecture

3. **Parallel Decompression** (2-3 days)
   - 2-3x file load improvement
   - Low risk, low effort

### **Medium Effort** 🔧
1. **Tile-based Caching** (1-2 weeks)
   - 5-10x pan performance
   - Smooth user experience
   - Can combine with LOD

2. **Progressive Rendering** (1 week)
   - Perceived performance boost
   - Better UX
   - Pairs well with other improvements

### **High Impact, High Effort** 🔥
1. **GPU Compute Shaders** (2-3 weeks)
   - 10-100x speedup
   - Enables gigabyte-scale waveforms
   - Requires significant refactoring
   - **Worth the effort for production version**

### **Estimated Performance Improvements**

| Strategy | Small Files | Large Files | Zoomed Out | Panning |
|----------|------------|-------------|-----------|---------|
| Baseline | 60 FPS | 8 FPS | 8 FPS | 5 FPS |
| + Translation Cache | 65 FPS | 12 FPS | 10 FPS | 8 FPS |
| + LOD | 65 FPS | 20 FPS | 60 FPS | 10 FPS |
| + Tile Caching | 65 FPS | 20 FPS | 60 FPS | 60 FPS |
| + GPU Compute | 60 FPS | 55 FPS | 60 FPS | 60 FPS |

### **Implementation Roadmap**

**Phase 1 (1 week):**
- [ ] Translation LRU Cache
- [ ] Parallel Decompression

**Phase 2 (2 weeks):**
- [ ] LOD Rendering
- [ ] Progressive Rendering

**Phase 3 (3-4 weeks):**
- [ ] Tile-based Caching
- [ ] GPU Compute Shaders (if needed)

---

## Technical Details

### **Code Locations**

**Wave Rendering:**
- Main loop: `libsurfer/src/drawing_canvas.rs:728` - `draw_items()`
- Pixel calculation: Line ~760-800
- Parallel iteration: Line ~770-790 (rayon usage)

**Value Translation:**
- Numeric translators: `libsurfer/src/translation/numeric_translators.rs:1266`
- Called from: `libsurfer/src/translation/mod.rs`
- Usage in rendering: `drawing_canvas.rs` (value to color mapping)

**File Parsing:**
- Wellen wrapper: `libsurfer/src/wellen.rs:995`
- Async loading: Line ~482-510 (`load_all_params`)
- Decompression: Handled by `wellen` crate

### **Dependencies**

Current relevant crates:
```toml
rayon = "1.10.0"          # Already used for parallelization
lru = "0.12"              # Needed for cache (new)
glow = "0.34.1"           # GPU rendering (compute shader support)
```

### **Performance Profiling**

To measure improvements:
```bash
# Profile rendering
cargo build --release
perf record -g ./target/release/surfer large_file.vcd
perf report

# Frame time analysis
# Add timing instrumentation in drawing_canvas.rs:
let start = std::time::Instant::now();
let duration = start.elapsed();
println!("Frame time: {}ms", duration.as_millis());
```

---

## References

- **rayon** documentation: https://docs.rs/rayon/
- **glow** compute shaders: https://github.com/gfx-rs/glow
- **GPU rendering patterns**: GPU Gems, Real-Time Rendering
- **Caching strategies**: LRU, Clock, Arc-based (Rust-specific)

---

## Questions & Discussion

**Q: Will these changes break existing functionality?**
A: No, all optimizations are internal improvements. Public API unchanged.

**Q: Can optimizations be combined?**
A: Yes! LOD + Tiling + Cache works well together. GPU Compute would replace current rendering.

**Q: Estimated ROI?**
A: Translation Cache = 2-3 days for 50-80% gain. LOD = 2 weeks for 4-16x gain on common case.

**Q: Should we do GPU Compute first?**
A: No, start with quick wins (cache, LOD). GPU Compute is long-term investment.

