//! V1's three conditional blocks of `.chat-panel-root` (`RealAmadeusApp.tsx` L1873–1976): the
//! dialogue box, the input bar, and the MINI GAME / ACHIEVEMENTS triggers. Only one of them shows
//! at a time, and they are swapped the way React swapped them, so native Tab focus lands where it
//! did in V1 (KEY-009). OWNED BY: chat engineer.
//!
//! Chrome keeps the *sequential focus navigation starting point* as a live DOM range. When the
//! focused chat input (or a clicked button) is removed with its block, the range collapses to the
//! block's index in `.chat-panel-root`, and the next Tab that is not prevented (an overlay is open)
//! moves focus forward from there. React commits deletions before insertions and inserts the new
//! block at that same index, so the point ends up *before* the new block: Tab focuses the chat
//! input (or the dialogue's advance hitbox). A Leptos branch swap inserts the new nodes before the
//! old ones and then removes the old ones, which leaves the point *after* the new block: Tab
//! focused MINI GAME, and the next Enter opened the mini game behind the overlay.
//!
//! ChatPanel therefore renders the three blocks as one branch ([`ChatBlock`]), so that every change
//! is one swap, and calls [`after_swap`], which detaches the new block and puts it back in the same
//! place before anything else can run ([`reinsert_in_place`]). Removing the block moves a point that
//! sits right after it to its start, and inserting it there keeps that point in front of it. A
//! point anywhere else does not move (none can be inside the block yet). The net effect is
//! React's; the unit tests check it against a model of both renderers for every sequence of up to
//! four block changes. No focus is moved and no key is prevented: Tab stays native, as in V1.
//!
//! The block is read from the DOM ([`block_after`]): what lies between the Live2D wrapper
//! (`.live2d-shift`) and the mini game's root (`.mini-game-panel`), from the first element on. No
//! list of the branches' elements has to be kept in step with the views, so an element added to
//! a branch is re-inserted with the others.

use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use super::controller::ChatState;

/// The class of `MiniGamePanel`'s root (`app/panels/minigame.rs`), the node after the blocks.
pub const MINI_GAME_ROOT_CLASS: &str = "mini-game-panel";

/// The block `.chat-panel-root` shows (V1's `chatState` / `miniGameOpen` conditions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatBlock {
    /// `chatState !== "inputReady"`: `.dialogue-panel`.
    Dialogue,
    /// `inputReady && !miniGameOpen`: `.input-panel`, MINI GAME and ACHIEVEMENTS.
    Input,
    /// `inputReady && miniGameOpen`: none of them (the mini game is open).
    Hidden,
}

/// V1's conditions: the dialogue whenever the state is not `inputReady`, otherwise the input bar
/// and its triggers unless the mini game is open.
pub fn chat_block(state: ChatState, mini_game_open: bool) -> ChatBlock {
    if state != ChatState::InputReady {
        ChatBlock::Dialogue
    } else if mini_game_open {
        ChatBlock::Hidden
    } else {
        ChatBlock::Input
    }
}

/// The child-list operations [`reinsert_in_place`] needs: [`Dom`] in the app, a model in tests.
pub trait ChildList {
    /// A node handle.
    type Node: Clone + PartialEq;
    /// `node.parentNode`.
    fn parent(&self, node: &Self::Node) -> Option<Self::Node>;
    /// `node.nextSibling`.
    fn next_sibling(&self, node: &Self::Node) -> Option<Self::Node>;
    /// `node.nodeType === Node.ELEMENT_NODE`.
    fn is_element(&self, node: &Self::Node) -> bool;
    /// `parent.removeChild(node)`.
    fn remove(&mut self, parent: &Self::Node, node: &Self::Node);
    /// `parent.insertBefore(node, reference)` (append when `reference` is `None`).
    fn insert_before(
        &mut self,
        parent: &Self::Node,
        node: &Self::Node,
        reference: Option<&Self::Node>,
    );
}

/// Detaches `block`, a run of adjacent siblings, and inserts it again before the same next
/// sibling. The nodes keep their place and identity; only live ranges move: a range collapsed in
/// the parent between two of the nodes or right after the last one ends up right before the
/// first, and so does a range inside the nodes (as with any removal; hence this runs on freshly
/// mounted nodes only). Any other range is unchanged. Returns `false`, without touching
/// anything, when `block` is empty or not one run of adjacent siblings.
pub fn reinsert_in_place<L: ChildList>(list: &mut L, block: &[L::Node]) -> bool {
    let (Some(first), Some(last)) = (block.first(), block.last()) else {
        return false;
    };
    let Some(parent) = list.parent(first) else {
        return false;
    };
    if block
        .windows(2)
        .any(|pair| list.next_sibling(&pair[0]).as_ref() != Some(&pair[1]))
    {
        return false;
    }
    let reference = list.next_sibling(last);
    for node in block {
        list.remove(&parent, node);
    }
    for node in block {
        list.insert_before(&parent, node, reference.as_ref());
    }
    true
}

/// The block that follows `start` (the Live2D wrapper): the siblings after it up to the first
/// one `is_end` accepts (the mini game's root), from the first element on. Comments before that
/// element are skipped, so the hidden branch (only a placeholder comment) gives an empty run.
/// Comments after it stay in the run: they keep it one run of adjacent siblings, and a point
/// right after a trailing comment is moved as well. `None` when no sibling after `start` is
/// the end.
pub fn block_after<L: ChildList>(
    list: &L,
    start: &L::Node,
    is_end: impl Fn(&L::Node) -> bool,
) -> Option<Vec<L::Node>> {
    let mut run = Vec::new();
    let mut next = list.next_sibling(start);
    while let Some(node) = next {
        if is_end(&node) {
            return Some(run);
        }
        next = list.next_sibling(&node);
        if !run.is_empty() || list.is_element(&node) {
            run.push(node);
        }
    }
    None
}

/// [`ChildList`] over the live DOM.
pub struct Dom;

impl ChildList for Dom {
    type Node = web_sys::Node;

    fn parent(&self, node: &web_sys::Node) -> Option<web_sys::Node> {
        node.parent_node()
    }

    fn next_sibling(&self, node: &web_sys::Node) -> Option<web_sys::Node> {
        node.next_sibling()
    }

    fn is_element(&self, node: &web_sys::Node) -> bool {
        node.node_type() == web_sys::Node::ELEMENT_NODE
    }

    fn remove(&mut self, parent: &web_sys::Node, node: &web_sys::Node) {
        let _ = parent.remove_child(node);
    }

    fn insert_before(
        &mut self,
        parent: &web_sys::Node,
        node: &web_sys::Node,
        reference: Option<&web_sys::Node>,
    ) {
        let _ = parent.insert_before(node, reference);
    }
}

/// Whether `node` is `MiniGamePanel`'s root element.
fn is_mini_game_root(node: &web_sys::Node) -> bool {
    node.dyn_ref::<web_sys::Element>()
        .is_some_and(|element| element.class_list().contains(MINI_GAME_ROOT_CLASS))
}

/// Called while ChatPanel renders a block swap. Once the swap has been applied (a task on the
/// next microtask, before any input event or paint), [`reinsert_in_place`] runs on the block
/// shown at that moment, read from the DOM after `start` (the Live2D wrapper, `None` once the
/// panel is gone) by [`block_after`]: its top-level elements, none for [`ChatBlock::Hidden`],
/// whose placeholder comment Chrome's focus navigation skips anyway. Nothing moves while focus is
/// inside the block: the focused element is the starting point then, and moving it would blur
/// it. A debug build asserts that the mini game's root still follows the blocks.
pub fn after_swap(start: impl FnOnce() -> Option<web_sys::Node> + 'static) {
    spawn_local(async move {
        let Some(start) = start() else {
            return;
        };
        let block = block_after(&Dom, &start, is_mini_game_root);
        debug_assert!(
            block.is_some(),
            "KEY-009: `.{MINI_GAME_ROOT_CLASS}` no longer follows the chat blocks"
        );
        let Some(block) = block else {
            return;
        };
        let focused: Option<web_sys::Node> = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.active_element())
            .map(Into::into);
        if block.iter().any(|node| node.contains(focused.as_ref())) {
            return;
        }
        reinsert_in_place(&mut Dom, &block);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_follow_v1_conditions() {
        use ChatState::{InputReady, Typing, WaitForAdvance, WaitingApi};
        for state in [WaitingApi, Typing, WaitForAdvance] {
            assert_eq!(chat_block(state, false), ChatBlock::Dialogue);
            // The dialogue does not depend on the mini game.
            assert_eq!(chat_block(state, true), ChatBlock::Dialogue);
        }
        assert_eq!(chat_block(InputReady, false), ChatBlock::Input);
        assert_eq!(chat_block(InputReady, true), ChatBlock::Hidden);
    }

    /// `.chat-panel-root` with one live range, updated by the DOM's removing / inserting steps:
    /// a range inside a removed node collapses to the node's index in its parent, and a collapsed
    /// range in the parent shifts when a node before it is removed or inserted.
    struct Model {
        /// Node labels by id; id 0 is `.chat-panel-root`, `#` marks a comment.
        labels: Vec<&'static str>,
        children: Vec<usize>,
        point: Point,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Point {
        /// In the contents of a node (a clicked or focused element).
        Inside(usize),
        /// Collapsed at an offset in `.chat-panel-root`.
        At(usize),
    }

    const ROOT: usize = 0;

    impl Model {
        fn new() -> Self {
            Self {
                labels: vec!["root"],
                children: Vec::new(),
                point: Point::At(0),
            }
        }

        fn create(&mut self, label: &'static str) -> usize {
            self.labels.push(label);
            self.labels.len() - 1
        }

        fn index(&self, node: usize) -> Option<usize> {
            self.children.iter().position(|&child| child == node)
        }

        /// Where the next Tab looks first: the node that holds the point, or the elements from
        /// the point on (Chrome starts at the element before the point and skips comments).
        fn tab_order(&self) -> String {
            match self.point {
                Point::Inside(node) => format!("inside {}", self.labels[node]),
                Point::At(offset) => self.children[offset..]
                    .iter()
                    .filter(|node| self.is_element(node))
                    .map(|&node| self.labels[node])
                    .collect::<Vec<_>>()
                    .join(" "),
            }
        }
    }

    impl ChildList for Model {
        type Node = usize;

        fn parent(&self, node: &usize) -> Option<usize> {
            self.index(*node).map(|_| ROOT)
        }

        fn next_sibling(&self, node: &usize) -> Option<usize> {
            let index = self.index(*node)?;
            self.children.get(index + 1).copied()
        }

        fn is_element(&self, node: &usize) -> bool {
            self.labels[*node] != "#"
        }

        fn remove(&mut self, parent: &usize, node: &usize) {
            assert_eq!(*parent, ROOT);
            let index = self
                .index(*node)
                .expect("removing a node that is not a child");
            self.children.remove(index);
            self.point = match self.point {
                Point::Inside(inside) if inside == *node => Point::At(index),
                Point::At(offset) if offset > index => Point::At(offset - 1),
                point => point,
            };
        }

        fn insert_before(&mut self, parent: &usize, node: &usize, reference: Option<&usize>) {
            assert_eq!(*parent, ROOT);
            assert!(
                self.index(*node).is_none(),
                "inserting a node that is a child"
            );
            let index = reference.map_or(self.children.len(), |reference| {
                self.index(*reference).expect("reference is not a child")
            });
            self.children.insert(index, *node);
            if let Point::At(offset) = self.point {
                if offset > index {
                    self.point = Point::At(offset + 1);
                }
            }
        }
    }

    /// How a block is put on screen.
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Renderer {
        /// V1: deletions first, then the new block before the next sibling (the mini game).
        React,
        /// Leptos branch swap: new nodes before the old first node, then the old nodes removed.
        Leptos,
        /// The Leptos swap followed by [`after_swap`]: [`reinsert_in_place`] of the
        /// [`block_after`] the Live2D wrapper (ChatPanel).
        LeptosReinserted,
    }

    struct Panel {
        renderer: Renderer,
        model: Model,
        block: ChatBlock,
        nodes: Vec<usize>,
        live2d: usize,
        mini_game: usize,
    }

    impl Panel {
        /// `.chat-panel-root` showing `block`: Live2D, the block, the mini game, the toast.
        fn new(renderer: Renderer, block: ChatBlock) -> Self {
            let mut model = Model::new();
            let live2d = model.create("L");
            model.children.push(live2d);
            let mut panel = Self {
                renderer,
                model,
                block,
                nodes: Vec::new(),
                live2d,
                mini_game: 0,
            };
            panel.nodes = panel.build(block);
            panel.model.children.extend(panel.nodes.iter().copied());
            panel.mini_game = panel.model.create("MGP");
            let toast = panel.model.create("T");
            panel.model.children.extend([panel.mini_game, toast]);
            panel
        }

        /// Fresh nodes for `block`; Leptos renders a hidden branch as one comment.
        fn build(&mut self, block: ChatBlock) -> Vec<usize> {
            match block {
                ChatBlock::Dialogue => vec![self.model.create("D")],
                ChatBlock::Input => ["IP", "MB", "AB"]
                    .iter()
                    .map(|label| self.model.create(label))
                    .collect(),
                ChatBlock::Hidden if self.renderer == Renderer::React => Vec::new(),
                ChatBlock::Hidden => vec![self.model.create("#")],
            }
        }

        fn node(&self, label: &str) -> usize {
            *self
                .nodes
                .iter()
                .find(|&&node| self.model.labels[node] == label)
                .unwrap_or_else(|| panic!("{label} is not shown"))
        }

        fn show(&mut self, block: ChatBlock) {
            if block == self.block {
                return;
            }
            let old = std::mem::take(&mut self.nodes);
            let new = self.build(block);
            match self.renderer {
                Renderer::React => {
                    for node in &old {
                        self.model.remove(&ROOT, node);
                    }
                    for node in &new {
                        self.model.insert_before(&ROOT, node, Some(&self.mini_game));
                    }
                }
                Renderer::Leptos | Renderer::LeptosReinserted => {
                    for node in &new {
                        self.model.insert_before(&ROOT, node, Some(&old[0]));
                    }
                    for node in &old {
                        self.model.remove(&ROOT, node);
                    }
                    if self.renderer == Renderer::LeptosReinserted {
                        let mini_game = self.mini_game;
                        let run = block_after(&self.model, &self.live2d, |&node| node == mini_game)
                            .expect("the mini game follows the blocks");
                        reinsert_in_place(&mut self.model, &run);
                    }
                }
            }
            self.nodes = new;
            self.block = block;
        }
    }

    const ALL: [ChatBlock; 3] = [ChatBlock::Dialogue, ChatBlock::Input, ChatBlock::Hidden];

    /// Every sequence of 1–4 changes of block.
    fn sequences(from: ChatBlock) -> Vec<Vec<ChatBlock>> {
        let mut out: Vec<Vec<ChatBlock>> = Vec::new();
        let mut frontier = vec![vec![from]];
        for _ in 0..4 {
            let mut next = Vec::new();
            for path in frontier {
                for block in ALL {
                    if Some(&block) != path.last() {
                        let mut longer = path.clone();
                        longer.push(block);
                        out.push(longer[1..].to_vec());
                        next.push(longer);
                    }
                }
            }
            frontier = next;
        }
        out
    }

    /// Starting points: inside each element of the shown block, or collapsed at any offset of
    /// the Leptos layout (comments included).
    fn starts(block: ChatBlock) -> Vec<(Option<&'static str>, usize)> {
        let labels: &[&str] = match block {
            ChatBlock::Dialogue => &["D"],
            ChatBlock::Input => &["IP", "MB", "AB"],
            ChatBlock::Hidden => &[],
        };
        let mut out: Vec<(Option<&'static str>, usize)> =
            labels.iter().map(|&label| (Some(label), 0)).collect();
        let len = Panel::new(Renderer::Leptos, block).model.children.len();
        out.extend((0..=len).map(|offset| (None, offset)));
        out
    }

    /// Shows `from`, places the starting point, applies `steps` and returns where Tab looks
    /// after each of them. A collapsed start is an offset in the Leptos layout; React (no
    /// comments) gets the offset with the same elements in front of it.
    fn run(
        renderer: Renderer,
        from: ChatBlock,
        start: (Option<&str>, usize),
        steps: &[ChatBlock],
    ) -> Vec<String> {
        let mut panel = Panel::new(renderer, from);
        panel.model.point = match start {
            (Some(label), _) => Point::Inside(panel.node(label)),
            (None, offset) if renderer != Renderer::React => Point::At(offset),
            (None, offset) => {
                let leptos = Panel::new(Renderer::Leptos, from);
                let elements = leptos.model.children[..offset]
                    .iter()
                    .filter(|node| leptos.model.is_element(node))
                    .count();
                Point::At(elements)
            }
        };
        steps
            .iter()
            .map(|&block| {
                panel.show(block);
                panel.model.tab_order()
            })
            .collect()
    }

    #[test]
    fn reinserted_swaps_leave_the_focus_start_where_react_does() {
        let mut cases = 0;
        for from in ALL {
            for start in starts(from) {
                for steps in sequences(from) {
                    let react = run(Renderer::React, from, start, &steps);
                    let leptos = run(Renderer::LeptosReinserted, from, start, &steps);
                    assert_eq!(
                        leptos, react,
                        "from {from:?}, start {start:?}, steps {steps:?}"
                    );
                    cases += 1;
                }
            }
        }
        // 21 starting points × 30 sequences.
        assert_eq!(cases, 630);
    }

    #[test]
    fn a_plain_leptos_swap_puts_the_focus_start_after_the_new_block() {
        // KEY-009: the focused input is submitted (dialogue), the reply ends (input again).
        let steps = [ChatBlock::Dialogue, ChatBlock::Input];
        let start = (Some("IP"), 0);
        let react = run(Renderer::React, ChatBlock::Input, start, &steps);
        assert_eq!(react, ["D MGP T", "IP MB AB MGP T"]);
        let plain = run(Renderer::Leptos, ChatBlock::Input, start, &steps);
        assert_eq!(plain, ["MGP T", "MGP T"]);
        assert_eq!(
            run(Renderer::LeptosReinserted, ChatBlock::Input, start, &steps),
            react
        );
        // MINI GAME clicked, the mini game closed with Escape: Tab goes to the input in V1.
        let steps = [ChatBlock::Hidden, ChatBlock::Input];
        let start = (Some("MB"), 0);
        let react = run(Renderer::React, ChatBlock::Input, start, &steps);
        assert_eq!(react, ["MGP T", "IP MB AB MGP T"]);
        assert_eq!(
            run(Renderer::Leptos, ChatBlock::Input, start, &steps)[1],
            "MGP T"
        );
        assert_eq!(
            run(Renderer::LeptosReinserted, ChatBlock::Input, start, &steps),
            react
        );
    }

    #[test]
    fn reinsert_keeps_order_and_moves_only_points_in_or_after_the_block() {
        let mut panel = Panel::new(Renderer::Leptos, ChatBlock::Input);
        let block = panel.nodes.clone();
        let before = panel.model.children.clone();
        // L IP MB AB MGP T: points between the block's nodes or right after it (2–4) move to
        // its start (1), the others stay.
        for offset in 0..=before.len() {
            panel.model.point = Point::At(offset);
            assert!(reinsert_in_place(&mut panel.model, &block));
            assert_eq!(panel.model.children, before);
            let expected = if (2..=4).contains(&offset) { 1 } else { offset };
            assert_eq!(panel.model.point, Point::At(expected), "offset {offset}");
        }
        // A point inside a node of the block collapses like on any removal; one inside another
        // node does not move.
        panel.model.point = Point::Inside(panel.node("MB"));
        assert!(reinsert_in_place(&mut panel.model, &block));
        assert_eq!(panel.model.point, Point::At(1));
        panel.model.point = Point::Inside(panel.mini_game);
        assert!(reinsert_in_place(&mut panel.model, &block));
        assert_eq!(panel.model.point, Point::Inside(panel.mini_game));
    }

    #[test]
    fn reinsert_refuses_what_is_not_one_run_of_siblings() {
        let mut panel = Panel::new(Renderer::Leptos, ChatBlock::Input);
        let before = panel.model.children.clone();
        panel.model.point = Point::At(4);
        let (input, achievements) = (panel.node("IP"), panel.node("AB"));
        assert!(!reinsert_in_place(&mut panel.model, &[]));
        assert!(!reinsert_in_place(&mut panel.model, &[input, achievements]));
        let detached = panel.model.create("D");
        assert!(!reinsert_in_place(&mut panel.model, &[detached]));
        assert_eq!(panel.model.children, before);
        assert_eq!(panel.model.point, Point::At(4));
    }

    #[test]
    fn the_block_is_read_between_the_live2d_wrapper_and_the_mini_game() {
        // ChatPanel's blocks: exactly their top-level elements, nothing for the hidden branch
        // (its placeholder comment is skipped), with either renderer's layout.
        for renderer in [Renderer::React, Renderer::Leptos] {
            for block in ALL {
                let panel = Panel::new(renderer, block);
                let mini_game = panel.mini_game;
                let run = block_after(&panel.model, &panel.live2d, |&node| node == mini_game);
                let elements: Vec<usize> = panel
                    .nodes
                    .iter()
                    .copied()
                    .filter(|node| panel.model.is_element(node))
                    .collect();
                assert_eq!(run, Some(elements), "{renderer:?} {block:?}");
            }
        }

        // No list to keep in step: an element added to a branch is part of the run, and so are
        // comments between or after its elements (a `<Show>` that is hidden), which keeps the
        // run adjacent and moves a point that the swap left after a trailing comment.
        let mut model = Model::new();
        let labels = ["L", "#", "IP", "#", "MB", "X", "#", "MGP", "T"];
        let nodes: Vec<usize> = labels.iter().map(|label| model.create(label)).collect();
        model.children = nodes.clone();
        let (live2d, mini_game) = (nodes[0], nodes[7]);
        let run = block_after(&model, &live2d, |&node| node == mini_game);
        assert_eq!(run.as_deref(), Some(&nodes[2..7]));
        model.point = Point::At(7);
        assert!(reinsert_in_place(&mut model, &nodes[2..7]));
        assert_eq!(model.children, nodes);
        assert_eq!(model.point, Point::At(2));
        assert_eq!(model.tab_order(), "IP MB X MGP T");

        // Nothing is re-inserted when the mini game's root does not follow the start.
        assert_eq!(
            block_after(&model, &mini_game, |&node| node == mini_game),
            None
        );
        assert_eq!(
            block_after(&model, &nodes[8], |&node| node == mini_game),
            None
        );
    }
}
