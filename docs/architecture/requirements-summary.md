# Requirements Summary

## 1. Goal
Build a Rust-based GPU WebUI engine that can replace existing engines in production.

## 2. MVP Scope
- Window and GPU initialization
- Basic primitives (rectangles/lines)
- Resize handling
- Mouse/keyboard input
- Minimal UI tree updates
- Reproduce at least one legacy screen via Compat

## 3. Performance Targets
- Avg frame time <= 16.6ms
- P95 frame time <= 20ms
- Draw calls <= 200 on major scenes
- Skip redraw on unchanged frames

## 4. Compatibility Targets
- Support major Node/Style/Event concepts
- Maintain legacy-to-new API mapping
- Enable staged migration with old/new coexistence

## 5. Quality Targets
- Compat/FastPath equivalence tests are mandatory
- MUST APIs are strictly managed
- P0/P1 CI gates must pass in PRs

## 6. Replacement Criteria
- API replacement ratio: >= 80%
- Screen reproduction ratio: >= 90%
- Speed: better than legacy, or equal FPS with lower CPU/GPU usage

## 7. References
- Detailed requirements: requirements.md
- Replacement strategy: replacement-strategy.md
- API mapping: api-mapping.md
- CI gate operations: ci-gates.md
