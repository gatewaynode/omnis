//! The fight screen's layout (alt-ARCHITECTURE.md §9): where the picture window, the action
//! column, the roll log and the figures sit in a window, the lines that draw the figures, and
//! which figure a point is on. Bevy-free: logical pixels, x to the right and y down from the
//! window's top left.
//!
//! The monster stacks stand across the top of the field, the stacks in front lower and larger
//! than those behind. The party stands in two rows across the bottom, the front row nearer.
//! Every living monster is a figure, up to `MOST` a stack, each with a health bar; a member is a
//! figure with a name and a health bar, marked when down or dead.

use crate::cinema::{self, Drawing, Scene};
use crate::combat_menu::Pick;
use omnis_sim::omnis_data::{Data, Size};
use omnis_sim::ops::party_view;
use omnis_sim::{ActorRef, World, combat_view};

/// Space kept between the screen's parts and around its edge.
pub const MARGIN: f32 = 16.0;
/// The height of the title line above the field.
pub const TITLE: f32 = 40.0;
/// The height of a name line.
pub const LABEL: f32 = 22.0;
/// The height of a health bar.
pub const BAR: f32 = 6.0;
/// The space between a figure, its bar and its name.
pub const GAP: f32 = 4.0;
/// The most figures drawn for one stack; a larger stack shows its count.
pub const MOST: usize = 8;
/// The height kept under the action column for the status lines.
pub const STATUS: f32 = 48.0;
/// The height kept under the roll log for the Save log and Quit buttons.
pub const BUTTONS: f32 = 34.0;

/// A rectangle: its top left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// The right edge.
    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whether the point lies inside or on the edge.
    #[must_use]
    pub fn contains(&self, (x, y): (f32, f32)) -> bool {
        (self.x..=self.right()).contains(&x) && (self.y..=self.bottom()).contains(&y)
    }

    /// Whether `other` lies wholly inside this one.
    #[must_use]
    pub fn encloses(&self, other: &Rect) -> bool {
        self.contains((other.x, other.y)) && self.contains((other.right(), other.bottom()))
    }

    /// Whether the two share any area; touching edges do not count.
    #[must_use]
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// The rectangle grown by `by` on every side.
    #[must_use]
    pub fn grown(&self, by: f32) -> Rect {
        Rect {
            x: self.x - by,
            y: self.y - by,
            w: self.w + 2.0 * by,
            h: self.h + 2.0 * by,
        }
    }

    /// The smallest rectangle holding both.
    #[must_use]
    pub fn union(&self, other: &Rect) -> Rect {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        Rect {
            x,
            y,
            w: self.right().max(other.right()) - x,
            h: self.bottom().max(other.bottom()) - y,
        }
    }

    /// The rectangle cut down to lie inside `bound`.
    #[must_use]
    pub fn within(&self, bound: &Rect) -> Rect {
        let (x, y) = (self.x.max(bound.x), self.y.max(bound.y));
        Rect {
            x,
            y,
            w: (self.right().min(bound.right()) - x).max(0.0),
            h: (self.bottom().min(bound.bottom()) - y).max(0.0),
        }
    }

    /// The `n` equal columns of this rectangle, left to right.
    fn columns(&self, n: usize) -> Vec<Rect> {
        #[allow(clippy::cast_precision_loss)]
        let w = self.w / n.max(1) as f32;
        #[allow(clippy::cast_precision_loss)]
        (0..n)
            .map(|i| Rect {
                x: self.x + w * i as f32,
                w,
                ..*self
            })
            .collect()
    }

    /// The top `share` of this rectangle and the rest below it.
    fn split(&self, share: f32) -> (Rect, Rect) {
        let top = Rect {
            h: self.h * share,
            ..*self
        };
        let rest = Rect {
            y: top.bottom(),
            h: self.h - top.h,
            ..*self
        };
        (top, rest)
    }
}

/// Where the screen's parts sit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// The window.
    pub window: Rect,
    /// The picture window, at the top left (`cinema::SIZE`).
    pub picture: Rect,
    /// The action column, under the picture window.
    pub actions: Rect,
    /// The status lines, under the action column at the bottom left.
    pub status: Rect,
    /// The roll log, down the right side.
    pub log: Rect,
    /// Save log and Quit, under the roll log at the bottom right.
    pub buttons: Rect,
    /// The title line, above the field.
    pub title: Rect,
    /// The field the figures stand in.
    pub field: Rect,
}

/// The screen's parts for a `width × height` window.
#[must_use]
pub fn layout(width: f32, height: f32) -> Layout {
    #[allow(clippy::cast_precision_loss)]
    let (pw, ph) = (cinema::SIZE.0 as f32, cinema::SIZE.1 as f32);
    let picture = Rect {
        x: MARGIN,
        y: MARGIN,
        w: pw,
        h: ph,
    };
    let actions = Rect {
        y: picture.bottom() + MARGIN,
        h: (height - ph - STATUS - 4.0 * MARGIN).max(0.0),
        ..picture
    };
    let status = Rect {
        y: height - MARGIN - STATUS,
        h: STATUS,
        ..picture
    };
    let log_w = (width * 0.24).clamp(300.0, 640.0);
    let log = Rect {
        x: width - MARGIN - log_w,
        y: MARGIN,
        w: log_w,
        h: (height - BUTTONS - 3.0 * MARGIN).max(0.0),
    };
    let buttons = Rect {
        y: height - MARGIN - BUTTONS,
        h: BUTTONS,
        ..log
    };
    let left = picture.right() + MARGIN;
    let title = Rect {
        x: left,
        y: MARGIN,
        w: (log.x - MARGIN - left).max(0.0),
        h: TITLE,
    };
    let field = Rect {
        y: title.bottom() + MARGIN,
        h: (height - title.bottom() - 2.0 * MARGIN).max(0.0),
        ..title
    };
    Layout {
        window: Rect {
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
        },
        picture,
        actions,
        status,
        log,
        buttons,
        title,
        field,
    }
}

/// How a figure stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// On its feet.
    Standing,
    /// At zero hit points.
    Down,
    /// Dead.
    Dead,
}

/// One figure.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    /// Where its drawing lies.
    pub rect: Rect,
    /// Its health bar, under it.
    pub bar: Rect,
    /// The share of its hit points left, 0 to 1.
    pub health: f32,
    /// How it stands.
    pub state: State,
    /// Its drawing, placed in the window.
    pub lines: Drawing,
}

/// A text to show, and where.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// The text.
    pub text: String,
    /// Where it goes.
    pub rect: Rect,
}

/// A stack or a member: what a click on it picks, its figures and its name.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// What a click on it picks.
    pub pick: Pick,
    /// Its figures, left to right.
    pub figures: Vec<Figure>,
    /// Its name, under the figures.
    pub label: Label,
    /// The space it was given; its figures, bars and label lie inside.
    pub slot: Rect,
}

impl Group {
    /// The figures and their bars, together.
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        self.figures
            .iter()
            .map(|f| f.rect.union(&f.bar))
            .reduce(|a, b| a.union(&b))
    }
}

/// What to mark: whose turn it is, what can be clicked, and what the pointer is on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Marks {
    /// The stack or member acting.
    pub acting: Option<Pick>,
    /// The valid targets.
    pub targets: Vec<Pick>,
    /// The target under the pointer.
    pub hover: Option<Pick>,
}

/// The fight screen laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct Arena {
    /// The screen's parts.
    pub layout: Layout,
    /// The stacks still standing.
    pub stacks: Vec<Group>,
    /// The members, in marching order.
    pub members: Vec<Group>,
    /// What is marked.
    pub marks: Marks,
}

impl Arena {
    /// Every group, stacks first.
    pub fn groups(&self) -> impl Iterator<Item = &Group> {
        self.stacks.iter().chain(&self.members)
    }
}

/// How tall a creature of this size stands, as a share of the room it is given.
fn stature(size: Size) -> f32 {
    match size {
        Size::Tiny => 0.45,
        Size::Small => 0.65,
        Size::Medium => 0.8,
        Size::Large => 0.9,
        Size::Huge | Size::Gargantuan => 1.0,
    }
}

/// A member's figure: a person seen from behind, in a unit square.
fn person() -> Drawing {
    let head: Vec<(f32, f32)> = (0..=12u8)
        .map(|i| {
            let a = f32::from(i) / 12.0 * core::f32::consts::TAU;
            (0.5 + 0.11 * a.cos(), 0.13 + 0.11 * a.sin())
        })
        .collect();
    vec![
        head,
        vec![(0.5, 0.24), (0.5, 0.62)],
        vec![(0.2, 0.5), (0.5, 0.32), (0.8, 0.5)],
        vec![(0.28, 0.98), (0.5, 0.62), (0.72, 0.98)],
    ]
}

/// One figure of a kind, as the slot lays it out.
struct Kind {
    drawing: Drawing,
    stature: f32,
}

/// The figures of a group in `slot`: up to `MOST` of them side by side, standing on a common
/// ground line, each with its bar below. Returns the figures and the label's rectangle.
fn stand(slot: &Rect, kind: &Kind, health: &[(f32, State)], text: String) -> (Vec<Figure>, Label) {
    let shown = health.len().min(MOST);
    let room = (slot.h - LABEL - BAR - 2.0 * GAP).max(0.0);
    #[allow(clippy::cast_precision_loss)]
    let cell = slot.w / shown.max(1) as f32;
    let (_, _, bw, bh) = cinema::bounds(&kind.drawing);
    let aspect = bw / bh;
    let mut h = room * kind.stature;
    let mut w = h * aspect;
    if w > cell * 0.85 {
        w = cell * 0.85;
        h = w / aspect;
    }
    let pitch = cell.min(w * 1.3);
    let ground = slot.y + room;
    #[allow(clippy::cast_precision_loss)]
    let start = slot.x + slot.w / 2.0 - pitch * shown as f32 / 2.0;
    let figures = health
        .iter()
        .take(shown)
        .enumerate()
        .map(|(i, &(share, state))| {
            #[allow(clippy::cast_precision_loss)]
            let centre = start + pitch * (i as f32 + 0.5);
            let rect = Rect {
                x: centre - w / 2.0,
                y: ground - h,
                w,
                h,
            };
            let at = cinema::place(&kind.drawing, (rect.x, rect.y, rect.w, rect.h), 0.0);
            Figure {
                rect,
                bar: Rect {
                    y: ground + GAP,
                    h: BAR,
                    ..rect
                },
                health: share.clamp(0.0, 1.0),
                state,
                lines: kind
                    .drawing
                    .iter()
                    .map(|line| line.iter().map(|&p| at(p)).collect())
                    .collect(),
            }
        })
        .collect();
    let label = Label {
        text,
        rect: Rect {
            y: ground + 2.0 * GAP + BAR,
            h: LABEL,
            ..*slot
        },
    };
    (figures, label)
}

/// `hp` out of `max` as a share.
#[allow(clippy::cast_precision_loss)]
fn share(hp: i32, max: i32) -> f32 {
    hp.max(0) as f32 / max.max(1) as f32
}

/// The fight screen for a `width × height` window, or `None` outside a fight. `targets` and
/// `hover` are marked; the acting stack or member is found from the world.
#[must_use]
pub fn arena(
    world: &World,
    data: &Data,
    (width, height): (f32, f32),
    targets: &[Pick],
    hover: Option<Pick>,
) -> Option<Arena> {
    let view = combat_view(world, data)?;
    let layout = layout(width, height);
    let (monsters, party) = layout.field.split(0.55);
    let (back, front) = monsters.split(0.45);
    let mut stacks = Vec::new();
    for (band, in_front) in [(back, false), (front, true)] {
        let here: Vec<_> = view
            .stacks
            .iter()
            .filter(|s| s.alive && s.in_front == in_front)
            .collect();
        for (stack, slot) in here.iter().zip(band.columns(here.len())) {
            let id = data.registry.monsters.get(&stack.monster)?;
            let monster = data.monsters.get(&id)?;
            let max = monster.hit_points.max();
            let health: Vec<_> = stack
                .hps
                .iter()
                .map(|&hp| (share(hp, max), State::Standing))
                .collect();
            let text = if stack.hps.len() > MOST {
                format!("{} x{}", data.label("en", &stack.name), stack.hps.len())
            } else {
                data.label("en", &stack.name).to_owned()
            };
            let kind = Kind {
                drawing: cinema::drawing(Scene::Enemy(id)),
                stature: stature(monster.size),
            };
            let (figures, label) = stand(&slot, &kind, &health, text);
            stacks.push(Group {
                pick: Pick::Stack(stack.stack),
                figures,
                label,
                slot,
            });
        }
    }
    let (back, front) = party.split(0.45);
    let view_members = party_view(world, data).members;
    let mut members = Vec::new();
    let kind = Kind {
        drawing: person(),
        stature: stature(Size::Medium),
    };
    for (band, in_front) in [(back, false), (front, true)] {
        let here: Vec<_> = view_members
            .iter()
            .filter(|m| m.in_front == in_front)
            .collect();
        for (member, slot) in here.iter().zip(band.columns(here.len())) {
            let state = if member.dead {
                State::Dead
            } else if member.down {
                State::Down
            } else {
                State::Standing
            };
            let health = [(share(member.hp, member.hp_max), state)];
            let (figures, label) = stand(&slot, &kind, &health, member.name.clone());
            members.push(Group {
                pick: Pick::Member(member.member),
                figures,
                label,
                slot,
            });
        }
    }
    let slot = |pick: Pick| {
        view_members
            .iter()
            .position(|m| Pick::Member(m.member) == pick)
    };
    members.sort_by_key(|g| slot(g.pick));
    let acting = match view.current {
        Some(ActorRef::Member(id)) => Some(Pick::Member(id)),
        Some(ActorRef::Stack(stack) | ActorRef::Monster { stack, .. }) => Some(Pick::Stack(stack)),
        None => None,
    };
    Some(Arena {
        layout,
        stacks,
        members,
        marks: Marks {
            acting,
            targets: targets.to_vec(),
            hover,
        },
    })
}

/// What a line is, for its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// A figure on its feet.
    Figure,
    /// A figure that is down, and its mark.
    Down,
    /// A dead figure, and its mark.
    Dead,
    /// A health bar's frame.
    Bar,
    /// A health bar's fill.
    Health,
    /// The frame round whoever is acting.
    Acting,
    /// The frame round a valid target.
    Target,
    /// The frame round the target under the pointer.
    Hover,
}

/// A line to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg2 {
    /// One end.
    pub a: (f32, f32),
    /// The other end.
    pub b: (f32, f32),
    /// What it is.
    pub tone: Tone,
}

/// The four sides of a rectangle.
fn outline(r: &Rect, tone: Tone, out: &mut Vec<Seg2>) {
    let corners = [
        (r.x, r.y),
        (r.right(), r.y),
        (r.right(), r.bottom()),
        (r.x, r.bottom()),
    ];
    for i in 0..4 {
        out.push(Seg2 {
            a: corners[i],
            b: corners[(i + 1) % 4],
            tone,
        });
    }
}

/// Brackets at a rectangle's four corners.
fn brackets(r: &Rect, tone: Tone, out: &mut Vec<Seg2>) {
    let arm = (r.w.min(r.h) / 4.0).min(14.0);
    for (x, y, dx, dy) in [
        (r.x, r.y, 1.0, 1.0),
        (r.right(), r.y, -1.0, 1.0),
        (r.right(), r.bottom(), -1.0, -1.0),
        (r.x, r.bottom(), 1.0, -1.0),
    ] {
        out.push(Seg2 {
            a: (x, y),
            b: (x + dx * arm, y),
            tone,
        });
        out.push(Seg2 {
            a: (x, y),
            b: (x, y + dy * arm),
            tone,
        });
    }
}

/// A figure's lines, its mark when down or dead, and its health bar.
fn figure(f: &Figure, out: &mut Vec<Seg2>) {
    let tone = match f.state {
        State::Standing => Tone::Figure,
        State::Down => Tone::Down,
        State::Dead => Tone::Dead,
    };
    for line in &f.lines {
        for pair in line.windows(2) {
            out.push(Seg2 {
                a: pair[0],
                b: pair[1],
                tone,
            });
        }
    }
    let r = &f.rect;
    match f.state {
        State::Standing => {}
        State::Down => out.push(Seg2 {
            a: (r.x, r.bottom()),
            b: (r.right(), r.y),
            tone,
        }),
        State::Dead => {
            out.push(Seg2 {
                a: (r.x, r.y),
                b: (r.right(), r.bottom()),
                tone,
            });
            out.push(Seg2 {
                a: (r.x, r.bottom()),
                b: (r.right(), r.y),
                tone,
            });
        }
    }
    outline(&f.bar, Tone::Bar, out);
    let filled = (f.bar.w - 2.0) * f.health;
    let mut y = f.bar.y + 1.5;
    while y < f.bar.bottom() - 1.0 && filled > 0.0 {
        out.push(Seg2 {
            a: (f.bar.x + 1.0, y),
            b: (f.bar.x + 1.0 + filled, y),
            tone: Tone::Health,
        });
        y += 1.0;
    }
}

/// How far a group's frames stand off its figures: the acting frame outside a target's.
const ACTING_FRAME: f32 = 10.0;
const TARGET_FRAME: f32 = 5.0;

/// Every line of the screen's figures: the figures, their bars and the frames that mark them.
#[must_use]
pub fn segments(arena: &Arena) -> Vec<Seg2> {
    let mut out = Vec::new();
    for group in arena.groups() {
        for f in &group.figures {
            figure(f, &mut out);
        }
        let Some(bounds) = group.bounds() else {
            continue;
        };
        let marks = &arena.marks;
        if marks.acting == Some(group.pick) {
            brackets(
                &bounds.grown(ACTING_FRAME).within(&group.slot),
                Tone::Acting,
                &mut out,
            );
        }
        if marks.targets.contains(&group.pick) {
            let tone = if marks.hover == Some(group.pick) {
                Tone::Hover
            } else {
                Tone::Target
            };
            outline(
                &bounds.grown(TARGET_FRAME).within(&group.slot),
                tone,
                &mut out,
            );
        }
    }
    out
}

/// The stack or member whose figure or health bar is at `point`.
#[must_use]
pub fn pick(arena: &Arena, point: (f32, f32)) -> Option<Pick> {
    arena
        .groups()
        .find(|g| {
            g.figures
                .iter()
                .any(|f| f.rect.contains(point) || f.bar.contains(point))
        })
        .map(|g| g.pick)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_lie_apart_inside_the_window() {
        for (w, h) in [(1600.0, 900.0), (5120.0, 1440.0), (1920.0, 1080.0)] {
            let l = layout(w, h);
            let parts = [
                l.picture, l.actions, l.status, l.log, l.buttons, l.title, l.field,
            ];
            for (i, a) in parts.iter().enumerate() {
                assert!(l.window.encloses(a), "{a:?} inside {w}×{h}");
                assert!(a.w > 0.0 && a.h > 0.0, "{a:?} has room");
                for b in &parts[i + 1..] {
                    assert!(!a.overlaps(b), "{a:?} apart from {b:?}");
                }
            }
            assert!(l.picture.y < l.actions.y, "the picture over the actions");
            assert_eq!(l.picture.x, l.actions.x);
        }
    }

    #[test]
    fn figures_in_a_slot_stand_apart_on_one_ground_line() {
        let slot = Rect {
            x: 100.0,
            y: 50.0,
            w: 600.0,
            h: 300.0,
        };
        let kind = Kind {
            drawing: person(),
            stature: 0.8,
        };
        let health = vec![(1.0, State::Standing); 12];
        let (figures, label) = stand(&slot, &kind, &health, "Twelve".into());
        assert_eq!(figures.len(), MOST, "no more than MOST drawn");
        for pair in figures.windows(2) {
            assert!(pair[0].rect.right() < pair[1].rect.x, "a gap between");
            assert!((pair[0].rect.bottom() - pair[1].rect.bottom()).abs() < 0.01);
        }
        for f in &figures {
            assert!(slot.encloses(&f.rect) && slot.encloses(&f.bar));
            assert!(f.bar.y > f.rect.bottom() && label.rect.y > f.bar.bottom());
        }
        assert!(slot.encloses(&label.rect));
    }

    #[test]
    fn a_health_bar_fills_by_the_share_left() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 40.0,
            h: 80.0,
        };
        let full = |health| Figure {
            rect,
            bar: Rect {
                y: 90.0,
                h: BAR,
                ..rect
            },
            health,
            state: State::Standing,
            lines: Vec::new(),
        };
        let reach = |health| {
            let mut out = Vec::new();
            figure(&full(health), &mut out);
            out.iter()
                .filter(|s| s.tone == Tone::Health)
                .map(|s| s.b.0 - s.a.0)
                .fold(0.0_f32, f32::max)
        };
        assert!((reach(1.0) - 38.0).abs() < 0.01);
        assert!((reach(0.25) - 9.5).abs() < 0.01);
        assert!(reach(0.0) == 0.0);
    }
}
