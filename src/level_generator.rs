use std::cmp::min;

use crate::level_generator::FieldType::{Corridor, Door, Empty};
use num::{signum, Integer};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

pub struct RoomOptions {
    pub max_rooms: usize,
    pub max_attempts: usize,
    pub min_size: usize,
    pub max_size: usize,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum FieldType {
    Corridor,
    Floor,
    Door,
    Empty,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct Field {
    pub typ: FieldType,
    pub walls: WallInfo,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct WallInfo {
    pub up_left: bool,
    pub up_right: bool,
    pub right_up: bool,
    pub right_down: bool,
    pub down_left: bool,
    pub down_right: bool,
    pub left_up: bool,
    pub left_down: bool,
}

impl Default for WallInfo {
    fn default() -> Self {
        WallInfo {
            up_left: false,
            up_right: false,
            right_up: false,
            right_down: false,
            down_left: false,
            down_right: false,
            left_up: false,
            left_down: false,
        }
    }
}

impl Default for Field {
    fn default() -> Self {
        Field {
            typ: FieldType::Empty,
            walls: WallInfo::default(),
        }
    }
}

impl Default for FieldType {
    fn default() -> Self {
        FieldType::Empty
    }
}

pub struct Level {
    pub rooms: Vec<Vec<(usize, usize)>>,
    pub corridors: Vec<Vec<(usize, usize)>>,
    pub map: Vec<Vec<Field>>,
}

impl Level {
    pub fn width(&self) -> usize {
        self.map.len()
    }

    pub fn height(&self) -> usize {
        self.map[0].len()
    }

    fn init_map(width: usize, height: usize) -> Vec<Vec<Field>> {
        if width.is_even() || height.is_even() {
            panic!(
                "width and height of map must be odd! width: {}, height: {}",
                width, height
            );
        }

        vec![vec![Field::default(); height]; width]
    }

    pub fn create_dungeon(
        width: usize,
        height: usize,
        room_options: RoomOptions,
        room_identifier: FieldType,
    ) -> Self {
        let mut level = Level::create_rooms(
            width,
            height,
            room_options.max_rooms,
            room_options.max_attempts,
            room_options.min_size,
            room_options.max_size,
            room_identifier,
        );

        level.add_maze();

        level.add_doors();

        loop {
            let removed = level.remove_dead_ends();
            if removed == 0 {
                break;
            }
        }

        level
    }

    fn create_rooms(
        width: usize,
        height: usize,
        max_rooms: usize,
        max_attempts: usize,
        min_size: usize,
        max_size: usize,
        room_identifier: FieldType,
    ) -> Self {
        let mut map = Level::init_map(width, height);

        let mut rooms = Vec::new();

        for _ in 0..max_rooms {
            'attempts: for _ in 0..max_attempts {
                let x = gen_odd_range(0, width - 1);
                let x_extent = gen_even_range(min_size, max_size);
                let x_extent = min(x_extent, width - x - 2);

                let y = gen_odd_range(0, height - 1);
                let y_extent = gen_even_range(min_size, max_size);
                let y_extent = min(y_extent, height - y - 2);

                if x_extent < 2 || y_extent < 2 {
                    continue 'attempts;
                }

                // try to place the room...
                for x_check in x..=(x + x_extent) {
                    for y_check in y..=(y + y_extent) {
                        if map[x_check][y_check].typ != Empty {
                            // field is already taken by another room, try again!
                            continue 'attempts;
                        }
                    }
                }

                let mut room_tiles = Vec::new();

                for x_check in x..=(x + x_extent) {
                    for y_check in y..=(y + y_extent) {
                        map[x_check][y_check].typ = room_identifier;
                        room_tiles.push((x_check, y_check));
                    }
                }

                rooms.push(room_tiles);

                // attempt successful. Create the next room!
                break 'attempts;
            }
        }

        Level {
            map,
            rooms,
            corridors: Vec::new(),
        }
    }

    pub fn get_neighbours(&self, cell: (usize, usize), distance: usize) -> Vec<(usize, usize)> {
        let x = cell.0;
        let y = cell.1;

        let in_bounds = |x: usize, y: usize| x < self.width() && y < self.height();

        let mut neighbours = Vec::new();
        // north
        if y >= distance {
            neighbours.push((x, y - distance))
        }
        // south
        if in_bounds(x, y + distance) {
            neighbours.push((x, y + distance));
        }
        // west
        if x >= distance {
            neighbours.push((x - distance, y));
        }
        // east
        if in_bounds(x + distance, y) {
            neighbours.push((x + distance, y));
        }

        neighbours
    }

    /// creates a maze using randomized depth-first search
    fn add_maze(&mut self) {
        let width = self.map.len();
        let height = self.map[0].len();

        let mut rng = thread_rng();

        let mut corridors = Vec::new();

        for x in (0..width).filter(Integer::is_odd) {
            for y in (0..height).filter(Integer::is_odd) {
                if self.map[x][y].typ != FieldType::Empty {
                    continue;
                }

                let mut visited_cells = Vec::new();

                self.map[x][y].typ = FieldType::Corridor;
                visited_cells.push((x, y));

                let mut corridor = Vec::new();

                corridor.push((x, y));

                while !visited_cells.is_empty() {
                    let cur_cell = visited_cells.pop().unwrap();

                    let neighbours = self.get_neighbours(cur_cell, 2);

                    if neighbours.is_empty() {
                        continue;
                    }

                    let unvisited_neighbours = neighbours
                        .into_iter()
                        .filter(|(x, y)| self.map[*x][*y].typ == Empty)
                        .collect::<Vec<(usize, usize)>>();

                    if unvisited_neighbours.is_empty() {
                        continue;
                    }

                    visited_cells.push(cur_cell);

                    let rand_neighbour =
                        unvisited_neighbours[rng.gen_range(0, unvisited_neighbours.len())];

                    let wall_to_remove = (
                        rand_neighbour.0 as i32 - cur_cell.0 as i32,
                        rand_neighbour.1 as i32 - cur_cell.1 as i32,
                    );

                    let wall_to_remove = (
                        cur_cell.0 as i32 + signum(wall_to_remove.0),
                        cur_cell.1 as i32 + signum(wall_to_remove.1),
                    );

                    // break in wall
                    self.map[wall_to_remove.0 as usize][wall_to_remove.1 as usize].typ = Corridor;
                    // create neighbour cell
                    self.map[rand_neighbour.0][rand_neighbour.1].typ = Corridor;

                    visited_cells.push(rand_neighbour);
                    corridor.push(rand_neighbour);
                    corridor.push((wall_to_remove.0 as usize, wall_to_remove.1 as usize));
                }

                corridors.push(corridor);
            }
        }

        self.corridors = corridors;
    }

    fn add_doors(&mut self) {
        let mut regions = Vec::new();
        regions.clone_from(&self.rooms);
        regions.append(&mut self.corridors.clone());

        let mut rng = thread_rng();

        // randomize walk-order, so the doors aren't always in the upper left area...
        let mut x_order = (2..self.width() - 2).collect::<Vec<usize>>();
        let mut y_order = (2..self.height() - 2).collect::<Vec<usize>>();

        x_order.shuffle(&mut rng);
        y_order.shuffle(&mut rng);

        // all regions are seperated now. find connectors and connect them.
        for &x in &x_order {
            for &y in &y_order {
                let mut left_region: Option<usize> = None;
                let mut right_region: Option<usize> = None;
                let mut top_region: Option<usize> = None;
                let mut bottom_region: Option<usize> = None;

                for (idx, r) in regions.iter().enumerate() {
                    let left = r.iter().any(|t| t.0 == x - 1 && t.1 == y);
                    let right = r.iter().any(|t| t.0 == x + 1 && t.1 == y);
                    let top = r.iter().any(|t| t.0 == x && t.1 == y - 1);
                    let bottom = r.iter().any(|t| t.0 == x && t.1 == y + 1);

                    // tiles are in the same region and connecting them makes no sense
                    if left && right || top && bottom {
                        continue;
                    }

                    if left {
                        left_region = Some(idx);
                    }
                    if right {
                        right_region = Some(idx);
                    }
                    if top {
                        top_region = Some(idx);
                    }
                    if bottom {
                        bottom_region = Some(idx);
                    }
                }

                let mut connect_if_possible = |region_a: Option<usize>, region_b: Option<usize>| {
                    if !(region_a.is_some() && region_b.is_some()) {
                        return;
                    }

                    self.map[x][y].typ = Door;
                    self.corridors[0].push((x, y));

                    let mut region_b_content = regions[region_b.unwrap()].clone();
                    regions[region_a.unwrap()].append(&mut region_b_content);

                    // chance to not remove region, so a room can have two doors
                    if rng.gen_bool(0.6) {
                        regions.remove(region_b.unwrap());
                    }
                };

                connect_if_possible(left_region, right_region);
                connect_if_possible(top_region, bottom_region);

                // all regions have been connected/merged into one
                if regions.len() == 1 {
                    break;
                }
            }
        }
    }

    fn remove_dead_ends(&mut self) -> usize {
        let mut corridors = self.corridors.clone();

        let mut removed = 0;
        for corridor in corridors.iter_mut() {
            for l in (0..corridor.len()).rev() {
                let cur_cell = &corridor[l];
                let neighbours = self.get_neighbours(*cur_cell, 1);
                let neighbour_count = neighbours
                    .iter()
                    .filter(|(x, y)| self.map[*x][*y].typ == Empty)
                    .count();
                if neighbour_count == 3 {
                    self.map[cur_cell.0][cur_cell.1].typ = Empty;
                    corridor.remove(l);
                    removed += 1;
                }
            }
        }

        self.corridors = corridors;

        removed
    }
}

fn gen_odd_range(lower: usize, upper: usize) -> usize {
    let mut x: usize;
    let mut rng = thread_rng();

    loop {
        x = rng.gen_range(lower, upper);
        if x.is_odd() {
            break;
        }
    }

    x
}

fn gen_even_range(lower: usize, upper: usize) -> usize {
    let mut x: usize;
    let mut rng = thread_rng();

    loop {
        x = rng.gen_range(lower, upper);
        if x.is_even() {
            break;
        }
    }

    x
}
