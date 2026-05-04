# WebUI Roadmap

## 1. Objective
Close the gap from "GPU UI core" to a production-ready WebUI stack.

## 2. Current Status (May 2026)
- Done: window/event loop, wgpu rendering path, primitive drawing, basic input capture, minimal node tree, dirty tracking baseline, profiling baseline.
- Missing: web deployment path, mature text stack, event propagation model, advanced layout, accessibility, standard component layer, migration completion metrics.

## 3. Milestones

### M0: Stability and CI Baseline (1-2 weeks)
Scope:
- Keep CI green for fmt/test/gates.
- Remove warning regressions.
- Define ownership for crates and docs.
Exit Criteria:
- 7 consecutive days with no red main branch CI.
- Zero rustfmt failures in PR checks.

### M1: Input and Event Model (2-3 weeks)
Scope:
- Complete pointer semantics (move/down/up/scroll consistency).
- Add capture/bubble event propagation in compat-facing API.
- Normalize focus behavior and keyboard navigation baseline.
Exit Criteria:
- Event-order tests for capture -> target -> bubble.
- Focus traversal tests for Tab/Shift+Tab.

### M2: Text and Layout Foundation (3-5 weeks)
Scope:
- Introduce real text pipeline (font loading, shaping-ready interfaces).
- Implement baseline text metrics and wrapping.
- Expand layout toward Flex-like behavior for key screens.
Exit Criteria:
- Text rendering for mixed strings at stable positions.
- Reproducible layout results for predefined fixtures.

### M3: Component and Accessibility Layer (3-4 weeks)
Scope:
- Build core components: Button, TextInput, List, Panel, Dialog.
- Add accessibility metadata and keyboard-operable paths.
- Define component API stability policy.
Exit Criteria:
- Keyboard-only operation for core component demos.
- Accessibility metadata exported in app-facing structures.

### M4: Migration and Replacement Validation (2-4 weeks)
Scope:
- Finish API mapping coverage and MUST compatibility checks.
- Reproduce representative legacy screens.
- Quantify migration cost and performance delta.
Exit Criteria:
- API replacement ratio >= 80%.
- Screen reproduction ratio >= 90%.
- Performance target met per requirements summary.

## 4. Cross-Cutting Tracks
- Performance: keep avg frame <= 16.6ms and p95 <= 20ms on target scenes.
- Reliability: minimize panic usage and improve actionable error messages.
- Documentation: every milestone must update EN/JA docs and changelog notes.

## 5. Suggested PR Strategy
- Keep one milestone split into small PRs (review, refactor, docs).
- Require tests or validation note for each functional PR.
- Avoid coupling architecture changes with large behavior changes in one PR.
