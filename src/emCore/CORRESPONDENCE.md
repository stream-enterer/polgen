# Cross-Cutting Patterns Across Marker Files

Patterns that span multiple marker files and are not visible by reading
any single file in isolation. Each pattern names the concern and lists
the files where evidence is documented.

## COW semantics not replicated

C++ copy-on-write (shared data, deep copy on mutation) appears in 5
types. Rust uses move semantics and Clone throughout. Whether any code
depends on COW behavior is NOT VERIFIED in any of these files.

- emArray.no_rs
- emList.no_rs
- emString.no_rs
- emAvlTreeMap.no_rs
- emAvlTreeSet.no_rs

## Stable iterators not replicated

C++ iterators that survive mutations (auto-adjust on element removal,
auto-adjust on COW clone) appear in the same 5 types plus emAvlTree.
Rust iterators borrow the collection immutably. Whether any code
mutates while iterating is NOT VERIFIED in any of these files.

- emArray.no_rs
- emList.no_rs
- emAvlTree.no_rs
- emAvlTreeMap.no_rs
- emAvlTreeSet.no_rs

## Zero emCore consumers with outside-emCore usage

Types that appear unused from within emCore but are consumed by
eaglemode apps. Each file has a NOTE about this. Gaps will surface
when those apps are ported.

- emFileStream.no_rs (13 outside files — all image format loaders)
- emAvlTreeSet.no_rs (4 outside files — emOsm, emStocks)
- emTmpFile.no_rs (2 outside files — emTmpConv)

## Workaround for missing feature

Rust code that reimplements part of an unported C++ type's functionality
under a different name, without referencing the original type.

- emResTga.rs decodes TGA from &[u8], working around missing emFileStream
  (documented in emFileStream.no_rs)
- emFontCache.rs uses OnceLock<emImage> single atlas, replacing C++
  emOwnPtrArray<Entry> dynamic cache + emRef/emModel shared ownership
  (documented in emOwnPtrArray.no_rs and emRef.no_rs)

## Concrete rendering/feature gaps

C++ functionality with no Rust counterpart where the gap affects
visible output or user-facing features.

- toolkit_images.rust_only: ImgTunnel missing — emTunnel rendering gap
- toolkit_images.rust_only: ImgDir/ImgDirUp missing — file selection icons
- emCrossPtr.no_rs: emBorder PanelPointerCache has no Rust counterpart
- emCrossPtr.no_rs: emFileDialog OverwriteDialog has no Rust counterpart

## Encoding risk

C++ emString is byte-oriented; Rust String enforces UTF-8. File paths
on Unix can contain non-UTF-8 bytes. This affects any code that stores
file paths in strings.

- emString.no_rs

## Architectural divergence chain

The threading model change and the record-replay pattern are causally
linked: panel state uses Rc (emLook.rs:22), Rc is not Send, therefore
user paint code cannot run on worker threads, therefore record-replay
was introduced.

- emThread.no_rs (threading model change)
- emPainterDrawList.rust_only (record-replay pattern)

## BreakCrossPtrs timing

C++ invalidates cross pointers early in destructors (before cleanup).
Rust Weak invalidates only when last Rc drops (after cleanup). Whether
any code checks a cross pointer during the target's destruction is
NOT VERIFIED.

- emCrossPtr.no_rs
