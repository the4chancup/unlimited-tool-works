# Unlimited Tool Works — Core plan: Parallelism

Part of the [Core plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Parallelism

### Design

**One model: `rayon` for parallel work, `crossbeam-channel` for hand-off, plain threads for
background tasks (update check, folder watching). No async runtime anywhere in the workspace — and
"anywhere" includes dependencies: a crate that starts its own Tokio/smol runtime internally (as
`reqwest` does even in blocking mode) is not approved, because the rule is about what runs in the
process, not about which module wrote `async`. The reasons: every I/O-bound thing the suite does
fits a thread; an async runtime would be a second concurrency model for agents to mix with the
first; and the `fmdl`/`pes_model` PyO3 constraint (guardrail 4) already forbids runtimes in the
libs those crates sit next to.**

Replace Blue's `ThreadPoolExecutor` + `console_lock` with `rayon` work-stealing:

```rust
// The Team compiler resolves exports into a run-level BuildManifest
// (serial planning: IDs, dependencies, output paths, collision decisions),
// then processes each manifest task independently. The object model lives in
// libs/aesthetics_export; compile behavior is in tool-local free functions.
let refs_preflight = reader_duplicate_refs_preflight(&discovered_exports); // before validation
let mut plan = plan_run(resolved_exports, &compile_ctx); // only identity-resolved ResolvedAestheticsExport values
plan.merge_discovery_preflight(refs_preflight); // messages + dropped ExportIds
emit_messages(&plan.messages);
let Some(manifest) = plan.manifest else {
    return; // AbortRun/global fatal; scoped drops still return a manifest
};
manifest.tasks.into_par_iter().for_each(|task| {
    let batch: TaskBatch = process_task(task, &compile_ctx); // producer ID, entries, messages, permit
    writer.send(batch);
});
```

This is the design in outline; the code's shapes are `spec/team_compiler/pipeline.md` "Run
driver shapes". There the duplicate-refs preflight is `reader::route`, before validation, plus an
export-scoped `multiple_ref_exports` on each conflicting refs export; `plan_run` accepts only
identity-resolved `ResolvedAestheticsExport` values and returns a `PlanReport` of the manifest
and planning's messages, a dropped export being its `DropExport` message, which nothing looks up
by ID; planning has no run-level fatal, so every run that reaches it gets a manifest; and the
coordinator admits tasks into the pool in manifest order instead of `into_par_iter`. No processing-time locks are needed for output
allocation — manifest planning establishes task independence before parallel work starts. The borrow
checker enforces safe memory access, not disjoint output paths; manifest collision checks and tests
establish the latter. No GIL, no `console_lock`, no free-threaded Python workarounds.

### Thread count

```rust
fn thread_count_detect(requested: usize) -> usize {
    if requested > 0 { return requested; }
    let logical = available_parallelism().map(|count| count.get()).unwrap_or(1);
    // Reserve one core for the reader and writer; minimum 1 worker
    logical.saturating_sub(1).max(1)
}
```

### Memory budget

Blue's `MemoryBudget` with backpressure exists because exports are large — approximately 200MB on
average, meaning a full 48-team compilation has ~9.6GB of raw export data in flight. With 7 parallel
workers, that's 1.4GB minimum just for in-flight exports, before counting the writer's CPK buffers,
texture conversion temporaries, and the OS. Users with 8GB RAM (common in the modding community)
would hit memory pressure without backpressure.

**Rust does not eliminate the need for a budget.** Ownership gives deterministic lifetimes, but
allocator behavior, parsed representations, and conversion buffers still add overhead. There is no
fixed Rust-versus-Python RSS ratio to rely on; measure the actual pipeline on representative inputs.

**The budget works at loaded-`BuildTask`-scope granularity**, improving on Blue's per-export
accounting: an export's structure (folder tree, names, sizes) is loaded eagerly but is tiny; each
model, kit, logo, Common, collar, portrait, or referee-marker task loads content lazily and owns its
lease and derived Arc clones until the writer drains its `TaskBatch`. Memory therefore drains
continuously during a run rather than in whole-export steps. The bounded exception is a
Team-compiler `PlayerBatchGroup`: face/boots/gloves child processing remains independent, but
permit-charged bytes shared by those children stay charged until every child reports and the group's
one shared-texture batch commits. Solid `.7z` archives are the other exception (one-pass
decompression forces whole-export residency); plain folders and `.zip` support lazy per-folder
loading. Details are in the Team compiler plan's walkthrough.

A task's charge includes source bytes, decoded textures, converted/merged models, and packed
entries; shared bytes and the bounded conversion cache stay charged while retained. Source size
alone does not bound peak memory: texture decoding and conversion can expand it substantially.
The goal is to bound live work independently of the number of teams, not to promise a fixed 2×
source-size multiplier. Complete accounting is an implementation gate (see the Team compiler plan).

Admission must remain progress-safe when a task already owns source/archive bytes and needs
conversion workspace, and when canonical writer order delays later batches. Do not let all workers
wait for extra capacity that only their own completion could release. Cancellation must also wake
budget waiters; the semaphore sketch below shows capacity accounting, not the full cancellation or
multi-allocation admission protocol.

**Keep the memory budget.** The implementation is a straightforward semaphore:

```rust
pub struct MemoryBudget {
    cap: usize,
    // The condition and the value it guards use the same mutex. Updating an
    // atomic outside this mutex could notify between a check and wait, losing
    // the wake-up and leaving an acquirer asleep indefinitely.
    in_flight: Mutex<usize>,
    notify: Condvar,
}

impl MemoryBudget {
    pub fn acquire(&self, size: usize) {
        let mut in_flight = self.in_flight.lock().unwrap();
        if size > self.cap {
            // Oversized request: wait for the pipeline to drain, then
            // proceed alone — waiting for ordinary capacity would deadlock.
            while *in_flight > 0 {
                in_flight = self.notify.wait(in_flight).unwrap();
            }
        } else {
            while (*in_flight).checked_add(size).map_or(true, |total| total > self.cap) {
                in_flight = self.notify.wait(in_flight).unwrap();
            }
        }
        *in_flight += size;
    }

    pub fn release(&self, size: usize) {
        let mut in_flight = self.in_flight.lock().unwrap();
        assert!(*in_flight >= size, "memory-budget permit released twice");
        *in_flight -= size;
        self.notify.notify_all();
    }
}
```

The crate's contract, with the permit that releases on drop, cancellation and the priority an
oversized request needs to run at all, is in [libs/pipeline](../libs/pipeline.md).

---
