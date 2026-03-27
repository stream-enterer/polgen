# Marker File Correspondence Audit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Document all 20 marker files in `src/emCore/` with factual evidence about what the C++ type does and where equivalent functionality lives in Rust (or doesn't), surfacing open questions for human review.

**Architecture:** 6 parallel research agents (each in a worktree), each analyzing a group of related marker files by reading C++ source, tracing usage, and searching the Rust codebase. A 7th synthesis agent consolidates findings. Agents produce evidence only — no classifications or recommendations.

**Tech Stack:** Git worktrees, grep, git log, file reading. No code compilation needed.

---

## Pre-flight

### Task 0: Create working directory

**Files:**
- Create: `target/marker-audit/` (gitignored directory for intermediates)

- [ ] **Step 1: Create the intermediates directory**

Run: `mkdir -p target/marker-audit`

- [ ] **Step 2: Verify C++ source is accessible**

Run: `ls ~/git/eaglemode-0.96.4/include/emCore/emArray.h`
Expected: file exists

---

## Research Agents (Tasks 1–6 run in parallel)

Each task below is dispatched as a single subagent. The agent prompt is the complete instruction set — copy it verbatim. Every agent runs in a worktree for isolation.

**Critical instruction for ALL agents:** You produce factual evidence only. You do NOT classify files (no labels like "stdlib-replacement" or "necessary-rust-specific"). You do NOT recommend actions (no "should be deleted" or "should be ported"). When uncertain, write `OPEN QUESTION:` instead of guessing.

---

### Task 1: Agent — stdlib-containers (emArray, emList, emString)

**Files:**
- Modify: `src/emCore/emArray.no_rust_equivalent`
- Modify: `src/emCore/emList.no_rust_equivalent`
- Modify: `src/emCore/emString.no_rust_equivalent`
- Create: `target/marker-audit/stdlib-containers.json`

- [ ] **Step 1: Dispatch agent with the following prompt**

```
You are a research agent performing evidence gathering on marker files in an eaglemode-rs
codebase. You produce FACTUAL EVIDENCE ONLY. No classifications, no recommendations, no
action items. When uncertain, write "OPEN QUESTION:" instead of guessing.

Your assignment: analyze 3 marker files — emArray, emList, emString.

For EACH of these 3 files, perform ALL of the following steps in order:

## Step A: Read the C++ header

Read the file ~/git/eaglemode-0.96.4/include/emCore/{name}.h

Record:
- Every public class, struct, and typedef declared
- All public method signatures with parameter types and return types
- What the type fundamentally does

Also check if ~/git/eaglemode-0.96.4/src/emCore/{name}.cpp exists. If so, read it and
note any significant implementation details not visible from the header.

C++ implementation files that exist for this group:
- emList.cpp exists
- emString.cpp exists
- emArray.cpp does NOT exist (header-only)

## Step B: Trace C++ usage

Use the Grep tool to search for uses of the C++ type in:
- ~/git/eaglemode-0.96.4/include/emCore/ (all .h files)
- ~/git/eaglemode-0.96.4/src/emCore/ (all .cpp files)

Search patterns: the class name (e.g. "emArray"), key method names.

Record for each usage:
- Which file and line number
- How the type is used (member variable, parameter, local variable, return type)
- Common method calls on the type

Limit to the first 30 usages if there are many — note the total count.

## Step C: Search the Rust codebase

For each C++ type and its key methods, search the Rust codebase at src/emCore/ for
equivalent functionality:
- Search for Rust stdlib types that serve the same purpose (Vec, VecDeque, String, &str, etc.)
- Search for any custom Rust types that might replace the C++ type
- Search for the C++ method names in case they were preserved
- Record specific file paths and line numbers

## Step D: Write findings into the marker file

Update each marker file (e.g. src/emCore/emArray.no_rust_equivalent) with this format:

```
C++ header: include/emCore/emFoo.h
C++ implementation: src/emCore/emFoo.cpp (or "header-only")

C++ public API:
  - class emFoo<T> (template parameters if any)
  - emFoo() — default constructor
  - MethodA(ParamType param) -> ReturnType — brief description
  - MethodB(ParamType param) -> ReturnType — brief description
  [... all public methods with full signatures]

C++ usage in emCore (N total usages found):
  - Used in emBaz.h (line N): as member variable of type emFoo<int>
  - Used in emQux.cpp (line N): constructed in method X
  [... representative usages with file:line]

Rust equivalents found:
  - emFoo broadly maps to std::SomeType
  - MethodA: see src/emCore/emBar.rs:42 — Rust uses equivalent_method()
  - MethodB: no equivalent found in Rust codebase
  [... per-method mapping with file:line or "not found"]

Coverage gaps:
  - MethodB has no Rust equivalent. In C++ it is called from [locations].
  [... only list gaps where C++ functionality has no Rust counterpart found]

OPEN QUESTIONS:
  - [any ambiguities the agent noticed but could not resolve]
```

IMPORTANT: Write the ACTUAL content into each marker file. Do not write placeholder
templates. Every method signature, file path, and line number must be real, verified data
from your research.

## Step E: Write JSON summary

After all 3 files are documented, write a JSON file to target/marker-audit/stdlib-containers.json:

{
  "agent": "stdlib-containers",
  "files_documented": ["emArray", "emList", "emString"],
  "open_question_count": <number>,
  "coverage_gap_count": <number>,
  "summary": "<one paragraph of factual observations across all 3 files>"
}
```

- [ ] **Step 2: Verify agent output**

Check that all 3 marker files are non-empty and follow the format:
Run: `wc -l src/emCore/emArray.no_rust_equivalent src/emCore/emList.no_rust_equivalent src/emCore/emString.no_rust_equivalent`
Expected: each file has 20+ lines

Run: `cat target/marker-audit/stdlib-containers.json`
Expected: valid JSON with all fields populated

---

### Task 2: Agent — ownership-memory (emRef, emOwnPtr, emOwnPtrArray, emCrossPtr, emAnything)

**Files:**
- Modify: `src/emCore/emRef.no_rust_equivalent`
- Modify: `src/emCore/emOwnPtr.no_rust_equivalent`
- Modify: `src/emCore/emOwnPtrArray.no_rust_equivalent`
- Modify: `src/emCore/emCrossPtr.no_rust_equivalent`
- Modify: `src/emCore/emAnything.no_rust_equivalent`
- Create: `target/marker-audit/ownership-memory.json`

- [ ] **Step 1: Dispatch agent with the following prompt**

```
You are a research agent performing evidence gathering on marker files in an eaglemode-rs
codebase. You produce FACTUAL EVIDENCE ONLY. No classifications, no recommendations, no
action items. When uncertain, write "OPEN QUESTION:" instead of guessing.

Your assignment: analyze 5 marker files — emRef, emOwnPtr, emOwnPtrArray, emCrossPtr, emAnything.

For EACH of these 5 files, perform ALL of the following steps in order:

## Step A: Read the C++ header

Read the file ~/git/eaglemode-0.96.4/include/emCore/{name}.h

Record:
- Every public class, struct, and typedef declared
- All public method signatures with parameter types and return types
- What the type fundamentally does

Also check if ~/git/eaglemode-0.96.4/src/emCore/{name}.cpp exists. If so, read it and
note any significant implementation details not visible from the header.

C++ implementation files that exist for this group:
- emRef.cpp exists
- emAnything.cpp exists
- emCrossPtr.cpp exists
- emOwnPtr.cpp does NOT exist (header-only)
- emOwnPtrArray.cpp does NOT exist (header-only)

## Step B: Trace C++ usage

Use the Grep tool to search for uses of the C++ type in:
- ~/git/eaglemode-0.96.4/include/emCore/ (all .h files)
- ~/git/eaglemode-0.96.4/src/emCore/ (all .cpp files)

Search patterns: the class name (e.g. "emRef"), key method names.

Record for each usage:
- Which file and line number
- How the type is used (member variable, parameter, local variable, return type)
- Common method calls on the type

Limit to the first 30 usages if there are many — note the total count.

## Step C: Search the Rust codebase

For each C++ type and its key methods, search the Rust codebase at src/emCore/ for
equivalent functionality:
- Search for Rust ownership types (Rc, Arc, Box, Weak, RefCell, etc.)
- Search for Any/dyn Any usage
- Search for the C++ method names in case they were preserved
- Record specific file paths and line numbers

## Step D: Write findings into the marker file

Update each marker file with the standard format:

C++ header: include/emCore/emFoo.h
C++ implementation: src/emCore/emFoo.cpp (or "header-only")

C++ public API:
  - class emFoo<T>
  - All public methods with full signatures
  [...]

C++ usage in emCore (N total usages found):
  - Representative usages with file:line
  [...]

Rust equivalents found:
  - Per-method mapping with file:line or "not found"
  [...]

Coverage gaps:
  - C++ functionality with no Rust counterpart found
  [...]

OPEN QUESTIONS:
  - Ambiguities

IMPORTANT: Write ACTUAL content. Every method signature, file path, and line number must
be real, verified data from your research.

## Step E: Write JSON summary

Write to target/marker-audit/ownership-memory.json:

{
  "agent": "ownership-memory",
  "files_documented": ["emRef", "emOwnPtr", "emOwnPtrArray", "emCrossPtr", "emAnything"],
  "open_question_count": <number>,
  "coverage_gap_count": <number>,
  "summary": "<one paragraph of factual observations across all 5 files>"
}
```

- [ ] **Step 2: Verify agent output**

Check that all 5 marker files are non-empty:
Run: `wc -l src/emCore/emRef.no_rust_equivalent src/emCore/emOwnPtr.no_rust_equivalent src/emCore/emOwnPtrArray.no_rust_equivalent src/emCore/emCrossPtr.no_rust_equivalent src/emCore/emAnything.no_rust_equivalent`
Expected: each file has 20+ lines

Run: `cat target/marker-audit/ownership-memory.json`
Expected: valid JSON with all fields populated

---

### Task 3: Agent — system-primitives (emThread, emFileStream, emTmpFile)

**Files:**
- Modify: `src/emCore/emThread.no_rust_equivalent`
- Modify: `src/emCore/emFileStream.no_rust_equivalent`
- Modify: `src/emCore/emTmpFile.no_rust_equivalent`
- Create: `target/marker-audit/system-primitives.json`

- [ ] **Step 1: Dispatch agent with the following prompt**

```
You are a research agent performing evidence gathering on marker files in an eaglemode-rs
codebase. You produce FACTUAL EVIDENCE ONLY. No classifications, no recommendations, no
action items. When uncertain, write "OPEN QUESTION:" instead of guessing.

Your assignment: analyze 3 marker files — emThread, emFileStream, emTmpFile.

For EACH of these 3 files, perform ALL of the following steps in order:

## Step A: Read the C++ header

Read the file ~/git/eaglemode-0.96.4/include/emCore/{name}.h

Record:
- Every public class, struct, and typedef declared
- All public method signatures with parameter types and return types
- What the type fundamentally does

Also check if ~/git/eaglemode-0.96.4/src/emCore/{name}.cpp exists. If so, read it and
note any significant implementation details not visible from the header.

C++ implementation files that exist for this group:
- emThread.cpp exists
- emFileStream.cpp exists
- emTmpFile.cpp exists

## Step B: Trace C++ usage

Use the Grep tool to search for uses of the C++ type in:
- ~/git/eaglemode-0.96.4/include/emCore/ (all .h files)
- ~/git/eaglemode-0.96.4/src/emCore/ (all .cpp files)

Search patterns: the class name (e.g. "emThread"), key method names.

Record for each usage:
- Which file and line number
- How the type is used
- Common method calls on the type

Limit to the first 30 usages if there are many — note the total count.

## Step C: Search the Rust codebase

For each C++ type and its key methods, search the Rust codebase at src/emCore/ for
equivalent functionality:
- Search for std::thread, std::fs, tempfile crate usage
- Search for any custom threading/file/temp abstractions
- Search for the C++ method names in case they were preserved
- Record specific file paths and line numbers

## Step D: Write findings into the marker file

Update each marker file with the standard format:

C++ header: include/emCore/emFoo.h
C++ implementation: src/emCore/emFoo.cpp (or "header-only")

C++ public API:
  - All public types and methods with full signatures
  [...]

C++ usage in emCore (N total usages found):
  - Representative usages with file:line
  [...]

Rust equivalents found:
  - Per-method mapping with file:line or "not found"
  [...]

Coverage gaps:
  - C++ functionality with no Rust counterpart found
  [...]

OPEN QUESTIONS:
  - Ambiguities

IMPORTANT: Write ACTUAL content. Every method signature, file path, and line number must
be real, verified data from your research.

## Step E: Write JSON summary

Write to target/marker-audit/system-primitives.json:

{
  "agent": "system-primitives",
  "files_documented": ["emThread", "emFileStream", "emTmpFile"],
  "open_question_count": <number>,
  "coverage_gap_count": <number>,
  "summary": "<one paragraph of factual observations across all 3 files>"
}
```

- [ ] **Step 2: Verify agent output**

Check that all 3 marker files are non-empty:
Run: `wc -l src/emCore/emThread.no_rust_equivalent src/emCore/emFileStream.no_rust_equivalent src/emCore/emTmpFile.no_rust_equivalent`
Expected: each file has 20+ lines

Run: `cat target/marker-audit/system-primitives.json`
Expected: valid JSON with all fields populated

---

### Task 4: Agent — framework-glue (emAvlTree, emAvlTreeMap, emAvlTreeSet, emToolkit)

**Files:**
- Modify: `src/emCore/emAvlTree.no_rust_equivalent`
- Modify: `src/emCore/emAvlTreeMap.no_rust_equivalent`
- Modify: `src/emCore/emAvlTreeSet.no_rust_equivalent`
- Modify: `src/emCore/emToolkit.no_rust_equivalent`
- Create: `target/marker-audit/framework-glue.json`

- [ ] **Step 1: Dispatch agent with the following prompt**

```
You are a research agent performing evidence gathering on marker files in an eaglemode-rs
codebase. You produce FACTUAL EVIDENCE ONLY. No classifications, no recommendations, no
action items. When uncertain, write "OPEN QUESTION:" instead of guessing.

Your assignment: analyze 4 marker files — emAvlTree, emAvlTreeMap, emAvlTreeSet, emToolkit.

For EACH of these 4 files, perform ALL of the following steps in order:

## Step A: Read the C++ header

Read the file ~/git/eaglemode-0.96.4/include/emCore/{name}.h

Record:
- Every public class, struct, and typedef declared
- All public method signatures with parameter types and return types
- What the type fundamentally does

Also check if ~/git/eaglemode-0.96.4/src/emCore/{name}.cpp exists. If so, read it and
note any significant implementation details not visible from the header.

C++ implementation files that exist for this group:
- emAvlTree.cpp exists
- emAvlTreeMap.cpp does NOT exist (header-only)
- emAvlTreeSet.cpp does NOT exist (header-only)
- emToolkit.cpp does NOT exist (header-only, but this needs careful reading)

## Step B: Trace C++ usage

Use the Grep tool to search for uses of the C++ type in:
- ~/git/eaglemode-0.96.4/include/emCore/ (all .h files)
- ~/git/eaglemode-0.96.4/src/emCore/ (all .cpp files)

Search patterns: the class name (e.g. "emAvlTree", "emToolkit"), key method names.

For emToolkit in particular: this is a key framework type. Be thorough — trace what
classes it declares, what methods it provides, and how it's used throughout the codebase.
Search for all direct references.

Record for each usage:
- Which file and line number
- How the type is used
- Common method calls on the type

Limit to the first 30 usages per type if there are many — note the total count.

## Step C: Search the Rust codebase

For each C++ type and its key methods, search the Rust codebase at src/emCore/ for
equivalent functionality:
- For AVL trees: search for BTreeMap, BTreeSet, HashMap, HashSet, or any custom tree types
- For emToolkit: search broadly — this type's functionality may be scattered across
  multiple Rust files. Search for toolkit, initialization, resource management patterns.
  Also check src/emCore/toolkit_images.rs and src/emCore/emGUIFramework.rs.
- Search for the C++ method names in case they were preserved
- Record specific file paths and line numbers

## Step D: Write findings into the marker file

Update each marker file with the standard format:

C++ header: include/emCore/emFoo.h
C++ implementation: src/emCore/emFoo.cpp (or "header-only")

C++ public API:
  - All public types and methods with full signatures
  [...]

C++ usage in emCore (N total usages found):
  - Representative usages with file:line
  [...]

Rust equivalents found:
  - Per-method mapping with file:line or "not found"
  [...]

Coverage gaps:
  - C++ functionality with no Rust counterpart found
  [...]

OPEN QUESTIONS:
  - Ambiguities

IMPORTANT: Write ACTUAL content. Every method signature, file path, and line number must
be real, verified data from your research.

## Step E: Write JSON summary

Write to target/marker-audit/framework-glue.json:

{
  "agent": "framework-glue",
  "files_documented": ["emAvlTree", "emAvlTreeMap", "emAvlTreeSet", "emToolkit"],
  "open_question_count": <number>,
  "coverage_gap_count": <number>,
  "summary": "<one paragraph of factual observations across all 4 files>"
}
```

- [ ] **Step 2: Verify agent output**

Check that all 4 marker files are non-empty:
Run: `wc -l src/emCore/emAvlTree.no_rust_equivalent src/emCore/emAvlTreeMap.no_rust_equivalent src/emCore/emAvlTreeSet.no_rust_equivalent src/emCore/emToolkit.no_rust_equivalent`
Expected: each file has 20+ lines

Run: `cat target/marker-audit/framework-glue.json`
Expected: valid JSON with all fields populated

---

### Task 5: Agent — rust-only-infra (fixed, rect, widget_utils, emPainterDrawList)

**Files:**
- Modify: `src/emCore/fixed.rust_only`
- Modify: `src/emCore/rect.rust_only`
- Modify: `src/emCore/widget_utils.rust_only`
- Modify: `src/emCore/emPainterDrawList.rust_only`
- Create: `target/marker-audit/rust-only-infra.json`

- [ ] **Step 1: Dispatch agent with the following prompt**

```
You are a research agent performing evidence gathering on marker files in an eaglemode-rs
codebase. You produce FACTUAL EVIDENCE ONLY. No classifications, no recommendations, no
action items. When uncertain, write "OPEN QUESTION:" instead of guessing.

Your assignment: analyze 4 rust_only marker files — fixed, rect, widget_utils, emPainterDrawList.

These are Rust files that have NO corresponding C++ header. Your job is to document what
they contain, why they exist (based on git history and code evidence), and how they relate
to the C++ codebase.

For EACH of these 4 files, perform ALL of the following steps in order:

## Step A: Read the current Rust file

Read src/emCore/{name}.rs. Record:
- Every public type (struct, enum, trait) defined
- Every public function defined
- Key constants or type aliases
- Module-level documentation if any

## Step B: Check git history

Run these commands and record the results:

1. Find creation commit:
   git log --follow --diff-filter=A --format="%H %ai %s" -- src/emCore/{name}.rs

2. Find major changes (look for commits that significantly changed scope/purpose):
   git log --follow --format="%H %ai %s" --stat -- src/emCore/{name}.rs

Record:
- Creation commit hash, date, and full commit message
- Any commits that significantly changed the file's scope or purpose (large diffs,
  renames, content moving in/out)
- For each major commit, note what actually changed (added types? removed types? refactored?)

## Step C: Trace C++ relationship

Based on what the Rust file contains, search the C++ codebase for related code:

1. Identify concepts in the Rust file (e.g., fixed-point arithmetic, rectangle types,
   widget helper functions, draw list structures)
2. Grep ~/git/eaglemode-0.96.4/include/emCore/ and ~/git/eaglemode-0.96.4/src/emCore/
   for those concepts
3. Record where equivalent C++ code lives — specific files, line ranges, function names
4. Note structural differences (e.g., "C++ has this inline in emPainter.h lines 100-150,
   Rust extracts it into a separate file")

## Step D: Search for Rust dependents

Grep the Rust codebase (src/emCore/) for imports and uses of types from this file.
Record:
- Which files import from this module
- Which specific types/functions they use
- How central this file is to the dependency graph

## Step E: Write findings into the marker file

Update the marker file (e.g. src/emCore/fixed.rust_only) with this format:

Rust file: src/emCore/foo.rs

Defines:
  - struct Foo — description
  - fn bar() -> ReturnType — description
  [... all public items]

Used by:
  - src/emCore/emBaz.rs:42 — uses Foo as field type
  - src/emCore/emQux.rs:99 — calls bar() in method X
  [... all dependents with file:line]

Git history:
  - Created: [hash] [date] — "[full commit message]"
  - Major change: [hash] [date] — "[commit message]" — [what specifically changed]

Related C++ code:
  - The functionality in this file corresponds to code in:
    - include/emCore/emBar.h lines N-M (description of what's there)
    - src/emCore/emBaz.cpp lines N-M (description of what's there)
  - Structural differences: [factual description]

OPEN QUESTIONS:
  - [any ambiguities about why this exists as a separate file]

IMPORTANT: Write ACTUAL content. Every type name, file path, line number, and commit hash
must be real, verified data from your research.

## Step F: Write JSON summary

Write to target/marker-audit/rust-only-infra.json:

{
  "agent": "rust-only-infra",
  "files_documented": ["fixed", "rect", "widget_utils", "emPainterDrawList"],
  "open_question_count": <number>,
  "coverage_gap_count": <number>,
  "summary": "<one paragraph of factual observations across all 4 files>"
}
```

- [ ] **Step 2: Verify agent output**

Check that all 4 marker files are non-empty:
Run: `wc -l src/emCore/fixed.rust_only src/emCore/rect.rust_only src/emCore/widget_utils.rust_only src/emCore/emPainterDrawList.rust_only`
Expected: each file has 20+ lines

Run: `cat target/marker-audit/rust-only-infra.json`
Expected: valid JSON with all fields populated

---

### Task 6: Agent — rust-only-toolkit (toolkit_images)

**Files:**
- Modify: `src/emCore/toolkit_images.rust_only`
- Create: `target/marker-audit/rust-only-toolkit.json`

- [ ] **Step 1: Dispatch agent with the following prompt**

```
You are a research agent performing evidence gathering on marker files in an eaglemode-rs
codebase. You produce FACTUAL EVIDENCE ONLY. No classifications, no recommendations, no
action items. When uncertain, write "OPEN QUESTION:" instead of guessing.

Your assignment: analyze 1 rust_only marker file — toolkit_images.

This is a Rust file that has NO corresponding C++ header. Your job is to document what
it contains, why it exists (based on git history and code evidence), and how it relates
to the C++ codebase.

## Step A: Read the current Rust file

Read src/emCore/toolkit_images.rs. Record:
- Every public type (struct, enum, trait) defined
- Every public function defined
- Key constants or type aliases
- Module-level documentation if any

## Step B: Check git history

Run these commands and record the results:

1. Find creation commit:
   git log --follow --diff-filter=A --format="%H %ai %s" -- src/emCore/toolkit_images.rs

2. Find major changes:
   git log --follow --format="%H %ai %s" --stat -- src/emCore/toolkit_images.rs

Record:
- Creation commit hash, date, and full commit message
- Any commits that significantly changed the file's scope or purpose
- For each major commit, note what actually changed

## Step C: Trace C++ relationship

This file likely relates to emToolkit in C++. Investigate:

1. Read ~/git/eaglemode-0.96.4/include/emCore/emToolkit.h — look for image/resource
   loading, bitmap data, or embedded image constants
2. Search C++ source for toolkit image resources, embedded bitmaps, or image initialization
3. Search for where toolkit images are loaded/used in the C++ codebase
4. Record specific files, line ranges, and function names

Also check:
- ~/git/eaglemode-0.96.4/src/emCore/ for any image data files or resource loading
- Whether C++ embeds images differently (resource files, compiled-in data, etc.)

## Step D: Search for Rust dependents

Grep the Rust codebase (src/emCore/) for imports and uses of types from toolkit_images.
Record:
- Which files import from this module
- Which specific types/functions they use

## Step E: Write findings into the marker file

Update src/emCore/toolkit_images.rust_only with:

Rust file: src/emCore/toolkit_images.rs

Defines:
  - [all public items with types and descriptions]

Used by:
  - [all dependents with file:line]

Git history:
  - Created: [hash] [date] — "[full commit message]"
  - Major change: [hash] [date] — "[commit message]" — [what changed]

Related C++ code:
  - [where equivalent functionality lives in C++, with file:line]
  - Structural differences: [factual description]

OPEN QUESTIONS:
  - [ambiguities]

IMPORTANT: Write ACTUAL content. Every data point must be real, verified data.

## Step F: Write JSON summary

Write to target/marker-audit/rust-only-toolkit.json:

{
  "agent": "rust-only-toolkit",
  "files_documented": ["toolkit_images"],
  "open_question_count": <number>,
  "coverage_gap_count": <number>,
  "summary": "<one paragraph of factual observations>"
}
```

- [ ] **Step 2: Verify agent output**

Check that marker file is non-empty:
Run: `wc -l src/emCore/toolkit_images.rust_only`
Expected: file has 20+ lines

Run: `cat target/marker-audit/rust-only-toolkit.json`
Expected: valid JSON with all fields populated

---

## Synthesis

### Task 7: Agent — synthesis

**Depends on:** Tasks 1–6 (all must complete first)

**Files:**
- Create: `docs/marker-audit-summary.md`

- [ ] **Step 1: Dispatch synthesis agent with the following prompt**

```
You are a synthesis agent consolidating evidence from 6 research agents that documented
marker files in an eaglemode-rs codebase. You produce FACTUAL SUMMARIES ONLY. No
classifications, no recommendations, no action items.

## Step A: Validate all marker files are documented

Read each of these 20 marker files and verify they are non-empty:

no_rust_equivalent:
- src/emCore/emAnything.no_rust_equivalent
- src/emCore/emArray.no_rust_equivalent
- src/emCore/emAvlTree.no_rust_equivalent
- src/emCore/emAvlTreeMap.no_rust_equivalent
- src/emCore/emAvlTreeSet.no_rust_equivalent
- src/emCore/emCrossPtr.no_rust_equivalent
- src/emCore/emFileStream.no_rust_equivalent
- src/emCore/emList.no_rust_equivalent
- src/emCore/emOwnPtr.no_rust_equivalent
- src/emCore/emOwnPtrArray.no_rust_equivalent
- src/emCore/emRef.no_rust_equivalent
- src/emCore/emString.no_rust_equivalent
- src/emCore/emThread.no_rust_equivalent
- src/emCore/emTmpFile.no_rust_equivalent
- src/emCore/emToolkit.no_rust_equivalent

rust_only:
- src/emCore/emPainterDrawList.rust_only
- src/emCore/fixed.rust_only
- src/emCore/rect.rust_only
- src/emCore/toolkit_images.rust_only
- src/emCore/widget_utils.rust_only

If any are still empty, note them as "NOT DOCUMENTED" in the summary.

## Step B: Read JSON summaries

Read all 6 JSON files from target/marker-audit/:
- target/marker-audit/stdlib-containers.json
- target/marker-audit/ownership-memory.json
- target/marker-audit/system-primitives.json
- target/marker-audit/framework-glue.json
- target/marker-audit/rust-only-infra.json
- target/marker-audit/rust-only-toolkit.json

## Step C: Collect all OPEN QUESTIONS

Read through every marker file and extract every line starting with "OPEN QUESTION:"
or appearing under an "OPEN QUESTIONS:" section. Group them by theme (e.g., "missing
methods", "structural divergence", "unclear purpose").

## Step D: Identify cross-cutting observations

Look for patterns that span multiple marker files:
- Are there observations that multiple agents made independently?
- Are there C++ features that appear across multiple no_rust_equivalent files?
- Do any rust_only files relate to functionality from no_rust_equivalent files?

Report these as factual observations, not interpretations.

## Step E: Write docs/marker-audit-summary.md

Write the summary with this structure:

# Marker File Audit Summary

Generated: 2026-03-27

## Overview

[One paragraph: how many files documented, how many open questions total, how many
coverage gaps total]

## File Summary Table

| Marker File | Type | C++ API Size | Rust Equivalents Found | Coverage Gaps | Open Questions |
|-------------|------|-------------|----------------------|---------------|----------------|
| emArray | no_rust_equivalent | N methods | [brief] | N gaps | N questions |
[... one row per file, 20 total]

## All Open Questions

### [Theme 1]
- [question] — from [marker file]
- [question] — from [marker file]

### [Theme 2]
- [question] — from [marker file]
[...]

## Cross-Cutting Observations
- [factual observation spanning multiple files]
- [factual observation spanning multiple files]
[...]

IMPORTANT: This summary contains NO classifications, NO recommendations, and NO action
items. It is a factual consolidation of evidence for human review.
```

- [ ] **Step 2: Verify synthesis output**

Run: `wc -l docs/marker-audit-summary.md`
Expected: file has 40+ lines

Run: `grep -c "OPEN QUESTION\|open question" docs/marker-audit-summary.md`
Expected: at least 1 (confirms open questions were collected)

---

## Finalize

### Task 8: Commit all results

**Depends on:** Task 7

- [ ] **Step 1: Review what changed**

Run: `git status`
Run: `git diff --stat`
Expected: 20 modified marker files + 1 new docs/marker-audit-summary.md

- [ ] **Step 2: Commit**

```bash
git add src/emCore/*.no_rust_equivalent src/emCore/*.rust_only docs/marker-audit-summary.md
git commit -m "docs: document all 20 marker files with C++/Rust correspondence evidence

Parallel agent audit of all no_rust_equivalent and rust_only marker files
in src/emCore/. Each file now contains factual evidence: C++ API surface,
usage patterns, Rust equivalents found, coverage gaps, and open questions.

Summary: docs/marker-audit-summary.md

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
```

- [ ] **Step 3: Verify commit**

Run: `git log --oneline -1`
Run: `git diff HEAD~1 --stat`
Expected: 21 files changed
