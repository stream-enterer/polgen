use std::collections::HashMap;

use slotmap::{new_key_type, SlotMap};

use super::behavior::{NoticeFlags, PanelBehavior};
use super::ctx::PanelCtx;
use crate::foundation::{Color, Rect};

// ── Identity encode/decode free functions ─────────────────────────────

/// Encode an array of panel names into a colon-delimited identity string,
/// escaping `:` and `\` with backslash prefixes.
///
/// Corresponds to `emPanel::EncodeIdentity`.
pub fn encode_identity(names: &[&str]) -> String {
    // First pass: compute total length for the output
    let mut len = 0usize;
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            len += 1; // ':'
        }
        for ch in name.chars() {
            if ch == ':' || ch == '\\' {
                len += 2; // escape char + original
            } else {
                len += ch.len_utf8();
            }
        }
    }

    let mut result = String::with_capacity(len);
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            result.push(':');
        }
        for ch in name.chars() {
            if ch == ':' || ch == '\\' {
                result.push('\\');
            }
            result.push(ch);
        }
    }
    result
}

/// Decode a colon-delimited identity string back into a list of panel names,
/// handling backslash-escaped colons and backslashes.
///
/// Corresponds to `emPanel::DecodeIdentity`.
pub fn decode_identity(identity: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = identity.as_bytes();
    let mut pos = 0;

    loop {
        if pos >= bytes.len() {
            break;
        }
        // Collect one name
        let mut name = String::new();
        loop {
            if pos >= bytes.len() {
                break;
            }
            let ch = bytes[pos] as char;
            if ch == ':' {
                // End of this name segment; skip the ':'
                pos += 1;
                break;
            }
            if ch == '\\' {
                pos += 1; // skip escape
                if pos >= bytes.len() {
                    break;
                }
                name.push(bytes[pos] as char);
                pos += 1;
            } else {
                name.push(ch);
                pos += 1;
            }
        }
        names.push(name);
    }
    names
}

new_key_type! {
    /// Unique handle for a panel in the panel tree.
    pub struct PanelId;
}

/// Data stored for each panel in the arena.
///
/// Fields are crate-internal. Use accessor methods on [`PanelTree`] for reading
/// panel state, and dedicated setters (e.g. `set_layout_rect`, `set_visible`)
/// for mutation.
pub(crate) struct PanelData {
    // Tree-managed linkage
    pub(crate) parent: Option<PanelId>,
    pub(crate) first_child: Option<PanelId>,
    pub(crate) last_child: Option<PanelId>,
    pub(crate) next_sibling: Option<PanelId>,
    pub(crate) prev_sibling: Option<PanelId>,

    // Identity
    pub(crate) name: String,

    // Layout & appearance
    pub(crate) layout_rect: Rect,
    pub(crate) canvas_color: Color,
    pub(crate) visible: bool,
    pub(crate) focusable: bool,

    // Enable state
    pub(crate) enable_switch: bool,
    /// Computed: true if this panel and all ancestors have enable_switch=true.
    pub(crate) enabled: bool,

    // Notices & behavior
    pub(crate) pending_notices: NoticeFlags,
    pub(crate) behavior: Option<Box<dyn PanelBehavior>>,

    // Viewing state (set by View::update_viewing each frame)
    pub(crate) viewed: bool,
    pub(crate) in_viewed_path: bool,
    pub(crate) in_active_path: bool,
    pub(crate) is_active: bool,
    pub(crate) viewed_x: f64,
    pub(crate) viewed_y: f64,
    pub(crate) viewed_width: f64,
    pub(crate) viewed_height: f64,
    pub(crate) clip_x: f64,
    pub(crate) clip_y: f64,
    pub(crate) clip_w: f64,
    pub(crate) clip_h: f64,
}

impl PanelData {
    fn new(name: String) -> Self {
        Self {
            parent: None,
            first_child: None,
            last_child: None,
            next_sibling: None,
            prev_sibling: None,
            name,
            layout_rect: Rect::default(),
            canvas_color: Color::TRANSPARENT,
            visible: true,
            focusable: false,
            enable_switch: true,
            enabled: true,
            pending_notices: NoticeFlags::empty(),
            behavior: None,
            viewed: false,
            in_viewed_path: false,
            in_active_path: false,
            is_active: false,
            viewed_x: 0.0,
            viewed_y: 0.0,
            viewed_width: 0.0,
            viewed_height: 0.0,
            clip_x: 0.0,
            clip_y: 0.0,
            clip_w: 0.0,
            clip_h: 0.0,
        }
    }
}

/// Arena-based panel tree using SlotMap for stable handles.
pub struct PanelTree {
    panels: SlotMap<PanelId, PanelData>,
    root: Option<PanelId>,
    /// Per-parent name index: (parent, child_name) → child_id.
    /// Root panels use their own id as the "parent" key.
    name_index: HashMap<(PanelId, String), PanelId>,
}

impl PanelTree {
    pub fn new() -> Self {
        Self {
            panels: SlotMap::with_key(),
            root: None,
            name_index: HashMap::new(),
        }
    }

    /// Create the root panel.
    ///
    /// # Panics
    /// Panics if a root panel already exists.
    pub fn create_root(&mut self, name: &str) -> PanelId {
        assert!(
            self.root.is_none(),
            "create_root called but root panel already exists"
        );
        let id = self.panels.insert(PanelData::new(name.to_string()));
        // Root uses its own id as the parent key
        self.name_index.insert((id, name.to_string()), id);
        self.root = Some(id);
        id
    }

    /// Create a child panel under the given parent.
    pub fn create_child(&mut self, parent: PanelId, name: &str) -> PanelId {
        let id = self.panels.insert(PanelData::new(name.to_string()));
        self.name_index.insert((parent, name.to_string()), id);

        // Link into parent's child list
        self.panels[id].parent = Some(parent);

        let prev_last = self.panels[parent].last_child;
        if let Some(prev) = prev_last {
            self.panels[prev].next_sibling = Some(id);
            self.panels[id].prev_sibling = Some(prev);
        } else {
            self.panels[parent].first_child = Some(id);
        }
        self.panels[parent].last_child = Some(id);

        // Inherit parent's enabled state
        self.recompute_enabled(id);

        // Notify parent
        self.panels[parent]
            .pending_notices
            .insert(NoticeFlags::CHILDREN_CHANGED);

        id
    }

    /// Remove a panel and all its descendants.
    pub fn remove(&mut self, id: PanelId) {
        // Collect all descendants first
        let descendants = self.collect_descendants(id);

        // Unlink from parent's child list
        if let Some(parent_id) = self.panels[id].parent {
            let prev = self.panels[id].prev_sibling;
            let next = self.panels[id].next_sibling;

            if let Some(prev_id) = prev {
                self.panels[prev_id].next_sibling = next;
            } else {
                self.panels[parent_id].first_child = next;
            }

            if let Some(next_id) = next {
                self.panels[next_id].prev_sibling = prev;
            } else {
                self.panels[parent_id].last_child = prev;
            }

            self.panels[parent_id]
                .pending_notices
                .insert(NoticeFlags::CHILDREN_CHANGED);
        }

        // Remove root reference if needed
        if self.root == Some(id) {
            self.root = None;
        }

        // Remove from arena and name index
        for desc_id in descendants {
            if let Some(data) = self.panels.remove(desc_id) {
                if let Some(parent_id) = data.parent {
                    self.name_index.remove(&(parent_id, data.name));
                }
            }
        }
        if let Some(data) = self.panels.remove(id) {
            if let Some(parent_id) = data.parent {
                self.name_index.remove(&(parent_id, data.name));
            } else {
                // Root panel uses itself as key
                self.name_index.remove(&(id, data.name));
            }
        }
    }

    /// Get the root panel ID.
    pub fn root(&self) -> Option<PanelId> {
        self.root
    }

    /// Get a panel's data (crate-internal).
    pub(crate) fn get(&self, id: PanelId) -> Option<&PanelData> {
        self.panels.get(id)
    }

    /// Get a panel's data mutably (crate-internal).
    pub(crate) fn get_mut(&mut self, id: PanelId) -> Option<&mut PanelData> {
        self.panels.get_mut(id)
    }

    // ── Public read accessors ──────────────────────────────────────────

    /// Get the panel's name.
    pub fn name(&self, id: PanelId) -> Option<&str> {
        self.panels.get(id).map(|p| p.name.as_str())
    }

    /// Get the layout rectangle.
    pub fn layout_rect(&self, id: PanelId) -> Option<Rect> {
        self.panels.get(id).map(|p| p.layout_rect)
    }

    /// Get the canvas color.
    pub fn canvas_color(&self, id: PanelId) -> Option<Color> {
        self.panels.get(id).map(|p| p.canvas_color)
    }

    /// Whether the panel is visible.
    pub fn visible(&self, id: PanelId) -> bool {
        self.panels.get(id).map(|p| p.visible).unwrap_or(false)
    }

    /// Whether the panel can receive input focus.
    pub fn focusable(&self, id: PanelId) -> bool {
        self.panels.get(id).map(|p| p.focusable).unwrap_or(false)
    }

    /// Whether the panel is enabled (computed from enable_switch and ancestors).
    pub fn enabled(&self, id: PanelId) -> bool {
        self.panels.get(id).map(|p| p.enabled).unwrap_or(false)
    }

    /// Get pending notice flags.
    pub fn pending_notices(&self, id: PanelId) -> NoticeFlags {
        self.panels
            .get(id)
            .map(|p| p.pending_notices)
            .unwrap_or_else(NoticeFlags::empty)
    }

    // ── Public write accessors ─────────────────────────────────────────

    /// Set whether the panel is visible.
    pub fn set_visible(&mut self, id: PanelId, visible: bool) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.visible = visible;
        }
    }

    /// Set whether the panel can receive input focus.
    pub fn set_focusable(&mut self, id: PanelId, focusable: bool) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.focusable = focusable;
        }
    }

    /// Look up a child panel by parent and name.
    pub fn find_child_by_name(&self, parent: PanelId, name: &str) -> Option<PanelId> {
        self.name_index.get(&(parent, name.to_string())).copied()
    }

    /// Look up a panel by name (searches all panels).
    pub fn find_by_name(&self, name: &str) -> Option<PanelId> {
        self.panels
            .iter()
            .find(|(_, data)| data.name == name)
            .map(|(id, _)| id)
    }

    /// Check if a panel exists.
    pub fn contains(&self, id: PanelId) -> bool {
        self.panels.contains_key(id)
    }

    /// Get the total number of panels.
    pub fn len(&self) -> usize {
        self.panels.len()
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    /// Iterate over children of a panel.
    pub fn children(&self, parent: PanelId) -> ChildIter<'_> {
        let first = self.panels.get(parent).and_then(|p| p.first_child);
        ChildIter {
            tree: self,
            current: first,
        }
    }

    /// Get the number of children.
    pub fn child_count(&self, parent: PanelId) -> usize {
        self.children(parent).count()
    }

    /// Get the parent of a panel.
    pub fn parent(&self, id: PanelId) -> Option<PanelId> {
        self.panels.get(id).and_then(|p| p.parent)
    }

    /// Build a colon-delimited identity string by walking from `id` up to the
    /// root, collecting names, and encoding them.
    ///
    /// Corresponds to `emPanel::GetIdentity`.
    pub fn get_identity(&self, id: PanelId) -> String {
        // Walk up to root collecting names
        let mut names = Vec::new();
        let mut cur = id;
        while let Some(panel) = self.panels.get(cur) {
            names.push(panel.name.as_str());
            match panel.parent {
                Some(parent) => cur = parent,
                None => break,
            }
        }
        // names is child-to-root; reverse to root-to-child
        names.reverse();
        encode_identity(&names)
    }

    // ── Sibling reordering ───────────────────────────────────────────

    /// Unlink a panel from its position in the sibling chain, without
    /// removing it from the arena or name index. The panel must have a parent.
    fn unlink_sibling(&mut self, id: PanelId) {
        let prev = self.panels[id].prev_sibling;
        let next = self.panels[id].next_sibling;
        let parent = self.panels[id]
            .parent
            .expect("unlink_sibling called on root panel");

        if let Some(prev_id) = prev {
            self.panels[prev_id].next_sibling = next;
        } else {
            self.panels[parent].first_child = next;
        }

        if let Some(next_id) = next {
            self.panels[next_id].prev_sibling = prev;
        } else {
            self.panels[parent].last_child = prev;
        }

        self.panels[id].prev_sibling = None;
        self.panels[id].next_sibling = None;
    }

    /// After a sibling reorder, notify parent of child list change.
    fn notify_sibling_reorder(&mut self, id: PanelId) {
        if let Some(parent) = self.panels[id].parent {
            self.panels[parent]
                .pending_notices
                .insert(NoticeFlags::CHILDREN_CHANGED);
        }
    }

    /// Move this panel to the front (first) of its parent's child list.
    /// No-op if already first or if the panel is the root.
    ///
    /// Corresponds to `emPanel::BeFirst`.
    pub fn be_first(&mut self, id: PanelId) {
        // No-op if no parent or already first
        let parent = match self.panels.get(id).and_then(|p| p.parent) {
            Some(p) => p,
            None => return,
        };
        if self.panels[id].prev_sibling.is_none() {
            return;
        }

        self.unlink_sibling(id);

        // Relink as first child
        let old_first = self.panels[parent].first_child;
        self.panels[id].next_sibling = old_first;
        if let Some(old_first_id) = old_first {
            self.panels[old_first_id].prev_sibling = Some(id);
        }
        self.panels[parent].first_child = Some(id);
        if self.panels[parent].last_child.is_none() {
            self.panels[parent].last_child = Some(id);
        }

        self.notify_sibling_reorder(id);
    }

    /// Move this panel to the end (last) of its parent's child list.
    /// No-op if already last or if the panel is the root.
    ///
    /// Corresponds to `emPanel::BeLast`.
    pub fn be_last(&mut self, id: PanelId) {
        let parent = match self.panels.get(id).and_then(|p| p.parent) {
            Some(p) => p,
            None => return,
        };
        if self.panels[id].next_sibling.is_none() {
            return;
        }

        self.unlink_sibling(id);

        // Relink as last child
        let old_last = self.panels[parent].last_child;
        self.panels[id].prev_sibling = old_last;
        if let Some(old_last_id) = old_last {
            self.panels[old_last_id].next_sibling = Some(id);
        }
        self.panels[parent].last_child = Some(id);
        if self.panels[parent].first_child.is_none() {
            self.panels[parent].first_child = Some(id);
        }

        self.notify_sibling_reorder(id);
    }

    /// Move this panel to be immediately before the given sibling.
    /// If `sibling` is `None`, calls [`be_last`](Self::be_last).
    /// No-op if `sibling` is this panel, is already the next sibling, or has
    /// a different parent.
    ///
    /// Corresponds to `emPanel::BePrevOf`.
    pub fn be_prev_of(&mut self, id: PanelId, sibling: Option<PanelId>) {
        let sibling = match sibling {
            Some(s) => s,
            None => {
                self.be_last(id);
                return;
            }
        };

        // No-op checks
        if sibling == id {
            return;
        }
        if self.panels[id].next_sibling == Some(sibling) {
            return;
        }
        let my_parent = self.panels[id].parent;
        let sib_parent = self.panels[sibling].parent;
        if my_parent != sib_parent || my_parent.is_none() {
            return;
        }
        let parent = my_parent.expect("checked above");

        self.unlink_sibling(id);

        // Insert before sibling
        let sib_prev = self.panels[sibling].prev_sibling;
        self.panels[id].next_sibling = Some(sibling);
        self.panels[id].prev_sibling = sib_prev;
        self.panels[sibling].prev_sibling = Some(id);
        if let Some(prev_id) = sib_prev {
            self.panels[prev_id].next_sibling = Some(id);
        } else {
            self.panels[parent].first_child = Some(id);
        }

        self.notify_sibling_reorder(id);
    }

    /// Move this panel to be immediately after the given sibling.
    /// If `sibling` is `None`, calls [`be_first`](Self::be_first).
    /// No-op if `sibling` is this panel, is already the prev sibling, or has
    /// a different parent.
    ///
    /// Corresponds to `emPanel::BeNextOf`.
    pub fn be_next_of(&mut self, id: PanelId, sibling: Option<PanelId>) {
        let sibling = match sibling {
            Some(s) => s,
            None => {
                self.be_first(id);
                return;
            }
        };

        // No-op checks
        if sibling == id {
            return;
        }
        if self.panels[id].prev_sibling == Some(sibling) {
            return;
        }
        let my_parent = self.panels[id].parent;
        let sib_parent = self.panels[sibling].parent;
        if my_parent != sib_parent || my_parent.is_none() {
            return;
        }
        let parent = my_parent.expect("checked above");

        self.unlink_sibling(id);

        // Insert after sibling
        let sib_next = self.panels[sibling].next_sibling;
        self.panels[id].prev_sibling = Some(sibling);
        self.panels[id].next_sibling = sib_next;
        self.panels[sibling].next_sibling = Some(id);
        if let Some(next_id) = sib_next {
            self.panels[next_id].prev_sibling = Some(id);
        } else {
            self.panels[parent].last_child = Some(id);
        }

        self.notify_sibling_reorder(id);
    }

    /// Sort the children of a panel using the given comparator.
    /// Notifies `CHILDREN_CHANGED` only if the order actually changed.
    ///
    /// Corresponds to `emPanel::SortChildren`.
    pub fn sort_children<F>(&mut self, parent: PanelId, mut compare: F)
    where
        F: FnMut(PanelId, PanelId) -> std::cmp::Ordering,
    {
        // Collect children into a vec
        let children: Vec<PanelId> = self.children(parent).collect();
        if children.len() <= 1 {
            return;
        }

        // Sort
        let mut sorted = children.clone();
        sorted.sort_by(|&a, &b| compare(a, b));

        // Check if order actually changed
        if sorted == children {
            return;
        }

        // Relink the sibling chain according to sorted order
        for (i, &child) in sorted.iter().enumerate() {
            self.panels[child].prev_sibling = if i > 0 { Some(sorted[i - 1]) } else { None };
            self.panels[child].next_sibling = if i + 1 < sorted.len() {
                Some(sorted[i + 1])
            } else {
                None
            };
        }
        self.panels[parent].first_child = Some(sorted[0]);
        self.panels[parent].last_child = Some(sorted[sorted.len() - 1]);

        self.panels[parent]
            .pending_notices
            .insert(NoticeFlags::CHILDREN_CHANGED);
    }

    // ── Title / Icon ─────────────────────────────────────────────────

    /// Walk up the parent chain trying each panel's behavior for a title.
    /// If no behavior provides one, the root returns `"untitled"`.
    ///
    /// Corresponds to `emPanel::GetTitle`.
    pub fn get_title(&self, id: PanelId) -> String {
        let mut cur = id;
        loop {
            if let Some(panel) = self.panels.get(cur) {
                if let Some(ref behavior) = panel.behavior {
                    if let Some(title) = behavior.get_title() {
                        return title;
                    }
                }
                match panel.parent {
                    Some(parent) => cur = parent,
                    None => return "untitled".to_string(),
                }
            } else {
                return "untitled".to_string();
            }
        }
    }

    /// Walk up the parent chain trying each panel's behavior for an icon
    /// filename. If no behavior provides one, the root returns `""`.
    ///
    /// Corresponds to `emPanel::GetIconFileName`.
    pub fn get_icon_file_name(&self, id: PanelId) -> String {
        let mut cur = id;
        loop {
            if let Some(panel) = self.panels.get(cur) {
                if let Some(ref behavior) = panel.behavior {
                    if let Some(name) = behavior.get_icon_file_name() {
                        return name;
                    }
                }
                match panel.parent {
                    Some(parent) => cur = parent,
                    None => return String::new(),
                }
            } else {
                return String::new();
            }
        }
    }

    /// Remove all children of a panel.
    pub fn delete_all_children(&mut self, parent: PanelId) {
        let children: Vec<PanelId> = self.children(parent).collect();
        for child in children {
            self.remove(child);
        }
    }

    /// Set the layout rectangle for a panel.
    ///
    /// Width and height are clamped to a minimum of `1e-100` to prevent
    /// division-by-zero when computing tallness.
    pub fn set_layout_rect(&mut self, id: PanelId, x: f64, y: f64, w: f64, h: f64) {
        let rect = Rect {
            x,
            y,
            w: w.max(1e-100),
            h: h.max(1e-100),
        };
        if let Some(panel) = self.panels.get_mut(id) {
            if panel.layout_rect == rect {
                return;
            }
            panel.layout_rect = rect;
            panel.pending_notices.insert(NoticeFlags::LAYOUT_CHANGED);
        }
    }

    /// Set the canvas color for a panel.
    pub fn set_canvas_color(&mut self, id: PanelId, color: Color) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.canvas_color = color;
            panel.pending_notices.insert(NoticeFlags::CANVAS_CHANGED);
        }
    }

    /// Set the enable switch for a panel and recompute enabled state for descendants.
    pub fn set_enable_switch(&mut self, id: PanelId, enable: bool) {
        if let Some(panel) = self.panels.get_mut(id) {
            if panel.enable_switch == enable {
                return;
            }
            panel.enable_switch = enable;
        }
        self.recompute_enabled(id);
    }

    /// Recompute the `enabled` field for a panel and its descendants.
    fn recompute_enabled(&mut self, id: PanelId) {
        let parent_enabled = self
            .panels
            .get(id)
            .and_then(|p| p.parent)
            .and_then(|pid| self.panels.get(pid))
            .map(|p| p.enabled)
            .unwrap_or(true);

        if let Some(panel) = self.panels.get_mut(id) {
            let new_enabled = panel.enable_switch && parent_enabled;
            if panel.enabled != new_enabled {
                panel.enabled = new_enabled;
                panel.pending_notices.insert(NoticeFlags::ENABLE_CHANGED);
            }
        }

        // Recurse into children
        let child_ids: Vec<PanelId> = self.children(id).collect();
        for child_id in child_ids {
            self.recompute_enabled(child_id);
        }
    }

    /// Set the behavior for a panel.
    pub fn set_behavior(&mut self, id: PanelId, behavior: Box<dyn PanelBehavior>) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.behavior = Some(behavior);
        }
    }

    /// Extract the behavior from a panel (for calling methods that need &mut self on tree).
    pub fn take_behavior(&mut self, id: PanelId) -> Option<Box<dyn PanelBehavior>> {
        self.panels.get_mut(id).and_then(|p| p.behavior.take())
    }

    /// Put the behavior back after extraction.
    pub fn put_behavior(&mut self, id: PanelId, behavior: Box<dyn PanelBehavior>) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.behavior = Some(behavior);
        }
    }

    /// Deliver pending notices to all panels with behaviors.
    pub fn deliver_notices(&mut self) {
        let ids: Vec<PanelId> = self.panels.keys().collect();
        for id in ids {
            let flags = self.panels[id].pending_notices;
            if flags.is_empty() {
                continue;
            }
            self.panels[id].pending_notices = NoticeFlags::empty();
            if let Some(mut behavior) = self.take_behavior(id) {
                behavior.notice(flags);
                if flags.contains(NoticeFlags::LAYOUT_CHANGED) {
                    let mut ctx = PanelCtx::new(self, id);
                    behavior.layout_children(&mut ctx);
                }
                self.put_behavior(id, behavior);
            }
        }
    }

    /// Walk from `id` to root, returning ancestor chain (id first, root last).
    pub fn ancestors(&self, id: PanelId) -> Vec<PanelId> {
        let mut result = vec![id];
        let mut cur = id;
        while let Some(parent) = self.panels.get(cur).and_then(|p| p.parent) {
            result.push(parent);
            cur = parent;
        }
        result
    }

    /// Iterate children in reverse order (last_child → first_child).
    pub fn children_rev(&self, parent: PanelId) -> ChildRevIter<'_> {
        let last = self.panels.get(parent).and_then(|p| p.last_child);
        ChildRevIter {
            tree: self,
            current: last,
        }
    }

    /// Find nearest focusable ancestor of `id` (including self).
    pub fn focusable_ancestor(&self, id: PanelId) -> Option<PanelId> {
        let mut cur = Some(id);
        while let Some(c) = cur {
            if self.panels.get(c).map(|p| p.focusable).unwrap_or(false) {
                return Some(c);
            }
            cur = self.panels.get(c).and_then(|p| p.parent);
        }
        None
    }

    // ── Coordinate transforms ─────────────────────────────────────────

    /// Convert panel-space X to view-space X.
    pub fn panel_to_view_x(&self, id: PanelId, x: f64) -> f64 {
        let p = &self.panels[id];
        p.viewed_x + x * p.viewed_width
    }

    /// Convert panel-space Y to view-space Y.
    pub fn panel_to_view_y(&self, id: PanelId, y: f64) -> f64 {
        let p = &self.panels[id];
        p.viewed_y + y * p.viewed_height
    }

    /// Convert view-space X to panel-space X.
    pub fn view_to_panel_x(&self, id: PanelId, vx: f64) -> f64 {
        let p = &self.panels[id];
        (vx - p.viewed_x) / p.viewed_width
    }

    /// Convert view-space Y to panel-space Y.
    pub fn view_to_panel_y(&self, id: PanelId, vy: f64) -> f64 {
        let p = &self.panels[id];
        (vy - p.viewed_y) / p.viewed_height
    }

    /// Convert a panel-space delta X to view-space delta X.
    pub fn panel_to_view_delta_x(&self, id: PanelId, dx: f64) -> f64 {
        dx * self.panels[id].viewed_width
    }

    /// Convert a panel-space delta Y to view-space delta Y.
    pub fn panel_to_view_delta_y(&self, id: PanelId, dy: f64) -> f64 {
        dy * self.panels[id].viewed_height
    }

    /// Convert a view-space delta X to panel-space delta X.
    pub fn view_to_panel_delta_x(&self, id: PanelId, dvx: f64) -> f64 {
        dvx / self.panels[id].viewed_width
    }

    /// Convert a view-space delta Y to panel-space delta Y.
    pub fn view_to_panel_delta_y(&self, id: PanelId, dvy: f64) -> f64 {
        dvy / self.panels[id].viewed_height
    }

    // ── Geometry accessors ───────────────────────────────────────────

    /// Panel height in its own coordinate system: `layout_h / layout_w`.
    ///
    /// In the C++ source this is `GetHeight()` / `GetTallness()`.
    pub fn get_height(&self, id: PanelId) -> f64 {
        let p = &self.panels[id];
        p.layout_rect.h / p.layout_rect.w
    }

    /// Alias for [`get_height`](Self::get_height).
    pub fn get_tallness(&self, id: PanelId) -> f64 {
        self.get_height(id)
    }

    /// Return the substance rectangle and corner radius for a panel.
    ///
    /// The base `emPanel` implementation returns `(0, 0, 1, GetHeight(), 0)` --
    /// i.e. the full panel rect with zero radius. Subclass overrides (border
    /// panels) may return a smaller rect with a nonzero radius; those will be
    /// handled by the behavior trait. This method provides the default.
    pub fn get_substance_rect(&self, id: PanelId) -> (f64, f64, f64, f64, f64) {
        let h = self.get_height(id);
        (0.0, 0.0, 1.0, h, 0.0)
    }

    /// Test whether a point lies inside the substance rectangle (with rounded
    /// corners).
    pub fn is_point_in_substance_rect(&self, id: PanelId, x: f64, y: f64) -> bool {
        let h = self.get_height(id);

        // Quick rejection: outside panel bounds
        if !(0.0..1.0).contains(&x) || !(0.0..h).contains(&y) {
            return false;
        }

        let (sx, sy, sw, sh, sr) = self.get_substance_rect(id);
        let sw2 = sw * 0.5;
        let sh2 = sh * 0.5;

        // Distance from center of substance rect
        let dx = (x - sx - sw2).abs();
        let dy = (y - sy - sh2).abs();

        // Outside substance rect entirely
        if dx > sw2 || dy > sh2 {
            return false;
        }

        // Clamp radius to half-dimensions
        let r = sr.min(sw2).min(sh2);

        // Distance from the inner rect edge (where rounding begins)
        let cdx = dx - (sw2 - r);
        let cdy = dy - (sh2 - r);

        // Inside the non-rounded portion
        if cdx < 0.0 || cdy < 0.0 {
            return true;
        }

        // Corner arc test
        cdx * cdx + cdy * cdy <= r * r
    }

    /// Return the essence rectangle -- the substance rectangle without the
    /// corner-radius inset.
    pub fn get_essence_rect(&self, id: PanelId) -> (f64, f64, f64, f64) {
        let (sx, sy, sw, sh, _sr) = self.get_substance_rect(id);
        (sx, sy, sw, sh)
    }

    // ── Focusable navigation ─────────────────────────────────────────

    /// DFS for the first focusable descendant of `id`.
    pub fn focusable_first_child(&self, id: PanelId) -> Option<PanelId> {
        let mut p = self.panels.get(id)?.first_child?;
        loop {
            if self.panels[p].focusable {
                return Some(p);
            }
            if let Some(child) = self.panels[p].first_child {
                p = child;
                continue;
            }
            // Backtrack
            loop {
                if let Some(next) = self.panels[p].next_sibling {
                    p = next;
                    break;
                }
                let parent = self.panels[p].parent?;
                if parent == id {
                    return None;
                }
                p = parent;
            }
        }
    }

    /// Reverse DFS for the last focusable descendant of `id`.
    pub fn focusable_last_child(&self, id: PanelId) -> Option<PanelId> {
        let mut p = self.panels.get(id)?.last_child?;
        loop {
            if self.panels[p].focusable {
                return Some(p);
            }
            if let Some(child) = self.panels[p].last_child {
                p = child;
                continue;
            }
            // Backtrack
            loop {
                if let Some(prev) = self.panels[p].prev_sibling {
                    p = prev;
                    break;
                }
                let parent = self.panels[p].parent?;
                if parent == id {
                    return None;
                }
                p = parent;
            }
        }
    }

    /// Find the previous focusable panel relative to `id` in pre-order
    /// traversal. Searches within the same focusable ancestor boundary.
    pub fn focusable_prev(&self, id: PanelId) -> Option<PanelId> {
        let mut p = id;
        loop {
            match self.panels[p].prev_sibling {
                Some(prev) => {
                    p = prev;
                    loop {
                        if self.panels[p].focusable {
                            return Some(p);
                        }
                        match self.panels[p].last_child {
                            Some(child) => p = child,
                            None => break,
                        }
                    }
                }
                None => {
                    p = self.panels[p].parent?;
                    if self.panels[p].focusable {
                        return None;
                    }
                }
            }
        }
    }

    /// Find the next focusable panel relative to `id` in pre-order
    /// traversal. Searches within the same focusable ancestor boundary.
    pub fn focusable_next(&self, id: PanelId) -> Option<PanelId> {
        let mut p = id;
        loop {
            match self.panels[p].next_sibling {
                Some(next) => {
                    p = next;
                    loop {
                        if self.panels[p].focusable {
                            return Some(p);
                        }
                        match self.panels[p].first_child {
                            Some(child) => p = child,
                            None => break,
                        }
                    }
                }
                None => {
                    p = self.panels[p].parent?;
                    if self.panels[p].focusable {
                        return None;
                    }
                }
            }
        }
    }

    /// Clear all viewing flags on all panels.
    pub fn clear_viewing_flags(&mut self) {
        for (_, panel) in self.panels.iter_mut() {
            panel.viewed = false;
            panel.in_viewed_path = false;
            panel.in_active_path = false;
            panel.is_active = false;
            panel.viewed_x = 0.0;
            panel.viewed_y = 0.0;
            panel.viewed_width = 0.0;
            panel.viewed_height = 0.0;
            panel.clip_x = 0.0;
            panel.clip_y = 0.0;
            panel.clip_w = 0.0;
            panel.clip_h = 0.0;
        }
    }

    /// Get all panel IDs.
    pub fn all_ids(&self) -> Vec<PanelId> {
        self.panels.keys().collect()
    }

    fn collect_descendants(&self, id: PanelId) -> Vec<PanelId> {
        let mut result = Vec::new();
        let mut stack = Vec::new();
        if let Some(panel) = self.panels.get(id) {
            if let Some(child) = panel.first_child {
                stack.push(child);
            }
        }
        while let Some(current) = stack.pop() {
            result.push(current);
            if let Some(panel) = self.panels.get(current) {
                if let Some(child) = panel.first_child {
                    stack.push(child);
                }
                if let Some(next) = panel.next_sibling {
                    stack.push(next);
                }
            }
        }
        result
    }
}

impl Default for PanelTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over children of a panel.
pub struct ChildIter<'a> {
    tree: &'a PanelTree,
    current: Option<PanelId>,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = PanelId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current?;
        self.current = self.tree.panels.get(id).and_then(|p| p.next_sibling);
        Some(id)
    }
}

/// Iterator over children of a panel in reverse order (last -> first).
pub struct ChildRevIter<'a> {
    tree: &'a PanelTree,
    current: Option<PanelId>,
}

impl<'a> Iterator for ChildRevIter<'a> {
    type Item = PanelId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current?;
        self.current = self.tree.panels.get(id).and_then(|p| p.prev_sibling);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tree:
    ///   root (focusable)
    ///     a (not focusable)
    ///       a1 (focusable)
    ///       a2 (focusable)
    ///     b (focusable)
    ///     c (not focusable)
    ///       c1 (not focusable)
    ///         c1a (focusable)
    fn make_tree() -> (
        PanelTree,
        PanelId,
        PanelId,
        PanelId,
        PanelId,
        PanelId,
        PanelId,
    ) {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        t.set_focusable(root, true);
        t.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);

        let a = t.create_child(root, "a");
        t.set_layout_rect(a, 0.0, 0.0, 0.5, 0.5);

        let a1 = t.create_child(a, "a1");
        t.set_focusable(a1, true);
        t.set_layout_rect(a1, 0.0, 0.0, 0.5, 1.0);

        let a2 = t.create_child(a, "a2");
        t.set_focusable(a2, true);
        t.set_layout_rect(a2, 0.5, 0.0, 0.5, 1.0);

        let b = t.create_child(root, "b");
        t.set_focusable(b, true);
        t.set_layout_rect(b, 0.5, 0.0, 0.5, 0.5);

        let c = t.create_child(root, "c");
        t.set_layout_rect(c, 0.0, 0.5, 1.0, 0.5);

        let c1 = t.create_child(c, "c1");
        t.set_layout_rect(c1, 0.0, 0.0, 1.0, 1.0);

        let c1a = t.create_child(c1, "c1a");
        t.set_focusable(c1a, true);
        t.set_layout_rect(c1a, 0.0, 0.0, 1.0, 1.0);

        (t, root, a1, a2, b, c1a, c)
    }

    #[test]
    fn test_get_height_and_tallness() {
        let mut t = PanelTree::new();
        let root = t.create_root("r");
        t.set_layout_rect(root, 0.0, 0.0, 2.0, 6.0);
        assert!((t.get_height(root) - 3.0).abs() < 1e-12);
        assert!((t.get_tallness(root) - t.get_height(root)).abs() < 1e-15);
    }

    #[test]
    fn test_substance_rect_default() {
        let mut t = PanelTree::new();
        let root = t.create_root("r");
        t.set_layout_rect(root, 0.0, 0.0, 2.0, 4.0);
        let (sx, sy, sw, sh, sr) = t.get_substance_rect(root);
        assert_eq!((sx, sy, sw), (0.0, 0.0, 1.0));
        assert!((sh - 2.0).abs() < 1e-12);
        assert_eq!(sr, 0.0);
    }

    #[test]
    fn test_point_in_substance_rect() {
        let mut t = PanelTree::new();
        let root = t.create_root("r");
        t.set_layout_rect(root, 0.0, 0.0, 1.0, 2.0);
        assert!(t.is_point_in_substance_rect(root, 0.5, 1.0));
        assert!(t.is_point_in_substance_rect(root, 0.0, 0.0));
        assert!(!t.is_point_in_substance_rect(root, 1.0, 0.0));
        assert!(!t.is_point_in_substance_rect(root, 0.5, 2.0));
        assert!(!t.is_point_in_substance_rect(root, -0.1, 0.5));
    }

    #[test]
    fn test_essence_rect() {
        let mut t = PanelTree::new();
        let root = t.create_root("r");
        t.set_layout_rect(root, 0.0, 0.0, 1.0, 3.0);
        let (ex, ey, ew, eh) = t.get_essence_rect(root);
        assert_eq!((ex, ey, ew), (0.0, 0.0, 1.0));
        assert!((eh - 3.0).abs() < 1e-12);
    }

    #[test]
    fn test_focusable_first_child() {
        let (t, root, a1, _a2, _b, _c1a, _c) = make_tree();
        assert_eq!(t.focusable_first_child(root), Some(a1));
    }

    #[test]
    fn test_focusable_last_child() {
        let (t, root, _a1, _a2, _b, c1a, _c) = make_tree();
        assert_eq!(t.focusable_last_child(root), Some(c1a));
    }

    #[test]
    fn test_focusable_first_child_none() {
        let mut t = PanelTree::new();
        let root = t.create_root("r");
        let _child = t.create_child(root, "c");
        assert_eq!(t.focusable_first_child(root), None);
    }

    #[test]
    fn test_focusable_next_prev() {
        let (t, _root, a1, a2, _b, _c1a, _c) = make_tree();
        assert_eq!(t.focusable_next(a1), Some(a2));
        assert_eq!(t.focusable_prev(a2), Some(a1));
        assert_eq!(t.focusable_prev(a1), None);
    }

    #[test]
    fn test_focusable_next_crosses_subtree() {
        let (t, _root, _a1, a2, b, _c1a, _c) = make_tree();
        // a2 -> next: walk up to 'a' (not focusable), a.next = b (focusable)
        assert_eq!(t.focusable_next(a2), Some(b));
    }

    // ── Identity tests ───────────────────────────────────────────────

    #[test]
    fn test_encode_identity_basic() {
        assert_eq!(
            encode_identity(&["root", "child", "leaf"]),
            "root:child:leaf"
        );
    }

    #[test]
    fn test_encode_identity_escaping() {
        assert_eq!(encode_identity(&["a:b", "c\\d"]), r"a\:b:c\\d");
    }

    #[test]
    fn test_encode_identity_empty() {
        assert_eq!(encode_identity(&[]), "");
        assert_eq!(encode_identity(&[""]), "");
    }

    #[test]
    fn test_decode_identity_basic() {
        assert_eq!(
            decode_identity("root:child:leaf"),
            vec!["root", "child", "leaf"]
        );
    }

    #[test]
    fn test_decode_identity_escaping() {
        assert_eq!(decode_identity(r"a\:b:c\\d"), vec!["a:b", "c\\d"]);
    }

    #[test]
    fn test_decode_identity_empty_segments() {
        assert_eq!(decode_identity("a::b"), vec!["a", "", "b"]);
    }

    #[test]
    fn test_encode_decode_round_trip() {
        let names = vec!["root", "child:with:colons", "back\\slash"];
        let encoded = encode_identity(&names);
        let decoded = decode_identity(&encoded);
        let expected: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_get_identity() {
        let (t, root, a1, _a2, _b, c1a, _c) = make_tree();
        assert_eq!(t.get_identity(root), "root");
        assert_eq!(t.get_identity(a1), "root:a:a1");
        assert_eq!(t.get_identity(c1a), "root:c:c1:c1a");
    }

    // ── Sibling reordering tests ─────────────────────────────────────

    /// Helper: collect children names in order.
    fn child_names(t: &PanelTree, parent: PanelId) -> Vec<String> {
        t.children(parent)
            .map(|id| t.name(id).unwrap().to_string())
            .collect()
    }

    #[test]
    fn test_be_first() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let a = t.create_child(root, "a");
        let b = t.create_child(root, "b");
        let c = t.create_child(root, "c");

        // Move c to front
        t.be_first(c);
        assert_eq!(child_names(&t, root), vec!["c", "a", "b"]);

        // Move c again (already first → no-op)
        t.be_first(c);
        assert_eq!(child_names(&t, root), vec!["c", "a", "b"]);

        // Move b to front
        t.be_first(b);
        assert_eq!(child_names(&t, root), vec!["b", "c", "a"]);

        // Already first → no-op
        t.be_first(a);
        // a is last, move to first
        assert_eq!(child_names(&t, root), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_be_last() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let a = t.create_child(root, "a");
        let _b = t.create_child(root, "b");
        let _c = t.create_child(root, "c");

        // Move a to end
        t.be_last(a);
        assert_eq!(child_names(&t, root), vec!["b", "c", "a"]);
    }

    #[test]
    fn test_be_prev_of() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let a = t.create_child(root, "a");
        let b = t.create_child(root, "b");
        let c = t.create_child(root, "c");

        // Move c before a → c, a, b
        t.be_prev_of(c, Some(a));
        assert_eq!(child_names(&t, root), vec!["c", "a", "b"]);

        // Move b before a → c, b, a
        t.be_prev_of(b, Some(a));
        assert_eq!(child_names(&t, root), vec!["c", "b", "a"]);

        // be_prev_of with None → be_last
        t.be_prev_of(c, None);
        assert_eq!(child_names(&t, root), vec!["b", "a", "c"]);
    }

    #[test]
    fn test_be_next_of() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let a = t.create_child(root, "a");
        let _b = t.create_child(root, "b");
        let c = t.create_child(root, "c");

        // Move a after c → b, c, a
        t.be_next_of(a, Some(c));
        assert_eq!(child_names(&t, root), vec!["b", "c", "a"]);

        // be_next_of with None → be_first
        t.be_next_of(a, None);
        assert_eq!(child_names(&t, root), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_be_prev_of_no_op_cases() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let a = t.create_child(root, "a");
        let b = t.create_child(root, "b");

        // Same panel → no-op
        t.be_prev_of(a, Some(a));
        assert_eq!(child_names(&t, root), vec!["a", "b"]);

        // Already before sibling → no-op
        t.be_prev_of(a, Some(b));
        assert_eq!(child_names(&t, root), vec!["a", "b"]);
    }

    #[test]
    fn test_be_next_of_no_op_cases() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let a = t.create_child(root, "a");
        let b = t.create_child(root, "b");

        // Same panel → no-op
        t.be_next_of(b, Some(b));
        assert_eq!(child_names(&t, root), vec!["a", "b"]);

        // Already after sibling → no-op
        t.be_next_of(b, Some(a));
        assert_eq!(child_names(&t, root), vec!["a", "b"]);
    }

    #[test]
    fn test_sort_children() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let _c = t.create_child(root, "c");
        let _a = t.create_child(root, "a");
        let _b = t.create_child(root, "b");

        // Build a name map before sorting so the closure doesn't borrow t
        let names: HashMap<PanelId, String> = t
            .children(root)
            .map(|id| (id, t.name(id).unwrap().to_string()))
            .collect();
        t.sort_children(root, |a_id, b_id| names[&a_id].cmp(&names[&b_id]));
        assert_eq!(child_names(&t, root), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_sort_children_no_change() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let _a = t.create_child(root, "a");
        let _b = t.create_child(root, "b");

        // Clear pending notices before sort
        t.deliver_notices();

        // Build name map
        let names: HashMap<PanelId, String> = t
            .children(root)
            .map(|id| (id, t.name(id).unwrap().to_string()))
            .collect();

        // Already sorted -> should not set CHILDREN_CHANGED
        t.sort_children(root, |a_id, b_id| names[&a_id].cmp(&names[&b_id]));
        assert!(!t
            .pending_notices(root)
            .contains(NoticeFlags::CHILDREN_CHANGED));
    }

    #[test]
    fn test_sort_children_reverse() {
        let mut t = PanelTree::new();
        let root = t.create_root("root");
        let _a = t.create_child(root, "a");
        let _b = t.create_child(root, "b");
        let _c = t.create_child(root, "c");

        // Build name map
        let names: HashMap<PanelId, String> = t
            .children(root)
            .map(|id| (id, t.name(id).unwrap().to_string()))
            .collect();

        // Sort in reverse
        t.sort_children(root, |a_id, b_id| names[&b_id].cmp(&names[&a_id]));
        assert_eq!(child_names(&t, root), vec!["c", "b", "a"]);

        // Verify reverse iteration also works
        let rev_names: Vec<String> = t
            .children_rev(root)
            .map(|id| t.name(id).unwrap().to_string())
            .collect();
        assert_eq!(rev_names, vec!["a", "b", "c"]);
    }
}
