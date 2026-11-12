//! Find and replace in a [`code_editor`](crate::code_editor): the bar over the editor's top right
//! corner (a query, case, whole-word and regular-expression options, a count, next and previous,
//! replace and replace all), the match bands drawn behind the text, and the state a host can read
//! or drive, [`CodeFind`].
//!
//! The bar is built from the crate's own widgets while it is open and despawned when it closes, so
//! a closed editor owns none of it. The searching is [`crate::find_all`] and
//! [`crate::replace_edits`]; a replace all is one undo step.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::Pickable;
use bevy::picking::events::{PointerDrag, PointerMove, PointerPress};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ValueChange, observe};

use crate::code_buffer::{CodePos, display_col};
use crate::code_editor::{CodeEditor, CodeEditorValue, publish};
use crate::code_find::{FindOptions, find_all, replace_edits};
use crate::code_overlay::OverlayPart;
use crate::skin::Role;
use crate::widgets::{button, label, row, text_input, toggle};

/// What the player asked of the find bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindAction {
    Next,
    Previous,
    Replace,
    ReplaceAll,
    Close,
}

/// The find and replace state of a [`code_editor`](crate::code_editor): on every editor. Set `open`
/// and the bar shows; Cmd or Ctrl+F opens it (seeded with a one-line selection) and Cmd or Ctrl+Alt+F
/// opens it with the replace field. Read [`CodeFind::count`], [`CodeFind::current`] and
/// [`CodeFind::error`] for what the query found.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn search(mut finds: Query<&mut CodeFind>) {
///     for mut find in &mut finds {
///         find.open = true;
///         find.query = "TODO".into();
///         find.options.whole_word = true;
///     }
/// }
/// ```
#[derive(Component, Clone, Debug, Default)]
pub struct CodeFind {
    /// Whether the bar is open.
    pub open: bool,
    /// What is looked for.
    pub query: String,
    /// What a replace puts in; with a regular expression it may name groups (`$1`).
    pub replacement: String,
    /// Case, whole word and regular expression.
    pub options: FindOptions,
    /// Whether the bar shows the replace field and buttons.
    pub show_replace: bool,
    pub(crate) matches: Vec<(CodePos, CodePos)>,
    pub(crate) current: Option<usize>,
    pub(crate) error: Option<String>,
    pub(crate) key: Option<(u64, String, FindOptions)>,
    pub(crate) panel: Option<Entity>,
    pub(crate) panel_replace: bool,
    pub(crate) focus_pending: bool,
    pub(crate) actions: Vec<FindAction>,
    pub(crate) bands: Vec<Entity>,
}

impl CodeFind {
    /// How many matches the query has in the text (0 while the bar is closed).
    pub fn count(&self) -> usize {
        self.matches.len()
    }

    /// The match the bar is on, from 0.
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    /// Why the query could not be searched (a regular expression that does not compile).
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Opens the bar, with the replace field when `replace`, seeded with `seed` when it is a word
    /// or phrase on one line.
    pub(crate) fn show(&mut self, replace: bool, seed: &str) {
        self.open = true;
        self.show_replace = replace;
        if !seed.is_empty() && !seed.contains('\n') {
            self.query = seed.to_owned();
        }
        self.focus_pending = true;
    }
}

/// The count's words, so the system finds the label to write.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct FindCount(pub(crate) Entity);

/// The query's row, so focus can go to it when the bar opens.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct FindField(pub(crate) Entity);

fn panel(root: Entity, find: &CodeFind) -> impl Bundle {
    let query = find.query.clone();
    let replacement = find.replacement.clone();
    let options = find.options;
    let with_replace = find.show_replace;
    let act = move |action: FindAction| {
        observe(move |_: On<Activate>, mut finds: Query<&mut CodeFind>| {
            if let Ok(mut find) = finds.get_mut(root) {
                find.actions.push(action);
            }
        })
    };
    let option = move |name: &'static str, on: bool, set: fn(&mut FindOptions, bool)| {
        (
            toggle(name, on),
            observe(
                move |change: On<ValueChange<bool>>, mut finds: Query<&mut CodeFind>| {
                    if let Ok(mut find) = finds.get_mut(root) {
                        set(&mut find.options, change.value);
                    }
                },
            ),
        )
    };
    (
        Node::default(),
        Role::CodeFind,
        OverlayPart,
        // A press, drag or move on the bar is the bar's: the editor under it does not take it.
        observe(|mut press: On<PointerPress>| press.propagate(false)),
        observe(|mut drag: On<PointerDrag>| drag.propagate(false)),
        observe(|mut moved: On<PointerMove>| moved.propagate(false)),
        ChildOf(root),
        Children::spawn(bevy::ecs::spawn::SpawnWith(
            move |parts: &mut bevy::ecs::relationship::RelatedSpawner<ChildOf>| {
                parts.spawn((
                    text_input("Find", query.clone()),
                    FindField(root),
                    observe(
                        move |change: On<ValueChange<String>>, mut finds: Query<&mut CodeFind>| {
                            if let Ok(mut find) = finds.get_mut(root) {
                                find.query.clone_from(&change.value);
                                if change.is_final {
                                    find.actions.push(FindAction::Next);
                                }
                            }
                        },
                    ),
                ));
                if with_replace {
                    parts.spawn((
                        text_input("Replace", replacement.clone()),
                        observe(
                            move |change: On<ValueChange<String>>,
                                  mut finds: Query<&mut CodeFind>| {
                                if let Ok(mut find) = finds.get_mut(root) {
                                    find.replacement.clone_from(&change.value);
                                }
                            },
                        ),
                    ));
                }
                parts.spawn(option("Match case", options.case_sensitive, |o, v| {
                    o.case_sensitive = v;
                }));
                parts.spawn(option("Whole word", options.whole_word, |o, v| {
                    o.whole_word = v
                }));
                parts.spawn(option("Regular expression", options.regex, |o, v| {
                    o.regex = v
                }));
                parts.spawn((label(""), FindCount(root)));
                parts.spawn((
                    row(1.0),
                    children![
                        (button("Previous"), act(FindAction::Previous)),
                        (button("Next"), act(FindAction::Next)),
                    ],
                ));
                if with_replace {
                    parts.spawn((
                        row(1.0),
                        children![
                            (button("Replace"), act(FindAction::Replace)),
                            (button("Replace all"), act(FindAction::ReplaceAll)),
                        ],
                    ));
                }
                parts.spawn((button("Close"), act(FindAction::Close)));
            },
        )),
    )
}

/// The find bar, its matches and its actions, once a frame after the editor has laid out.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_code_find(
    mut editors: Query<(Entity, &mut CodeEditor, &mut CodeEditorValue, &mut CodeFind)>,
    counts: Query<(Entity, &FindCount)>,
    fields: Query<(Entity, &FindField)>,
    mut texts: Query<&mut Text>,
    mut bands: Query<(&mut Node, &mut Role), With<OverlayPart>>,
    mut focus: ResMut<InputFocus>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
) {
    let now = time.map_or(0.0, |time| time.elapsed_secs_f64());
    for (root, mut editor_mut, mut value, mut find_mut) in &mut editors {
        let editor = &mut *editor_mut;
        let find = &mut *find_mut;
        if !find.open {
            if let Some(panel) = find.panel.take() {
                commands.entity(panel).despawn();
                find.matches.clear();
                find.current = None;
                find.key = None;
            }
            for &band in &find.bands {
                if let Ok((mut node, _)) = bands.get_mut(band)
                    && node.display != Display::None
                {
                    node.display = Display::None;
                }
            }
            find.actions.clear();
            continue;
        }
        // The bar follows `show_replace`: it is built again when that changes.
        if find.panel_replace != find.show_replace
            && let Some(panel) = find.panel.take()
        {
            commands.entity(panel).despawn();
        }
        if find.panel.is_none() {
            find.panel = Some(commands.spawn(panel(root, find)).id());
            find.panel_replace = find.show_replace;
        }
        if find.focus_pending
            && let Some((field, _)) = fields.iter().find(|(_, f)| f.0 == root)
        {
            find.focus_pending = false;
            focus.set(field, FocusCause::Navigated);
        }

        // What the query finds in the text as it stands.
        let key = (editor.buffer.revision(), find.query.clone(), find.options);
        if find.key.as_ref() != Some(&key) {
            match find_all(&editor.buffer, &find.query, find.options) {
                Ok(found) => {
                    find.matches = found;
                    find.error = None;
                }
                Err(error) => {
                    find.matches.clear();
                    find.error = Some(error.to_string());
                }
            }
            let caret = editor.buffer.caret();
            find.current = if find.matches.is_empty() {
                None
            } else {
                Some(
                    find.matches
                        .iter()
                        .position(|(start, _)| *start >= caret)
                        .unwrap_or(0),
                )
            };
            find.key = Some(key);
        }

        // What the player asked.
        let actions = std::mem::take(&mut find.actions);
        for action in actions {
            let n = find.matches.len();
            match action {
                FindAction::Next | FindAction::Previous if n > 0 => {
                    let at = find.current.unwrap_or(0);
                    let next = if action == FindAction::Next {
                        (at + 1) % n
                    } else {
                        (at + n - 1) % n
                    };
                    // The first Next lands on the match already current when nothing is selected on it.
                    let on_current = {
                        let (start, end) = find.matches[at];
                        editor.buffer.selection().start() == start
                            && editor.buffer.selection().end() == end
                    };
                    let target = if on_current || action == FindAction::Previous {
                        next
                    } else {
                        at
                    };
                    find.current = Some(target);
                    let (start, end) = find.matches[target];
                    editor.select(start, end);
                }
                FindAction::Replace if find.current.is_some() => {
                    let at = find.current.unwrap_or(0);
                    if let Ok(edits) =
                        replace_edits(&editor.buffer, &find.query, &find.replacement, find.options)
                        && let Some(edit) = edits.get(at)
                    {
                        match editor
                            .buffer
                            .replace_ranges(std::slice::from_ref(edit), now)
                        {
                            Ok(_) => {
                                editor.touched();
                                publish(root, editor, &mut value, false, &mut commands);
                                find.key = None;
                            }
                            Err(why) => find.error = Some(why.to_string()),
                        }
                    }
                }
                FindAction::ReplaceAll if n > 0 => {
                    if let Ok(edits) =
                        replace_edits(&editor.buffer, &find.query, &find.replacement, find.options)
                    {
                        match editor.buffer.replace_ranges(&edits, now) {
                            Ok(_) => {
                                editor.touched();
                                publish(root, editor, &mut value, false, &mut commands);
                                find.key = None;
                            }
                            Err(why) => find.error = Some(why.to_string()),
                        }
                    }
                }
                FindAction::Close => {
                    find.open = false;
                    focus.set(root, FocusCause::Navigated);
                }
                _ => {}
            }
        }

        // The count's words.
        let words = if let Some(error) = &find.error {
            error.clone()
        } else if find.query.is_empty() {
            String::new()
        } else if let Some(at) = find.current {
            format!("{} of {}", at + 1, find.matches.len())
        } else {
            "No results".to_owned()
        };
        for (entity, count) in &counts {
            if count.0 == root
                && let Ok(mut text) = texts.get_mut(entity)
                && text.0 != words
            {
                text.0 = words.clone();
            }
        }

        // The matches in view, a band each.
        let view = editor.view;
        let Some(area) = editor.parts.area else {
            continue;
        };
        let mut segments: Vec<(usize, usize, usize, bool)> = Vec::new();
        if view.ready {
            let first_line = editor.fold.line_at(view.first_row);
            let at = find
                .matches
                .partition_point(|(_, end)| end.line < first_line);
            let tab = editor.buffer.tab_width();
            for (index, (start, end)) in find.matches.iter().enumerate().skip(at) {
                if segments.len() > view.needed + 4 {
                    break;
                }
                let row = editor.fold.row_of(start.line);
                if !(view.first_row..view.first_row + view.needed).contains(&row)
                    || editor.fold.is_hidden(start.line)
                {
                    if start.line > first_line + view.needed + 4 + editor.fold.hidden_lines() {
                        break;
                    }
                    continue;
                }
                let text = editor.buffer.line(start.line);
                let from = display_col(text, start.col, tab);
                let to = display_col(text, end.col.min(text.len()), tab).max(from + 1);
                segments.push((row, from, to, find.current == Some(index)));
            }
        }
        while find.bands.len() < segments.len() {
            let band = commands
                .spawn((
                    Node::default(),
                    Role::CodeFindMatch,
                    Pickable::IGNORE,
                    OverlayPart,
                    ChildOf(area),
                ))
                .id();
            find.bands.push(band);
        }
        for (slot, &band) in find.bands.iter().enumerate() {
            let Ok((mut node, mut role)) = bands.get_mut(band) else {
                continue;
            };
            match segments.get(slot) {
                Some(&(row, from, to, lit)) => {
                    if node.display != Display::Flex {
                        node.display = Display::Flex;
                    }
                    let left = Val::Px(view.pad + from as f32 * view.advance - view.shown.x);
                    let width = Val::Px((to - from) as f32 * view.advance);
                    let top = Val::Px(row as f32 * view.line_px - view.shown.y);
                    if node.left != left {
                        node.left = left;
                    }
                    if node.width != width {
                        node.width = width;
                    }
                    if node.top != top {
                        node.top = top;
                    }
                    let wanted = if lit {
                        Role::CodeFindMatchLit
                    } else {
                        Role::CodeFindMatch
                    };
                    if *role != wanted {
                        *role = wanted;
                    }
                }
                None => {
                    if node.display != Display::None {
                        node.display = Display::None;
                    }
                }
            }
        }
    }
}
