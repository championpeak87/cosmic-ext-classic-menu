// SPDX-License-Identifier: GPL-3.0-only

//! The menu as blocks, each with options of its own.
//!
//! A [`Layout`] stacks [`Row`]s, each holding [`Column`]s side by side, which
//! stack [`Block`]s. Rows holding a block that fills the height, such as the
//! app list, share the menu's height; the others take what they need.
//! Columns holding a block that fills the width share the row's width by
//! their [`Column::portion`]; the others take what they need.
//!
//! The layouts to pick from in the settings, see [`MenuLayout`], are
//! presets of blocks.

use serde::{Deserialize, Serialize};

use crate::config::{AppView, FeaturedApplications, ItemSize, MenuLayout};
use crate::model::application_category::ApplicationCategory;
use crate::model::place::Place;
use crate::model::power_action::PowerAction;

/// The blocks of the menu, see the [module docs](self).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub rows: Vec<Row>,
}

impl Default for Layout {
    fn default() -> Self {
        MenuLayout::default().layout()
    }
}

/// Columns side by side.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Row {
    pub columns: Vec<Column>,
    /// Whether the row is a colored bar.
    pub banner: bool,
}

/// Blocks stacked on top of each other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Column {
    pub blocks: Vec<Block>,
    /// Share of the row's width, if the column fills it, relative to the
    /// other columns filling it.
    pub portion: u16,
    /// Padding of the block at the same index of `blocks`. Blocks past its
    /// end have none, so configs without it keep their look; see
    /// [`Column::padding`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paddings: Vec<BlockPadding>,
}

impl Default for Column {
    fn default() -> Self {
        Column {
            blocks: Vec::new(),
            portion: Column::DEFAULT_PORTION,
            paddings: Vec::new(),
        }
    }
}

/// Room around a block, in pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct BlockPadding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl BlockPadding {
    pub const ZERO: BlockPadding = BlockPadding {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };
    /// Bounds of each side.
    pub const RANGE: std::ops::RangeInclusive<u16> = 0..=64;

    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }

    /// With each side within [`BlockPadding::RANGE`].
    pub fn clamped(self) -> Self {
        let clamp = |side: u16| side.clamp(*Self::RANGE.start(), *Self::RANGE.end());
        BlockPadding {
            top: clamp(self.top),
            right: clamp(self.right),
            bottom: clamp(self.bottom),
            left: clamp(self.left),
        }
    }
}

impl Column {
    pub const DEFAULT_PORTION: u16 = 50;
    /// Smallest share of the width two columns split between them that
    /// either of them keeps when resizing, in percent.
    pub const MIN_SHARE: u16 = 20;

    fn of(portion: u16, blocks: Vec<Block>) -> Self {
        Column {
            blocks,
            portion,
            paddings: Vec::new(),
        }
    }

    /// Padding of the block at `index`, within [`BlockPadding::RANGE`].
    pub fn padding(&self, index: usize) -> BlockPadding {
        self.paddings
            .get(index)
            .copied()
            .unwrap_or_default()
            .clamped()
    }

    /// Sets the padding of the block at `index`. The paddings stop at the
    /// last block having one, so a column without any compares equal to
    /// one that never had them, such as a preset's.
    fn set_padding(&mut self, index: usize, padding: BlockPadding) {
        if index >= self.paddings.len() {
            if padding.is_zero() {
                return;
            }
            self.paddings.resize(index + 1, BlockPadding::ZERO);
        }
        self.paddings[index] = padding;
        while self.paddings.last().is_some_and(|padding| padding.is_zero()) {
            self.paddings.pop();
        }
    }

    /// Whether a block makes the column take a share of the row's width.
    pub fn fills_width(&self) -> bool {
        self.blocks.iter().any(Block::fills_width)
    }
}

impl Row {
    /// Each of `blocks` in a column of its own.
    fn line(blocks: Vec<Block>) -> Self {
        Row {
            columns: blocks
                .into_iter()
                .map(|block| Column::of(Column::DEFAULT_PORTION, vec![block]))
                .collect(),
            banner: false,
        }
    }

    fn banner(blocks: Vec<Block>) -> Self {
        Row {
            banner: true,
            ..Row::line(blocks)
        }
    }

    fn of(columns: Vec<Column>) -> Self {
        Row {
            columns,
            banner: false,
        }
    }

    /// Whether a block makes the row take a share of the menu's height.
    pub fn fills_height(&self) -> bool {
        self.columns
            .iter()
            .flat_map(|column| &column.blocks)
            .any(Block::fills_height)
    }
}

/// A part of the menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    /// Picture and name of the logged in user.
    User(UserOptions),
    Search(SearchOptions),
    /// The apps, see [`AppListOptions`].
    AppList(AppListOptions),
    /// Categories filtering the app list, or places.
    SidePanel(SidePanelOptions),
    Power(PowerOptions),
    /// Favorite or recently used apps, in a bar of their own.
    Featured(FeaturedOptions),
    /// Empty room, pushing the blocks next to it apart.
    Spacer,
    /// A line between blocks.
    Divider,
}

/// What a [`Block`] is, regardless of its options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockKind {
    User,
    Search,
    AppList,
    SidePanel,
    Power,
    Featured,
    Spacer,
    Divider,
}

impl BlockKind {
    pub const ALL: [BlockKind; 8] = [
        BlockKind::User,
        BlockKind::Search,
        BlockKind::AppList,
        BlockKind::SidePanel,
        BlockKind::Power,
        BlockKind::Featured,
        BlockKind::Spacer,
        BlockKind::Divider,
    ];

    /// Whether the menu holds at most one block of this kind. They all show
    /// the same state, such as the search, so a second one would add
    /// nothing.
    pub fn is_unique(self) -> bool {
        matches!(self, BlockKind::User | BlockKind::Search | BlockKind::AppList)
    }

    /// Whether the menu must keep a block of this kind.
    pub fn is_required(self) -> bool {
        self == BlockKind::AppList
    }

    /// A new block of this kind.
    pub fn block(self) -> Block {
        match self {
            BlockKind::User => Block::User(UserOptions::default()),
            BlockKind::Search => Block::Search(SearchOptions::default()),
            BlockKind::AppList => Block::AppList(AppListOptions::default()),
            BlockKind::SidePanel => Block::SidePanel(SidePanelOptions::default()),
            BlockKind::Power => Block::Power(PowerOptions::default()),
            BlockKind::Featured => Block::Featured(FeaturedOptions::default()),
            BlockKind::Spacer => Block::Spacer,
            BlockKind::Divider => Block::Divider,
        }
    }
}

impl Block {
    pub fn kind(&self) -> BlockKind {
        match self {
            Block::User(_) => BlockKind::User,
            Block::Search(_) => BlockKind::Search,
            Block::AppList(_) => BlockKind::AppList,
            Block::SidePanel(_) => BlockKind::SidePanel,
            Block::Power(_) => BlockKind::Power,
            Block::Featured(_) => BlockKind::Featured,
            Block::Spacer => BlockKind::Spacer,
            Block::Divider => BlockKind::Divider,
        }
    }

    /// Whether the block takes all the width it gets.
    pub fn fills_width(&self) -> bool {
        match self {
            Block::Search(_)
            | Block::AppList(_)
            | Block::SidePanel(_)
            | Block::Spacer
            | Block::Divider => true,
            Block::Featured(options) => !options.vertical,
            Block::User(_) | Block::Power(_) => false,
        }
    }

    /// Whether the block takes all the height it gets.
    pub fn fills_height(&self) -> bool {
        match self {
            Block::AppList(_) | Block::SidePanel(_) => true,
            Block::Featured(options) => options.vertical,
            _ => false,
        }
    }
}

/// How the user's name is shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserName {
    /// The login name.
    #[default]
    Username,
    /// The full name, with the login name below, or the login name alone if
    /// no full name is set.
    FullName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserOptions {
    pub name: UserName,
    pub show_avatar: bool,
}

impl Default for UserOptions {
    fn default() -> Self {
        UserOptions {
            name: UserName::default(),
            show_avatar: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    /// Whether results of pop-launcher's plugins, such as the calculator or
    /// open windows, are shown above the apps found.
    pub launcher_results: bool,
    /// Height and text size of the field.
    pub size: ItemSize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            launcher_results: true,
            size: ItemSize::default(),
        }
    }
}

/// Options of the app list.
///
/// Favorite and recently used apps can lead the list of all apps, or be
/// shown on a start page of their own, leading to all apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppListOptions {
    pub view: AppView,
    pub item_size: ItemSize,
    /// Below the names in the column view, on hover in the grid view.
    pub show_description: bool,
    /// Whether "All applications" heads the list.
    pub show_heading: bool,
    /// Whether the first favorites lead the apps.
    pub favorites: bool,
    pub favorites_count: u16,
    /// Whether the most used apps lead the apps, after the favorites.
    pub recents: bool,
    pub recents_count: u16,
    /// Whether the favorite and recently used apps are shown on a page of
    /// their own, leading to all apps, rather than at the top of them.
    pub start_page: bool,
}

impl Default for AppListOptions {
    fn default() -> Self {
        AppListOptions {
            view: AppView::Column,
            item_size: ItemSize::default(),
            show_description: true,
            show_heading: false,
            favorites: false,
            favorites_count: 3,
            recents: false,
            recents_count: 5,
            start_page: false,
        }
    }
}

impl AppListOptions {
    /// Bounds of `favorites_count` and `recents_count`.
    pub const COUNT_RANGE: std::ops::RangeInclusive<u16> = 1..=15;

    /// How many favorites lead the apps.
    pub fn favorites_shown(&self) -> usize {
        if self.favorites {
            usize::from(Self::clamp_count(self.favorites_count))
        } else {
            0
        }
    }

    /// How many recently used apps lead the apps.
    pub fn recents_shown(&self) -> usize {
        if self.recents {
            usize::from(Self::clamp_count(self.recents_count))
        } else {
            0
        }
    }

    /// Whether favorite or recently used apps lead the apps.
    pub fn has_home(&self) -> bool {
        self.favorites || self.recents
    }

    /// Whether they do so on a page of their own.
    pub fn has_start_page(&self) -> bool {
        self.start_page && self.has_home()
    }

    pub fn clamp_count(count: u16) -> u16 {
        count.clamp(*Self::COUNT_RANGE.start(), *Self::COUNT_RANGE.end())
    }
}

/// What a side panel shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidePanelContent {
    /// App categories filtering the app list.
    #[default]
    Categories,
    /// Shortcuts to folders and system tools.
    Places,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidePanelOptions {
    pub content: SidePanelContent,
    /// Places shown, in order, with [`SidePanelContent::Places`].
    pub places: Vec<Place>,
    /// Whether the categories include the most used apps.
    pub show_recently_used: bool,
    /// Whether the categories include the favorites.
    pub show_favorites: bool,
    /// Id of the category the menu opens on.
    pub default_category: String,
    /// Whether the panel hides while searching, leaving the room to the
    /// results.
    pub hide_while_searching: bool,
    /// Whether the panel has a background of its own.
    pub tinted: bool,
}

impl Default for SidePanelOptions {
    fn default() -> Self {
        SidePanelOptions {
            content: SidePanelContent::default(),
            places: Place::DEFAULT.to_vec(),
            show_recently_used: true,
            show_favorites: false,
            default_category: ApplicationCategory::ALL.id().to_owned(),
            hide_while_searching: false,
            tinted: false,
        }
    }
}

impl SidePanelOptions {
    fn places(tinted: bool) -> Self {
        SidePanelOptions {
            content: SidePanelContent::Places,
            tinted,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerOptions {
    /// Actions shown, in order.
    pub actions: Vec<PowerAction>,
    /// Whether the buttons are stacked rather than side by side.
    pub vertical: bool,
    /// Whether hovering a button shows the name of its action.
    pub show_tooltips: bool,
}

impl Default for PowerOptions {
    fn default() -> Self {
        PowerOptions {
            actions: PowerAction::DEFAULT.to_vec(),
            vertical: false,
            show_tooltips: false,
        }
    }
}

/// Options of a bar of favorite or recently used apps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeaturedOptions {
    pub source: FeaturedApplications,
    /// A column of icons, rather than a row of icons with names.
    pub vertical: bool,
    /// How many apps are shown at most.
    pub count: u16,
    /// Whether a heading names the apps shown, in a row.
    pub show_heading: bool,
    /// Whether the bar hides while searching.
    pub hide_while_searching: bool,
}

impl Default for FeaturedOptions {
    fn default() -> Self {
        FeaturedOptions {
            source: FeaturedApplications::Favorites,
            vertical: false,
            count: 4,
            show_heading: true,
            hide_while_searching: true,
        }
    }
}

impl FeaturedOptions {
    /// Bounds of `count`.
    pub const COUNT_RANGE: std::ops::RangeInclusive<u16> = 1..=15;

    pub fn count(&self) -> usize {
        usize::from(
            self.count
                .clamp(*Self::COUNT_RANGE.start(), *Self::COUNT_RANGE.end()),
        )
    }
}

/// Where a block is in a [`Layout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPath {
    pub row: usize,
    pub column: usize,
    pub block: usize,
}

/// Which side of a block another one is dropped on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// In the same column, above it.
    Above,
    /// In the same column, below it.
    Below,
    /// In a new column, to its left.
    Before,
    /// In a new column, to its right.
    After,
}

/// Where a block is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Drop {
    /// Next to the block at `path`.
    Beside { path: BlockPath, side: Side },
    /// In a new row at this index.
    NewRow(usize),
}

impl Layout {
    pub fn get(&self, path: BlockPath) -> Option<&Block> {
        self.rows
            .get(path.row)?
            .columns
            .get(path.column)?
            .blocks
            .get(path.block)
    }

    pub fn get_mut(&mut self, path: BlockPath) -> Option<&mut Block> {
        self.rows
            .get_mut(path.row)?
            .columns
            .get_mut(path.column)?
            .blocks
            .get_mut(path.block)
    }

    /// Padding of the block at `path`.
    pub fn padding(&self, path: BlockPath) -> Option<BlockPadding> {
        let column = self.rows.get(path.row)?.columns.get(path.column)?;
        (path.block < column.blocks.len()).then(|| column.padding(path.block))
    }

    /// Sets the padding of the block at `path`, if there is one.
    pub fn set_padding(&mut self, path: BlockPath, padding: BlockPadding) {
        let Some(column) = self
            .rows
            .get_mut(path.row)
            .and_then(|row| row.columns.get_mut(path.column))
        else {
            return;
        };
        if path.block < column.blocks.len() {
            column.set_padding(path.block, padding.clamped());
        }
    }

    /// Every block, with where it is.
    pub fn blocks(&self) -> impl Iterator<Item = (BlockPath, &Block)> {
        self.rows.iter().enumerate().flat_map(|(row, r)| {
            r.columns.iter().enumerate().flat_map(move |(column, c)| {
                c.blocks.iter().enumerate().map(move |(block, b)| {
                    (BlockPath { row, column, block }, b)
                })
            })
        })
    }

    pub fn contains(&self, kind: BlockKind) -> bool {
        self.blocks().any(|(_, block)| block.kind() == kind)
    }

    /// Options of the app list, if there is one.
    pub fn app_list(&self) -> Option<&AppListOptions> {
        self.blocks().find_map(|(_, block)| match block {
            Block::AppList(options) => Some(options),
            _ => None,
        })
    }

    pub fn app_list_mut(&mut self) -> Option<&mut AppListOptions> {
        self.rows
            .iter_mut()
            .flat_map(|row| &mut row.columns)
            .flat_map(|column| &mut column.blocks)
            .find_map(|block| match block {
                Block::AppList(options) => Some(options),
                _ => None,
            })
    }

    pub fn search(&self) -> Option<&SearchOptions> {
        self.blocks().find_map(|(_, block)| match block {
            Block::Search(options) => Some(options),
            _ => None,
        })
    }

    /// The side panels, in order.
    pub fn side_panels(&self) -> impl Iterator<Item = &SidePanelOptions> {
        self.blocks().filter_map(|(_, block)| match block {
            Block::SidePanel(options) => Some(options),
            _ => None,
        })
    }

    /// The first side panel showing categories, which tells which one the
    /// menu opens on.
    pub fn categories(&self) -> Option<&SidePanelOptions> {
        self.side_panels()
            .find(|options| options.content == SidePanelContent::Categories)
    }

    /// Whether favorites are shown anywhere, so apps can be added to them.
    pub fn shows_favorites(&self) -> bool {
        self.blocks().any(|(_, block)| match block {
            Block::AppList(options) => options.favorites,
            Block::SidePanel(options) => {
                options.content == SidePanelContent::Categories && options.show_favorites
            }
            Block::Featured(options) => options.source == FeaturedApplications::Favorites,
            _ => false,
        })
    }

    /// Whether favorites are shown limited to the first ones, so the ones
    /// added go first to be seen.
    pub fn shows_first_favorites(&self) -> bool {
        self.app_list().is_some_and(|options| options.favorites)
    }

    /// Puts `block` where `drop` says. Returns where it ends up.
    pub fn insert(&mut self, block: Block, drop: Drop) -> Option<BlockPath> {
        let mut work = Work::of(self);
        work.place(
            Marked {
                block,
                padding: BlockPadding::ZERO,
                moved: true,
            },
            drop,
        )?;
        Some(work.finish(self))
    }

    /// Moves the block at `from` to where `drop` says. Returns where it
    /// ends up.
    pub fn move_block(&mut self, from: BlockPath, drop: Drop) -> Option<BlockPath> {
        // Dropped on itself, it stays where it is.
        if let Drop::Beside { path, .. } = drop
            && path == from
        {
            return self.get(from).is_some().then_some(from);
        }
        let mut work = Work::of(self);
        let mut block = work.take(from)?;
        block.moved = true;
        work.place(block, drop)?;
        Some(work.finish(self))
    }

    /// Removes the block at `from`, and the column and row left empty.
    pub fn remove(&mut self, from: BlockPath) -> Option<Block> {
        let mut work = Work::of(self);
        let block = work.take(from)?;
        work.finish(self);
        Some(block.block)
    }

    /// Splits the width of the columns at `left` and `left + 1` of `row`
    /// between them, giving `share` percent of it to the left one.
    pub fn resize_columns(&mut self, row: usize, left: usize, share: u16) {
        let Some(row) = self.rows.get_mut(row) else {
            return;
        };
        if left + 1 >= row.columns.len() {
            return;
        }
        let total = row.columns[left].portion + row.columns[left + 1].portion;
        let share = share.clamp(Column::MIN_SHARE, 100 - Column::MIN_SHARE);
        let portion = (u32::from(total) * u32::from(share) / 100) as u16;
        row.columns[left].portion = portion.max(1);
        row.columns[left + 1].portion = (total - portion).max(1);
    }
}

/// A block of a layout being rearranged, see [`Work`].
struct Marked {
    block: Block,
    /// Its padding, which moves with it.
    padding: BlockPadding,
    /// Whether it is the block being moved or inserted, to tell where it
    /// ends up.
    moved: bool,
}

/// A layout being rearranged. Taking a block out leaves its place empty,
/// so the paths of the others stay valid until [`Work::finish`].
struct Work {
    rows: Vec<(bool, Vec<(u16, Vec<Option<Marked>>)>)>,
}

impl Work {
    fn of(layout: &Layout) -> Self {
        Work {
            rows: layout
                .rows
                .iter()
                .map(|row| {
                    let columns = row
                        .columns
                        .iter()
                        .map(|column| {
                            let blocks = column
                                .blocks
                                .iter()
                                .enumerate()
                                .map(|(index, block)| {
                                    Some(Marked {
                                        block: block.clone(),
                                        padding: column.padding(index),
                                        moved: false,
                                    })
                                })
                                .collect();
                            (column.portion, blocks)
                        })
                        .collect();
                    (row.banner, columns)
                })
                .collect(),
        }
    }

    fn take(&mut self, path: BlockPath) -> Option<Marked> {
        self.rows
            .get_mut(path.row)?
            .1
            .get_mut(path.column)?
            .1
            .get_mut(path.block)?
            .take()
    }

    fn place(&mut self, block: Marked, drop: Drop) -> Option<()> {
        match drop {
            Drop::Beside { path, side } => {
                let row = &mut self.rows.get_mut(path.row)?.1;
                let blocks = &mut row.get_mut(path.column)?.1;
                if path.block >= blocks.len() {
                    return None;
                }
                match side {
                    Side::Above => blocks.insert(path.block, Some(block)),
                    Side::Below => blocks.insert(path.block + 1, Some(block)),
                    Side::Before | Side::After => {
                        let portion = Self::new_portion(row);
                        let at = path.column + usize::from(side == Side::After);
                        row.insert(at, (portion, vec![Some(block)]));
                    }
                }
            }
            Drop::NewRow(index) => {
                if index > self.rows.len() {
                    return None;
                }
                self.rows.insert(
                    index,
                    (false, vec![(Column::DEFAULT_PORTION, vec![Some(block)])]),
                );
            }
        }
        Some(())
    }

    /// Portion of a column added to `row`: the average of the others, so it
    /// gets a fair share.
    fn new_portion(row: &[(u16, Vec<Option<Marked>>)]) -> u16 {
        if row.is_empty() {
            return Column::DEFAULT_PORTION;
        }
        let total: u32 = row.iter().map(|(portion, _)| u32::from(*portion)).sum();
        (total / row.len() as u32).max(1) as u16
    }

    /// Writes the blocks to `layout`, dropping the empty places, columns and
    /// rows. Returns where the moved block ended up.
    fn finish(self, layout: &mut Layout) -> BlockPath {
        let mut moved = BlockPath {
            row: 0,
            column: 0,
            block: 0,
        };
        let mut rows = Vec::new();
        for (banner, columns) in self.rows {
            let mut row = Row {
                columns: Vec::new(),
                banner,
            };
            for (portion, blocks) in columns {
                let mut column = Column::of(portion, Vec::new());
                for block in blocks.into_iter().flatten() {
                    if block.moved {
                        moved = BlockPath {
                            row: rows.len(),
                            column: row.columns.len(),
                            block: column.blocks.len(),
                        };
                    }
                    column.set_padding(column.blocks.len(), block.padding);
                    column.blocks.push(block.block);
                }
                if !column.blocks.is_empty() {
                    row.columns.push(column);
                }
            }
            if !row.columns.is_empty() {
                rows.push(row);
            }
        }
        layout.rows = rows;
        moved
    }
}

impl MenuLayout {
    /// The blocks of this preset.
    pub fn layout(self) -> Layout {
        let user = || Block::User(UserOptions::default());
        let search = || Block::Search(SearchOptions::default());
        let app_list = || Block::AppList(AppListOptions::default());
        let categories = || Block::SidePanel(SidePanelOptions::default());
        let power = || Block::Power(PowerOptions::default());

        let rows = match self {
            MenuLayout::Classic => vec![
                Row::line(vec![user()]),
                Row::line(vec![search()]),
                Row::of(vec![
                    Column::of(62, vec![app_list()]),
                    Column::of(38, vec![categories(), power()]),
                ]),
            ],
            MenuLayout::Retro => vec![
                Row::banner(vec![user()]),
                Row::of(vec![
                    Column::of(62, vec![app_list()]),
                    Column::of(38, vec![Block::SidePanel(SidePanelOptions::places(true))]),
                ]),
                Row::line(vec![search()]),
                Row::banner(vec![Block::Spacer, power()]),
            ],
            MenuLayout::Compact => vec![
                Row::line(vec![user(), search()]),
                Row::of(vec![
                    Column::of(38, vec![categories()]),
                    Column::of(62, vec![app_list()]),
                ]),
                Row::line(vec![Block::Spacer, power()]),
            ],
            MenuLayout::Cinnamon => vec![
                Row::line(vec![user()]),
                Row::line(vec![search()]),
                Row::of(vec![
                    Column::of(
                        Column::DEFAULT_PORTION,
                        vec![
                            Block::Featured(FeaturedOptions {
                                source: FeaturedApplications::Favorites,
                                vertical: true,
                                count: *FeaturedOptions::COUNT_RANGE.end(),
                                show_heading: false,
                                hide_while_searching: false,
                            }),
                            Block::Power(PowerOptions {
                                vertical: true,
                                ..Default::default()
                            }),
                        ],
                    ),
                    Column::of(38, vec![categories()]),
                    Column::of(62, vec![app_list()]),
                ]),
            ],
            MenuLayout::Modern => vec![
                Row::line(vec![search()]),
                Row::of(vec![Column::of(
                    Column::DEFAULT_PORTION,
                    vec![
                        Block::AppList(AppListOptions {
                            view: AppView::Grid,
                            show_description: false,
                            show_heading: true,
                            ..Default::default()
                        }),
                        Block::Featured(FeaturedOptions {
                            source: FeaturedApplications::RecentlyUsed,
                            ..Default::default()
                        }),
                    ],
                )]),
                Row::line(vec![Block::Divider]),
                Row::line(vec![user(), Block::Spacer, power()]),
            ],
            MenuLayout::Whistler => vec![
                Row::banner(vec![user()]),
                Row::of(vec![
                    Column::of(
                        62,
                        vec![Block::AppList(AppListOptions {
                            favorites: true,
                            favorites_count: 2,
                            recents: true,
                            recents_count: 8,
                            start_page: true,
                            ..Default::default()
                        })],
                    ),
                    Column::of(38, vec![Block::SidePanel(SidePanelOptions::places(true))]),
                ]),
                Row::line(vec![search()]),
                Row::banner(vec![Block::Spacer, power()]),
            ],
        };
        Layout { rows }
    }

    /// The preset `layout` is, if any.
    pub fn matching(layout: &Layout) -> Option<MenuLayout> {
        MenuLayout::ALL
            .into_iter()
            .find(|preset| preset.layout() == *layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(row: usize, column: usize, block: usize) -> BlockPath {
        BlockPath { row, column, block }
    }

    fn kinds(layout: &Layout) -> Vec<Vec<Vec<BlockKind>>> {
        layout
            .rows
            .iter()
            .map(|row| {
                row.columns
                    .iter()
                    .map(|column| column.blocks.iter().map(Block::kind).collect())
                    .collect()
            })
            .collect()
    }

    use BlockKind::*;

    #[test]
    fn presets_hold_one_app_list_and_tell_themselves_apart() {
        for preset in MenuLayout::ALL {
            let layout = preset.layout();
            let app_lists = layout
                .blocks()
                .filter(|(_, block)| block.kind() == AppList)
                .count();
            assert_eq!(app_lists, 1, "{preset:?}");
            assert_eq!(MenuLayout::matching(&layout), Some(preset));
        }
    }

    #[test]
    fn rows_fill_the_height_only_with_a_block_that_does() {
        let layout = MenuLayout::Classic.layout();
        let fills: Vec<bool> = layout.rows.iter().map(Row::fills_height).collect();
        assert_eq!(fills, [false, false, true]);
    }

    #[test]
    fn moving_a_block_above_another_stacks_them() {
        let mut layout = MenuLayout::Classic.layout();
        // The search field above the categories.
        let moved = layout.move_block(
            path(1, 0, 0),
            Drop::Beside {
                path: path(2, 1, 0),
                side: Side::Above,
            },
        );

        // Its row was left empty and is gone.
        assert_eq!(kinds(&layout), [vec![vec![User]], vec![vec![AppList], vec![Search, SidePanel, Power]]]);
        assert_eq!(moved, Some(path(1, 1, 0)));
    }

    #[test]
    fn moving_a_block_within_its_column_keeps_the_others() {
        let mut layout = MenuLayout::Classic.layout();
        let moved = layout.move_block(
            path(2, 1, 1),
            Drop::Beside {
                path: path(2, 1, 0),
                side: Side::Above,
            },
        );

        assert_eq!(kinds(&layout)[2], [vec![AppList], vec![Power, SidePanel]]);
        assert_eq!(moved, Some(path(2, 1, 0)));
    }

    #[test]
    fn dropping_beside_a_block_adds_a_column() {
        let mut layout = MenuLayout::Classic.layout();
        let moved = layout.move_block(
            path(0, 0, 0),
            Drop::Beside {
                path: path(1, 0, 0),
                side: Side::After,
            },
        );

        assert_eq!(kinds(&layout)[0], [vec![Search], vec![User]]);
        assert_eq!(moved, Some(path(0, 1, 0)));
    }

    #[test]
    fn dropping_between_rows_adds_a_row() {
        let mut layout = MenuLayout::Classic.layout();
        let moved = layout.move_block(path(1, 0, 0), Drop::NewRow(3));

        assert_eq!(kinds(&layout).last().unwrap(), &vec![vec![Search]]);
        assert_eq!(moved, Some(path(2, 0, 0)));
    }

    #[test]
    fn dropping_a_block_on_itself_changes_nothing() {
        let mut layout = MenuLayout::Classic.layout();
        let moved = layout.move_block(
            path(1, 0, 0),
            Drop::Beside {
                path: path(1, 0, 0),
                side: Side::Before,
            },
        );

        assert_eq!(layout, MenuLayout::Classic.layout());
        assert_eq!(moved, Some(path(1, 0, 0)));
    }

    #[test]
    fn new_blocks_go_where_they_are_dropped() {
        let mut layout = MenuLayout::Classic.layout();
        let inserted = layout.insert(
            Block::Spacer,
            Drop::Beside {
                path: path(0, 0, 0),
                side: Side::After,
            },
        );

        assert_eq!(kinds(&layout)[0], [vec![User], vec![Spacer]]);
        assert_eq!(inserted, Some(path(0, 1, 0)));
        assert!(layout.insert(Block::Spacer, Drop::NewRow(99)).is_none());
    }

    #[test]
    fn removing_the_last_block_of_a_row_removes_the_row() {
        let mut layout = MenuLayout::Classic.layout();
        assert!(matches!(layout.remove(path(0, 0, 0)), Some(Block::User(_))));
        assert_eq!(layout.rows.len(), 2);
        assert!(!layout.contains(User));
    }

    #[test]
    fn resizing_columns_keeps_their_total_and_bounds() {
        let mut layout = MenuLayout::Classic.layout();
        layout.resize_columns(2, 0, 70);
        let portions: Vec<u16> = layout.rows[2].columns.iter().map(|c| c.portion).collect();
        assert_eq!(portions, [70, 30]);

        layout.resize_columns(2, 0, 99);
        let portions: Vec<u16> = layout.rows[2].columns.iter().map(|c| c.portion).collect();
        assert_eq!(portions, [80, 20]);
    }

    #[test]
    fn options_missing_from_the_config_get_their_defaults() {
        let options: AppListOptions = ron::from_str("(view: Grid)").unwrap();
        assert_eq!(options.view, AppView::Grid);
        assert_eq!(options.item_size, ItemSize::Medium);
        assert!(options.show_description);
    }

    #[test]
    fn power_buttons_of_older_configs_have_no_tooltips() {
        let options: PowerOptions = ron::from_str("(vertical: true)").unwrap();
        assert!(options.vertical);
        assert!(!options.show_tooltips);
    }

    #[test]
    fn search_fields_of_older_configs_keep_their_size() {
        let options: SearchOptions = ron::from_str("(launcher_results: false)").unwrap();
        assert_eq!(options.size, ItemSize::Medium);
    }

    #[test]
    fn layouts_survive_the_config() {
        for preset in MenuLayout::ALL {
            let layout = preset.layout();
            let text = ron::to_string(&layout).unwrap();
            assert_eq!(ron::from_str::<Layout>(&text).unwrap(), layout);
        }
    }

    #[test]
    fn padding_moves_with_its_block() {
        let mut layout = MenuLayout::Classic.layout();
        let padding = BlockPadding {
            top: 4,
            right: 8,
            bottom: 12,
            left: 16,
        };
        layout.set_padding(path(2, 1, 1), padding);
        assert_eq!(layout.padding(path(2, 1, 1)), Some(padding));

        // The power buttons above the categories.
        layout.move_block(
            path(2, 1, 1),
            Drop::Beside {
                path: path(2, 1, 0),
                side: Side::Above,
            },
        );
        assert_eq!(layout.padding(path(2, 1, 0)), Some(padding));
        assert_eq!(layout.padding(path(2, 1, 1)), Some(BlockPadding::ZERO));

        // Removing the block above shifts the padding along with the block.
        layout.remove(path(1, 0, 0));
        assert_eq!(layout.padding(path(1, 1, 0)), Some(padding));
    }

    #[test]
    fn padding_set_back_to_none_matches_the_preset_again() {
        let mut layout = MenuLayout::Classic.layout();
        let padding = BlockPadding {
            top: 6,
            ..BlockPadding::ZERO
        };
        layout.set_padding(path(2, 0, 0), padding);
        assert_eq!(MenuLayout::matching(&layout), None);

        layout.set_padding(path(2, 0, 0), BlockPadding::ZERO);
        assert_eq!(MenuLayout::matching(&layout), Some(MenuLayout::Classic));
    }

    #[test]
    fn padding_stays_within_its_bounds() {
        let mut layout = MenuLayout::Classic.layout();
        layout.set_padding(
            path(0, 0, 0),
            BlockPadding {
                top: 1000,
                ..BlockPadding::ZERO
            },
        );
        assert_eq!(
            layout.padding(path(0, 0, 0)).map(|padding| padding.top),
            Some(*BlockPadding::RANGE.end())
        );
        assert_eq!(layout.padding(path(9, 0, 0)), None);
    }

    #[test]
    fn padding_survives_the_config() {
        let mut layout = MenuLayout::Classic.layout();
        layout.set_padding(
            path(2, 1, 1),
            BlockPadding {
                bottom: 10,
                ..BlockPadding::ZERO
            },
        );
        let text = ron::to_string(&layout).unwrap();
        assert_eq!(ron::from_str::<Layout>(&text).unwrap(), layout);

        // Columns of older configs have none.
        let column: Column = ron::from_str("(blocks: [Spacer], portion: 50)").unwrap();
        assert_eq!(column.padding(0), BlockPadding::ZERO);
    }

    #[test]
    fn favorites_are_shown_where_a_block_shows_them() {
        assert!(!MenuLayout::Classic.layout().shows_favorites());
        assert!(MenuLayout::Cinnamon.layout().shows_favorites());
        assert!(MenuLayout::Whistler.layout().shows_favorites());
        assert!(MenuLayout::Whistler.layout().shows_first_favorites());
    }
}
