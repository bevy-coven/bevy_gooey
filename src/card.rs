//! The card grid: what keeps a [`crate::card_grid`]'s cursor (the one tab stop) and its selection
//! true, and the events a game hears when a card is chosen. Its look is [`crate::apply_skin`]'s.

use bevy::ecs::relationship::RelatedSpawner;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{Justify, LineBreak};
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{InteractionDisabled, Selected};
use bevy::ui_widgets::{Activate, ValueChange, observe};

use crate::skin::Role;

/// One card of a [`crate::card_grid`]: a picture above a name and a few quiet lines ("Opened 2
/// hours ago", "13 pieces"). A card with no picture draws a placeholder of the same size, so a row
/// of cards stays even; a disabled card is dimmed and cannot be chosen, but focus still reaches it
/// so the player can read it.
///
/// ```
/// # use bevy_gooey::*;
/// let card = Card::new("Harbour").line("~/maps/harbour").line("Opened 2 hours ago");
/// assert_eq!(card.lines.len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
pub struct Card {
    /// The card's name, in the body's size under the picture; a screen reader's name for it.
    pub name: String,
    /// The picture, drawn as it is (not tinted) across the card's width at 16:9.
    pub image: Option<Handle<Image>>,
    /// The quiet lines under the name, in the skin's small size; a screen reader's value for it.
    pub lines: Vec<String>,
    /// A dimmed card: `InteractionDisabled`, so nothing chooses it.
    pub disabled: bool,
}

impl Card {
    /// A card called `name`, with no picture and no lines.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            image: None,
            lines: Vec::new(),
            disabled: false,
        }
    }

    /// The card's picture.
    pub fn image(mut self, image: Handle<Image>) -> Self {
        self.image = Some(image);
        self
    }

    /// One more quiet line under the name.
    pub fn line(mut self, text: impl Into<String>) -> Self {
        self.lines.push(text.into());
        self
    }

    /// A dimmed card nothing chooses.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// The root of a [`crate::card_grid`]. Its state is [`CardCursor`] (the tab stop) and
/// [`CardSelection`] (the chosen card).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[require(CardCursor, CardSelection)]
pub struct CardGrid;

/// The card Tab enters a [`crate::card_grid`] on: the one the player last had focus on. It is the
/// grid's one tab stop (every other card is reached by the arrows). Focus moving to a card writes
/// it and sends one non-final [`ValueChange<usize>`] on the grid; write it yourself to move the
/// stop from code (no event is sent for that). A value past the last card is pulled back to it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct CardCursor(pub usize);

/// The chosen card of a [`crate::card_grid`], drawn lit (Bevy's `Selected` on it). Enter, Space,
/// the pad's South and a click on an enabled card choose it and send one final
/// [`ValueChange<usize>`] on the grid; write it yourself to choose from code (no event). A value
/// past the last card chooses none.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct CardSelection(pub Option<usize>);

/// Where a card is: its grid and its index there. On every card the grid draws; put a
/// [`crate::ContextMenu`] on the card entity for a menu of its own.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct CardRef {
    /// The [`CardGrid`].
    pub grid: Entity,
    /// Which card of it, from 0 in the order given.
    pub index: usize,
}

/// Enter, Space, the pad's South and a click (Bevy's `Button` makes each an [`Activate`]) choose a
/// card: it is the grid's selection and one final [`ValueChange`] goes to the grid.
fn card_on_activate(
    event: On<Activate>,
    cards: Query<&CardRef, Without<InteractionDisabled>>,
    mut grids: Query<&mut CardSelection>,
    mut commands: Commands,
) {
    let Ok(card) = cards.get(event.entity) else {
        return;
    };
    if let Ok(mut selection) = grids.get_mut(card.grid) {
        selection.set_if_neq(CardSelection(Some(card.index)));
    }
    commands.trigger(ValueChange {
        source: card.grid,
        value: card.index,
        is_final: true,
    });
}

/// The cards of a grid, spawned under it with their [`CardRef`]s.
pub(crate) fn spawn_cards(cards: Vec<Card>) -> impl FnOnce(&mut RelatedSpawner<ChildOf>) {
    move |spawner: &mut RelatedSpawner<ChildOf>| {
        let grid = spawner.target_entity();
        for (index, card) in cards.into_iter().enumerate() {
            let mut cell = spawner.spawn((
                Node::default(),
                bevy::ui_widgets::Button,
                Role::CardCell,
                Hovered::default(),
                TabIndex(if index == 0 { 0 } else { -1 }),
                AutoDirectionalNavigation::default(),
                CardRef { grid, index },
                observe(card_on_activate),
            ));
            if card.disabled {
                cell.insert(InteractionDisabled);
            }
            cell.with_children(|parts| {
                let mut picture = parts.spawn((Node::default(), Role::CardImage, Pickable::IGNORE));
                if let Some(image) = card.image {
                    picture.insert(ImageNode::new(image));
                }
                parts.spawn((
                    Text::new(card.name),
                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                    Role::CardName,
                    Pickable::IGNORE,
                ));
                for line in card.lines {
                    parts.spawn((
                        Text::new(line),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Role::CardLine,
                        Pickable::IGNORE,
                    ));
                }
            });
        }
    }
}

/// Keeps every [`crate::card_grid`]'s tab stop, cursor and lit card true: exactly the cursor card
/// has `TabIndex(0)` and the others `TabIndex(-1)` (Bevy's Tab skips a negative index, so the grid
/// is one stop); focus moving onto a card of the grid moves the cursor there with one non-final
/// [`ValueChange<usize>`]; the [`CardSelection`]'s card wears Bevy's `Selected`. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`]; writes only a difference.
pub fn update_card_grids(
    focus: Res<InputFocus>,
    mut grids: Query<(Entity, &mut CardCursor, &mut CardSelection, &Children), With<CardGrid>>,
    mut cards: Query<(Entity, &CardRef, &mut TabIndex, Has<Selected>)>,
    mut commands: Commands,
) {
    let focused = focus.get();
    for (grid, mut cursor, mut selection, children) in &mut grids {
        let count = children.iter().filter(|&c| cards.contains(c)).count();
        if count == 0 {
            continue;
        }
        // Focus on one of its cards is where the player is: the cursor follows it.
        if focus.is_changed()
            && let Some(card) = focused.and_then(|f| cards.get(f).ok()).map(|(_, r, ..)| *r)
            && card.grid == grid
            && card.index != cursor.0
        {
            cursor.0 = card.index;
            commands.trigger(ValueChange {
                source: grid,
                value: card.index,
                is_final: false,
            });
        }
        if cursor.0 >= count {
            cursor.0 = count - 1;
        }
        if selection.0.is_some_and(|index| index >= count) {
            selection.0 = None;
        }
        let stop = cursor.0;
        let lit = selection.0;
        let mut found = cards.iter_many_mut(children.iter());
        while let Some(item) = found.fetch_next() {
            let Ok((entity, card, mut tab, selected)) = item else {
                continue;
            };
            let want = if card.index == stop { 0 } else { -1 };
            if tab.0 != want {
                tab.0 = want;
            }
            let wants = lit == Some(card.index);
            if wants && !selected {
                commands.entity(entity).insert(Selected);
            } else if !wants && selected {
                commands.entity(entity).remove::<Selected>();
            }
        }
    }
}
