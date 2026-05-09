//! Live tour of `sweeten`'s CSS-flex [`Row`] / [`Column`] widgets.
//!
//! A two-pane layout: a sticky control sidebar on the left, and a
//! demo canvas on the right. Each demo card shows one feature in
//! isolation — `justify-content`, `align-items`, `flex-grow`,
//! `flex-shrink`, `flex-basis`, `align-self`, `flex-direction:
//! row-reverse`, and a kitchen-sink "real-world" example.
//!
//! The sliders and pick lists in the sidebar drive the active card,
//! so the same demo can be re-rendered with every combination of
//! axis, justify, align, gap, padding, and reverse.
//!
//! Run with: `cargo run --example flex`
//!
//! [`Row`]: sweeten::widget::flex::Row
//! [`Column`]: sweeten::widget::flex::Column

use iced::widget::{column, container, row, rule, scrollable, slider, text};
use iced::{
    Center, Element, Fill, Length, Shrink, Subscription, Theme, keyboard,
};

use sweeten::widget::flex::{self, AlignItems, Axis, FlexChild, Justify, flex};
use sweeten::widget::{checkbox, pick_list};

/// Cycle forward (or backward) through a static list, wrapping at
/// either end. Returns the input unchanged for empty slices.
fn cycle<T: PartialEq + Copy>(items: &[T], current: T, forward: bool) -> T {
    let n = items.len();
    if n == 0 {
        return current;
    }
    let pos = items.iter().position(|x| *x == current).unwrap_or(0);
    let next = if forward {
        (pos + 1) % n
    } else {
        (pos + n - 1) % n
    };
    items[next]
}

pub fn main() -> iced::Result {
    iced::application(FlexTour::default, FlexTour::update, FlexTour::view)
        .title(FlexTour::title)
        .theme(FlexTour::theme)
        .subscription(FlexTour::subscription)
        .window_size((1100.0, 720.0))
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Demo {
    Basic,
    JustifyContent,
    AlignItems,
    Grow,
    Shrink,
    Basis,
    AlignSelf,
    PaddingGap,
    Mixed,
}

impl Demo {
    const ALL: &'static [Self] = &[
        Self::Basic,
        Self::JustifyContent,
        Self::AlignItems,
        Self::Grow,
        Self::Shrink,
        Self::Basis,
        Self::AlignSelf,
        Self::PaddingGap,
        Self::Mixed,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::JustifyContent => "justify-content",
            Self::AlignItems => "align-items",
            Self::Grow => "flex-grow",
            Self::Shrink => "flex-shrink",
            Self::Basis => "flex-basis",
            Self::AlignSelf => "align-self",
            Self::PaddingGap => "padding & gap",
            Self::Mixed => "Mixed (kitchen sink)",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Self::Basic => {
                "Three children with default flex props — \
                packed at the main-start edge, no grow, no shrink \
                beyond CSS defaults."
            }
            Self::JustifyContent => {
                "Six rows, one per Justify variant. \
                The container's main length is fixed, so each leftover \
                pocket is distributed differently."
            }
            Self::AlignItems => {
                "Four columns, one per AlignItems variant. \
                Children have varied cross sizes so the alignment is \
                visible at a glance."
            }
            Self::Grow => {
                "Three items with grow ratios 1 : 2 : 1 — the \
                middle child takes twice the surplus main-axis space."
            }
            Self::Shrink => {
                "Three fixed-basis items in a too-narrow \
                container. CSS shrink scales by basis * shrink, so the \
                largest item absorbs the largest share of the deficit."
            }
            Self::Basis => {
                "Three items with explicit pixel bases (80, \
                160, 80) plus a fourth that grows into the leftover space."
            }
            Self::AlignSelf => {
                "Container is align-items: Stretch. Two \
                children opt out via align_self — one to Start, one to \
                End — overriding the container's choice."
            }
            Self::PaddingGap => {
                "Padding insets the children from the \
                container; gap separates adjacent items. Both contribute \
                to the main-axis budget before grow distributes leftover."
            }
            Self::Mixed => {
                "A real-world layout: a flex_column with a \
                header bar, a body row with a sidebar (fixed basis) plus \
                a growing main panel, and a footer."
            }
        }
    }
}

impl std::fmt::Display for Demo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AxisChoice(Axis);

impl AxisChoice {
    const ALL: &'static [Self] =
        &[Self(Axis::Horizontal), Self(Axis::Vertical)];
}

impl std::fmt::Display for AxisChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Axis::Horizontal => f.write_str("Row (horizontal)"),
            Axis::Vertical => f.write_str("Column (vertical)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JustifyChoice(Option<Justify>);

impl JustifyChoice {
    const ALL: &'static [Self] = &[
        Self(None),
        Self(Some(Justify::Start)),
        Self(Some(Justify::End)),
        Self(Some(Justify::Center)),
        Self(Some(Justify::SpaceBetween)),
        Self(Some(Justify::SpaceAround)),
        Self(Some(Justify::SpaceEvenly)),
    ];
}

impl std::fmt::Display for JustifyChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => f.write_str("Default (per-demo)"),
            Some(Justify::Start) => f.write_str("Start"),
            Some(Justify::End) => f.write_str("End"),
            Some(Justify::Center) => f.write_str("Center"),
            Some(Justify::SpaceBetween) => f.write_str("SpaceBetween"),
            Some(Justify::SpaceAround) => f.write_str("SpaceAround"),
            Some(Justify::SpaceEvenly) => f.write_str("SpaceEvenly"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AlignChoice(Option<AlignItems>);

impl AlignChoice {
    const ALL: &'static [Self] = &[
        Self(None),
        Self(Some(AlignItems::Start)),
        Self(Some(AlignItems::End)),
        Self(Some(AlignItems::Center)),
        Self(Some(AlignItems::Stretch)),
    ];
}

impl std::fmt::Display for AlignChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => f.write_str("Default (per-demo)"),
            Some(AlignItems::Start) => f.write_str("Start"),
            Some(AlignItems::End) => f.write_str("End"),
            Some(AlignItems::Center) => f.write_str("Center"),
            Some(AlignItems::Stretch) => f.write_str("Stretch"),
        }
    }
}

struct FlexTour {
    demo: Demo,
    axis: AxisChoice,
    gap: f32,
    padding: f32,
    justify_override: JustifyChoice,
    align_override: AlignChoice,
    reverse: bool,
}

impl Default for FlexTour {
    fn default() -> Self {
        Self {
            demo: Demo::Basic,
            axis: AxisChoice(Axis::Horizontal),
            gap: 12.0,
            padding: 16.0,
            justify_override: JustifyChoice(None),
            align_override: AlignChoice(None),
            reverse: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    DemoSelected(Demo),
    AxisSelected(AxisChoice),
    GapChanged(f32),
    PaddingChanged(f32),
    JustifySelected(JustifyChoice),
    AlignSelected(AlignChoice),
    ReverseToggled(bool),
    // Keyboard-driven actions. Direct messages so the keyboard
    // subscription stays a thin mapping layer.
    NextDemo,
    PrevDemo,
    ToggleAxis,
    ForceRowAxis,
    ForceColumnAxis,
    JustifyNext,
    JustifyPrev,
    AlignNext,
    AlignPrev,
    GapDelta(f32),
    PaddingDelta(f32),
    ToggleReverse,
}

impl FlexTour {
    fn title(&self) -> String {
        format!("sweeten • flex tour — {}", self.demo.label())
    }

    fn theme(&self) -> Theme {
        Theme::Oxocarbon
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::DemoSelected(demo) => self.demo = demo,
            Message::AxisSelected(axis) => self.axis = axis,
            Message::GapChanged(gap) => self.gap = gap,
            Message::PaddingChanged(padding) => self.padding = padding,
            Message::JustifySelected(j) => self.justify_override = j,
            Message::AlignSelected(a) => self.align_override = a,
            Message::ReverseToggled(r) => self.reverse = r,
            Message::NextDemo => {
                self.demo = cycle(Demo::ALL, self.demo, true);
            }
            Message::PrevDemo => {
                self.demo = cycle(Demo::ALL, self.demo, false);
            }
            Message::ToggleAxis => {
                self.axis = AxisChoice(match self.axis.0 {
                    Axis::Horizontal => Axis::Vertical,
                    Axis::Vertical => Axis::Horizontal,
                });
            }
            Message::ForceRowAxis => {
                self.axis = AxisChoice(Axis::Horizontal);
            }
            Message::ForceColumnAxis => {
                self.axis = AxisChoice(Axis::Vertical);
            }
            Message::JustifyNext => {
                self.justify_override =
                    cycle(JustifyChoice::ALL, self.justify_override, true);
            }
            Message::JustifyPrev => {
                self.justify_override =
                    cycle(JustifyChoice::ALL, self.justify_override, false);
            }
            Message::AlignNext => {
                self.align_override =
                    cycle(AlignChoice::ALL, self.align_override, true);
            }
            Message::AlignPrev => {
                self.align_override =
                    cycle(AlignChoice::ALL, self.align_override, false);
            }
            Message::GapDelta(d) => {
                self.gap = (self.gap + d).clamp(0.0, 48.0);
            }
            Message::PaddingDelta(d) => {
                self.padding = (self.padding + d).clamp(0.0, 32.0);
            }
            Message::ToggleReverse => {
                self.reverse = !self.reverse;
            }
        }
    }

    /// Global keybindings, mirroring `~/projects/hyozu/examples/
    /// gauge_chart.rs`'s subscription pattern.
    ///
    /// - `PageUp` / `PageDown`: previous / next demo
    /// - `X`: toggle row ↔ column
    /// - `R` / `C`: force-set Row / Column axis
    /// - `V`: toggle reverse
    /// - `J` / `Shift+J`: cycle justify-content forward / backward
    /// - `A` / `Shift+A`: cycle align-items forward / backward
    /// - `←` / `→`: gap −2 / +2
    /// - `↑` / `↓`: padding +2 / −2
    fn subscription(&self) -> Subscription<Message> {
        use keyboard::key::{Key, Named};

        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event
            else {
                return None;
            };

            match key.as_ref() {
                Key::Named(Named::PageDown) => Some(Message::NextDemo),
                Key::Named(Named::PageUp) => Some(Message::PrevDemo),
                Key::Named(Named::ArrowLeft) => Some(Message::GapDelta(-2.0)),
                Key::Named(Named::ArrowRight) => Some(Message::GapDelta(2.0)),
                Key::Named(Named::ArrowUp) => Some(Message::PaddingDelta(2.0)),
                Key::Named(Named::ArrowDown) => {
                    Some(Message::PaddingDelta(-2.0))
                }
                Key::Character(c) => {
                    let lower = c.to_lowercase();
                    match (lower.as_str(), modifiers.shift()) {
                        ("x", _) => Some(Message::ToggleAxis),
                        ("r", _) => Some(Message::ForceRowAxis),
                        ("c", _) => Some(Message::ForceColumnAxis),
                        ("v", _) => Some(Message::ToggleReverse),
                        ("j", true) => Some(Message::JustifyPrev),
                        ("j", false) => Some(Message::JustifyNext),
                        ("a", true) => Some(Message::AlignPrev),
                        ("a", false) => Some(Message::AlignNext),
                        _ => None,
                    }
                }
                _ => None,
            }
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let canvas = container(self.canvas())
            .padding(20)
            .width(Fill)
            .height(Fill)
            .style(canvas_style);

        // Pull the divider darker than the default (`background.strong`,
        // bright on dark themes) and one notch darker than the
        // `rule::weak` preset, so it reads as a recessed seam between
        // the sidebar and the canvas rather than a bright outline.
        let divider = rule::vertical(1).style(|theme: &Theme| rule::Style {
            color: theme.palette().background.weaker.color,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        });

        row![sidebar(self), divider, canvas].height(Fill).into()
    }

    fn canvas(&self) -> Element<'_, Message> {
        let title = text(self.demo.label()).size(22);
        let blurb = text(self.demo.blurb()).size(13).style(text::secondary);

        let demo: Element<'_, Message> = match self.demo {
            Demo::Basic => self.demo_basic(),
            Demo::JustifyContent => self.demo_justify_content(),
            Demo::AlignItems => self.demo_align_items(),
            Demo::Grow => self.demo_grow(),
            Demo::Shrink => self.demo_shrink(),
            Demo::Basis => self.demo_basis(),
            Demo::AlignSelf => self.demo_align_self(),
            Demo::PaddingGap => self.demo_padding_gap(),
            Demo::Mixed => self.demo_mixed(),
        };

        column![title, blurb, demo]
            .spacing(14)
            .width(Fill)
            .height(Fill)
            .into()
    }

    // --- Demos ----------------------------------------------------------

    /// Three children, default props. Showcases packing.
    fn demo_basic(&self) -> Element<'_, Message> {
        let kids = [
            cell("A", "", 90.0, 60.0),
            cell("B", "", 120.0, 60.0),
            cell("C", "", 80.0, 60.0),
        ];
        let demo = flex_demo(self.axis.0, kids)
            .gap(self.gap)
            .padding(self.padding)
            .align(self.align_override.0.unwrap_or(AlignItems::Start))
            .justify(self.justify_override.0.unwrap_or(Justify::Start))
            .reverse(self.reverse)
            .width(Fill)
            .height(Fill);
        frame(demo.into())
    }

    /// Six tracks, one per Justify variant. In Row mode they stack
    /// vertically as wide-and-thin strips; in Column mode they sit
    /// side-by-side as narrow-and-tall strips so each variant has a
    /// generous main-axis budget for its leftover-distribution.
    fn demo_justify_content(&self) -> Element<'_, Message> {
        let variants = [
            (Justify::Start, "Start"),
            (Justify::End, "End"),
            (Justify::Center, "Center"),
            (Justify::SpaceBetween, "SpaceBetween"),
            (Justify::SpaceAround, "SpaceAround"),
            (Justify::SpaceEvenly, "SpaceEvenly"),
        ];

        let is_horizontal = matches!(self.axis.0, Axis::Horizontal);

        let tracks =
            variants
                .into_iter()
                .map(|(j, name)| -> Element<'_, Message> {
                    let inner_builder = flex_demo(
                        self.axis.0,
                        [
                            cell("A", "", 56.0, 40.0),
                            cell("B", "", 56.0, 40.0),
                            cell("C", "", 56.0, 40.0),
                        ],
                    )
                    .gap(self.gap)
                    .padding(8.0)
                    .align(AlignItems::Center)
                    .justify(self.justify_override.0.unwrap_or(j))
                    .reverse(self.reverse);

                    let inner: Element<'_, Message> = if is_horizontal {
                        // Wide horizontal strip — main = width.
                        inner_builder.width(Fill).height(60.0).into()
                    } else {
                        // Narrow vertical strip — main = height. 220px
                        // gives 3*40 + 2*gap (~24) + padding(16) = ~160
                        // intrinsic plus ~60 leftover for the
                        // SpaceBetween/Around/Evenly cases to spread.
                        inner_builder.width(84.0).height(220.0).into()
                    };

                    if is_horizontal {
                        row![
                            text(name).size(12).width(110.0),
                            container(inner).width(Fill).style(track_style),
                        ]
                        .spacing(8)
                        .align_y(Center)
                        .into()
                    } else {
                        column![
                            text(name).size(12),
                            container(inner).style(track_style),
                        ]
                        .spacing(4)
                        .align_x(Center)
                        .into()
                    }
                });

        let stack: Element<'_, Message> = if is_horizontal {
            column(tracks).spacing(8).width(Fill).into()
        } else {
            row(tracks).spacing(8).height(Fill).into()
        };
        frame(stack)
    }

    /// Four variants of AlignItems, one per column. Cells have varied
    /// cross-axis sizes so the alignment is visible at a glance — that
    /// means varying *heights* in Row mode (h=28/56/84) and varying
    /// *widths* in Column mode (w=28/56/84).
    fn demo_align_items(&self) -> Element<'_, Message> {
        let variants = [
            (AlignItems::Start, "Start"),
            (AlignItems::Center, "Center"),
            (AlignItems::End, "End"),
            (AlignItems::Stretch, "Stretch"),
        ];

        let is_horizontal = matches!(self.axis.0, Axis::Horizontal);

        let columns =
            variants
                .into_iter()
                .map(|(a, name)| -> Element<'_, Message> {
                    let cells = if is_horizontal {
                        // Cross = height; vary it.
                        [
                            cell("A", "h=28", 36.0, 28.0),
                            cell("B", "h=56", 36.0, 56.0),
                            cell("C", "h=84", 36.0, 84.0),
                        ]
                    } else {
                        // Cross = width; vary it.
                        [
                            cell("A", "w=28", 28.0, 36.0),
                            cell("B", "w=56", 56.0, 36.0),
                            cell("C", "w=84", 84.0, 36.0),
                        ]
                    };

                    let inner_builder = flex_demo(self.axis.0, cells)
                        .gap(self.gap)
                        .padding(8.0)
                        .align(self.align_override.0.unwrap_or(a))
                        .justify(Justify::Center)
                        .reverse(self.reverse);

                    let inner: Element<'_, Message> = if is_horizontal {
                        inner_builder.width(Fill).height(140.0).into()
                    } else {
                        // Cross budget needs to fit the widest cell
                        // (84) plus padding plus a little headroom so
                        // Stretch is distinguishable from Start/End.
                        inner_builder.width(140.0).height(220.0).into()
                    };

                    column![
                        text(name).size(12),
                        container(inner).style(track_style),
                    ]
                    .spacing(6)
                    .width(Fill)
                    .into()
                });

        let grid = row(columns).spacing(12).width(Fill);
        frame(grid.into())
    }

    /// Three growing items with 1 : 2 : 1 ratios.
    fn demo_grow(&self) -> Element<'_, Message> {
        let kids = [
            flex(boxed("A", "grow=1", Accent::Primary)).grow(1.0),
            flex(boxed("B", "grow=2", Accent::Warning)).grow(2.0),
            flex(boxed("C", "grow=1", Accent::Success)).grow(1.0),
        ];
        frame(self.shell_with(kids, AlignItems::Stretch, Justify::Start))
    }

    /// Three fixed-basis items in a too-narrow container.
    fn demo_shrink(&self) -> Element<'_, Message> {
        let kids = [
            flex(boxed("A", "basis=200", Accent::Primary))
                .basis(200.0)
                .shrink(1.0),
            flex(boxed("B", "basis=300", Accent::Warning))
                .basis(300.0)
                .shrink(1.0),
            flex(boxed("C", "basis=200", Accent::Success))
                .basis(200.0)
                .shrink(1.0),
        ];
        frame(self.shell_with(kids, AlignItems::Stretch, Justify::Start))
    }

    /// Three explicit-basis items plus a fourth grower.
    fn demo_basis(&self) -> Element<'_, Message> {
        let kids = [
            flex(boxed("A", "basis=80", Accent::Primary)).basis(80.0),
            flex(boxed("B", "basis=160", Accent::Warning)).basis(160.0),
            flex(boxed("C", "basis=80", Accent::Success)).basis(80.0),
            flex(boxed("D", "grow=1", Accent::Danger)).grow(1.0),
        ];
        frame(self.shell_with(kids, AlignItems::Stretch, Justify::Start))
    }

    /// Container is Stretch; two children override via align_self.
    fn demo_align_self(&self) -> Element<'_, Message> {
        use sweeten::widget::flex::AlignSelf;

        let kids = [
            flex(cell("A", "", 80.0, 0.0)),
            flex(cell("B", "self=Start", 80.0, 40.0))
                .align_self(AlignSelf::Start),
            flex(cell("C", "", 80.0, 0.0)),
            flex(cell("D", "self=End", 80.0, 40.0)).align_self(AlignSelf::End),
            flex(cell("E", "", 80.0, 0.0)),
        ];
        frame(self.shell_with(kids, AlignItems::Stretch, Justify::Start))
    }

    /// Padding & gap interaction with grow.
    fn demo_padding_gap(&self) -> Element<'_, Message> {
        let kids = [
            flex(boxed("A", "grow=1", Accent::Primary)).grow(1.0),
            flex(boxed("B", "grow=1", Accent::Warning)).grow(1.0),
            flex(boxed("C", "grow=1", Accent::Success)).grow(1.0),
        ];
        frame(self.shell_with(kids, AlignItems::Stretch, Justify::Start))
    }

    /// Kitchen-sink: nested flex_row inside flex_column.
    fn demo_mixed(&self) -> Element<'_, Message> {
        let header = container(text("HEADER").size(14))
            .padding([8.0, 12.0])
            .width(Fill)
            .style(header_style);

        let sidebar = container(
            column![
                text("Sidebar").size(13),
                text("• item 1").size(11).style(text::secondary),
                text("• item 2").size(11).style(text::secondary),
                text("• item 3").size(11).style(text::secondary),
            ]
            .spacing(6),
        )
        .padding(12)
        .width(Fill)
        .height(Fill)
        .style(sidebar_style);

        let main = container(
            column![
                text("Main panel").size(14),
                text(
                    "Grows into the leftover main-axis space. The \
                    sidebar holds a fixed pixel basis."
                )
                .size(11)
                .style(text::secondary),
            ]
            .spacing(6),
        )
        .padding(12)
        .width(Fill)
        .height(Fill)
        .style(panel_style);

        let body = flex::row([])
            .push_flex(flex(sidebar).basis(160.0).shrink(0.0))
            .push_flex(flex(main).grow(1.0))
            .gap(8.0)
            .width(Fill)
            .height(Fill);

        let footer = container(
            text("Footer · last updated just now")
                .size(11)
                .style(text::secondary),
        )
        .padding([6.0, 12.0])
        .width(Fill)
        .style(footer_style);

        let app: Element<'_, Message> = flex::column![
            flex(header).shrink(0.0),
            flex(body).grow(1.0),
            flex(footer).shrink(0.0),
        ]
        .gap(self.gap)
        .padding(self.padding)
        .width(Fill)
        .height(Fill)
        .into();

        frame(app)
    }

    // --- Helpers --------------------------------------------------------

    /// Builds the active flex container with the current axis, gap,
    /// padding, and override settings.
    fn shell_with<'a, I>(
        &'a self,
        children: I,
        default_align: AlignItems,
        default_justify: Justify,
    ) -> Element<'a, Message>
    where
        I: IntoIterator<Item = FlexChild<'a, Message>>,
    {
        flex_demo_flex(self.axis.0, children)
            .gap(self.gap)
            .padding(self.padding)
            .align(self.align_override.0.unwrap_or(default_align))
            .justify(self.justify_override.0.unwrap_or(default_justify))
            .reverse(self.reverse)
            .width(Fill)
            .height(Fill)
            .into()
    }
}

// --- Free helpers ---------------------------------------------------------

/// Wraps any element in the bordered demo frame.
fn frame<'a, Message: 'a>(
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    container(content)
        .padding(8)
        .width(Fill)
        .height(Fill)
        .style(frame_style)
        .into()
}

/// Builds a flex container along `axis` from a list of `FlexChild`.
///
/// We can't return `flex::Row` or `flex::Column` directly because the
/// two have distinct types — wrapping into an `Element` at the call
/// site would lose the builder methods. Instead we construct one and
/// return an `Element` after applying the shared modifiers.
fn flex_demo_flex<'a, Message: 'a, I>(
    axis: Axis,
    children: I,
) -> ContainerBuilder<'a, Message>
where
    I: IntoIterator<Item = FlexChild<'a, Message>>,
{
    match axis {
        Axis::Horizontal => {
            ContainerBuilder::Row(flex::Row::with_flex_children(children))
        }
        Axis::Vertical => {
            ContainerBuilder::Column(flex::Column::with_flex_children(children))
        }
    }
}

/// Same as [`flex_demo_flex`] but takes plain elements.
fn flex_demo<'a, Message: 'a, I, E>(
    axis: Axis,
    children: I,
) -> ContainerBuilder<'a, Message>
where
    I: IntoIterator<Item = E>,
    E: Into<Element<'a, Message>>,
{
    let flex_children = children.into_iter().map(|e| FlexChild::from(e.into()));
    flex_demo_flex(axis, flex_children)
}

/// A small wrapper that lets the demo functions chain builder methods
/// once and convert to `Element` at the end, regardless of axis. Each
/// builder method dispatches to the underlying [`flex::Row`] or
/// [`flex::Column`].
enum ContainerBuilder<'a, Message> {
    Row(flex::Row<'a, Message>),
    Column(flex::Column<'a, Message>),
}

impl<'a, Message: 'a> ContainerBuilder<'a, Message> {
    fn gap(self, amount: f32) -> Self {
        match self {
            Self::Row(r) => Self::Row(r.gap(amount)),
            Self::Column(c) => Self::Column(c.gap(amount)),
        }
    }

    fn padding(self, amount: f32) -> Self {
        match self {
            Self::Row(r) => Self::Row(r.padding(amount)),
            Self::Column(c) => Self::Column(c.padding(amount)),
        }
    }

    fn align(self, align: AlignItems) -> Self {
        match self {
            Self::Row(r) => Self::Row(r.align(align)),
            Self::Column(c) => Self::Column(c.align(align)),
        }
    }

    fn justify(self, j: Justify) -> Self {
        match self {
            Self::Row(r) => Self::Row(r.justify(j)),
            Self::Column(c) => Self::Column(c.justify(j)),
        }
    }

    fn reverse(self, r: bool) -> Self {
        match self {
            Self::Row(row) => Self::Row(row.reverse(r)),
            Self::Column(col) => Self::Column(col.reverse(r)),
        }
    }

    fn width(self, w: impl Into<Length>) -> Self {
        let w = w.into();
        match self {
            Self::Row(r) => Self::Row(r.width(w)),
            Self::Column(c) => Self::Column(c.width(w)),
        }
    }

    fn height(self, h: impl Into<Length>) -> Self {
        let h = h.into();
        match self {
            Self::Row(r) => Self::Row(r.height(h)),
            Self::Column(c) => Self::Column(c.height(h)),
        }
    }
}

impl<'a, Message: 'a> From<ContainerBuilder<'a, Message>>
    for Element<'a, Message>
{
    fn from(b: ContainerBuilder<'a, Message>) -> Self {
        match b {
            ContainerBuilder::Row(r) => r.into(),
            ContainerBuilder::Column(c) => c.into(),
        }
    }
}

/// A labelled cell with a fixed cross-axis size. The `letter` is the
/// block's primary identity (A, B, C, …); `note` is an optional
/// feature annotation rendered as a secondary line beneath. Pass `""`
/// for `note` when the block has no distinguishing flex feature.
fn cell<'a, Message: 'a>(
    letter: &'a str,
    note: &'a str,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    // The note inherits the cell container's text_color
    // (palette.primary.weak.text — readable by construction per
    // iced's Pair). A small alpha dim gives visual hierarchy
    // without breaking contrast against any theme.
    let body: Element<'_, Message> = if note.is_empty() {
        text(letter).size(13).into()
    } else {
        column![
            text(letter).size(13),
            text(note).size(10).style(|theme: &Theme| text::Style {
                color: Some(
                    theme.palette().primary.weak.text.scale_alpha(0.75),
                ),
            }),
        ]
        .spacing(1)
        .into()
    };

    let mut c = container(body).padding([6.0, 10.0]).style(cell_style);

    if width > 0.0 {
        c = c.width(width);
    }

    if height > 0.0 {
        c = c.height(height);
    } else {
        c = c.height(Shrink);
    }

    c.into()
}

/// Semantic accent picked from the active theme's palette.
///
/// `boxed()` takes one of these so demos never hardcode RGB —
/// the actual hex resolves at draw time from `theme.palette()`.
#[derive(Debug, Clone, Copy)]
enum Accent {
    Primary,
    Success,
    Warning,
    Danger,
}

impl Accent {
    fn color(self, palette: &iced::theme::Palette) -> iced::Color {
        match self {
            Accent::Primary => palette.primary.base.color,
            Accent::Success => palette.success.base.color,
            Accent::Warning => palette.warning.base.color,
            Accent::Danger => palette.danger.base.color,
        }
    }
}

/// A flexible coloured box used by grow/shrink/basis demos. The
/// `letter` is the block's primary identity (A, B, C, …); `note` is
/// the feature annotation (e.g. `grow=1`, `basis=80`). Sized
/// `Fill x Fill` so the flex container can resize it freely. The
/// hue resolves from the active theme's palette via [`Accent`].
fn boxed<'a, Message: 'a>(
    letter: &'a str,
    note: &'a str,
    accent: Accent,
) -> Element<'a, Message> {
    // Note inherits the box's text_color (palette.background.base.text
    // — readable on the tinted accent background). Dimmed slightly for
    // visual hierarchy without a hardcoded color.
    container(
        column![
            text(letter).size(14),
            text(note).size(11).style(|theme: &Theme| text::Style {
                color: Some(
                    theme.palette().background.base.text.scale_alpha(0.75),
                ),
            }),
        ]
        .spacing(2),
    )
    .padding(10)
    .width(Fill)
    .height(Fill)
    .style(move |theme: &Theme| boxed_style(theme, accent))
    .into()
}

// --- Sidebar --------------------------------------------------------------

fn sidebar(app: &FlexTour) -> Element<'_, Message> {
    let label = |s: &'static str| text(s).size(11).style(text::secondary);

    let demo_picker = pick_list(Some(app.demo), Demo::ALL, Demo::to_string)
        .on_select(Message::DemoSelected)
        .text_size(12)
        .width(Fill);

    let axis_picker =
        pick_list(Some(app.axis), AxisChoice::ALL, AxisChoice::to_string)
            .on_select(Message::AxisSelected)
            .text_size(12)
            .width(Fill);

    let justify_picker = pick_list(
        Some(app.justify_override),
        JustifyChoice::ALL,
        JustifyChoice::to_string,
    )
    .on_select(Message::JustifySelected)
    .text_size(12)
    .width(Fill);

    let align_picker = pick_list(
        Some(app.align_override),
        AlignChoice::ALL,
        AlignChoice::to_string,
    )
    .on_select(Message::AlignSelected)
    .text_size(12)
    .width(Fill);

    // Dogfood: each slider readout is `flex::row![label, hotkey,
    // value.grow(1).align(Right)]` — the label and hotkey hint pack
    // at the start, the value text grows into the leftover and
    // right-aligns itself within that space.
    let gap_slider = column![
        flex::row![
            label("gap"),
            hotkey("← →"),
            flex(
                text(format!("{:.0}px", app.gap))
                    .size(11)
                    .align_x(iced::Right)
            )
            .grow(1.0),
        ]
        .align(AlignItems::Center)
        .width(Fill),
        slider(0.0..=48.0, app.gap, Message::GapChanged)
            .step(1.0)
            .style(slider_style),
    ]
    .spacing(4);

    let padding_slider = column![
        flex::row![
            label("padding"),
            hotkey("↑ ↓"),
            flex(
                text(format!("{:.0}px", app.padding))
                    .size(11)
                    .align_x(iced::Right)
            )
            .grow(1.0),
        ]
        .align(AlignItems::Center)
        .width(Fill),
        slider(0.0..=32.0, app.padding, Message::PaddingChanged)
            .step(1.0)
            .style(slider_style),
    ]
    .spacing(4);

    let reverse_box: Element<'_, Message> = flex::row![
        checkbox(app.reverse)
            .label("reverse")
            .text_size(11)
            .on_toggle(Message::ReverseToggled),
        hotkey("V"),
    ]
    .justify(Justify::SpaceBetween)
    .align(AlignItems::Center)
    .width(Fill)
    .into();

    let body = column![
        text("Flex tour").size(20),
        text("CSS Flexbox for iced").size(11).style(text::secondary),
        section("demo", "PgUp / PgDn", demo_picker.into()),
        section("axis", "X · R / C", axis_picker.into()),
        section("justify-content", "J / ⇧J", justify_picker.into()),
        section("align-items", "A / ⇧A", align_picker.into()),
        gap_slider,
        padding_slider,
        reverse_box,
    ]
    .spacing(14)
    .padding(20)
    .width(280.0);

    container(scrollable(body)).width(280.0).height(Fill).into()
}

fn section<'a>(
    title: &'static str,
    hotkey_hint: &'static str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        flex::row![
            text(title).size(11).style(text::secondary),
            hotkey(hotkey_hint),
        ]
        .justify(Justify::SpaceBetween)
        .align(AlignItems::Center)
        .width(Fill),
        body,
    ]
    .spacing(4)
    .into()
}

/// Tiny dimmed label for keyboard-shortcut hints next to section
/// titles. Smaller and more muted than the secondary text around it
/// so the hotkey reads as ancillary metadata rather than a peer of
/// the section title.
fn hotkey<'a>(s: &'a str) -> iced::widget::Text<'a, Theme> {
    text(s).size(10).style(|theme: &Theme| text::Style {
        color: Some(theme.palette().background.base.text.scale_alpha(0.45)),
    })
}

// --- Styling --------------------------------------------------------------

fn canvas_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.weakest.color.into()),
        ..container::Style::default()
    }
}

fn frame_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

fn slider_style(
    theme: &Theme,
    status: iced::widget::slider::Status,
) -> iced::widget::slider::Style {
    use iced::widget::slider::{Handle, HandleShape, Rail, Status, Style};

    let palette = theme.palette();

    // Filled portion uses primary; the unfilled tail blends quietly
    // into the sidebar's background. Hovered/dragged states bump the
    // fill brightness without changing geometry.
    let fill = match status {
        Status::Active => palette.primary.base.color,
        Status::Hovered => palette.primary.strong.color,
        Status::Dragged => palette.primary.strong.color,
    };
    let track = palette.background.weaker.color;

    Style {
        rail: Rail {
            backgrounds: (fill.into(), track.into()),
            width: 1.0,
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 0.5.into(),
            },
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 5.0 },
            background: fill.into(),
            border_color: palette.background.base.color,
            border_width: 1.5,
        },
    }
}

fn track_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.weakest.color.into()),
        border: iced::Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn cell_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.primary.weak.color.into()),
        text_color: Some(palette.primary.weak.text),
        border: iced::Border {
            color: palette.primary.base.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn boxed_style(theme: &Theme, accent: Accent) -> container::Style {
    let palette = theme.palette();
    let color = accent.color(palette);
    let tint = color.scale_alpha(if palette.is_dark { 0.35 } else { 0.55 });

    container::Style {
        background: Some(tint.into()),
        text_color: Some(palette.background.base.text),
        border: iced::Border {
            color,
            width: 1.5,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn header_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.primary.base.color.into()),
        text_color: Some(palette.primary.base.text),
        border: iced::Border {
            color: palette.primary.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn footer_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn sidebar_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

fn panel_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.weakest.color.into()),
        border: iced::Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}
