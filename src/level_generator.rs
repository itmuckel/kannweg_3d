use std::cmp::min;

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
pub enum Field {
    Corridor,
    Floor,
    Empty,
}

impl Default for Field {
    fn default() -> Self {
        Field::Empty
    }
}

pub struct Level<T: Default + Eq + Copy> {
    rooms: Vec<Vec<(usize, usize)>>,
    corridors: Vec<Vec<(usize, usize)>>,
    pub map: Vec<Vec<T>>,
}

impl<T> Level<T>
where
    T: Default + Eq + Copy,
{
    pub fn width(&self) -> usize {
        self.map.len()
    }

    pub fn height(&self) -> usize {
        self.map[0].len()
    }

    fn init_map(width: usize, height: usize) -> Vec<Vec<T>> {
        if width.is_even() || height.is_even() {
            panic!(
                "width and height of map must be odd! width: {}, height: {}",
                width, height
            );
        }

        vec![vec![T::default(); height]; width]
    }

    pub fn create_dungeon(
        width: usize,
        height: usize,
        room_options: RoomOptions,
        room_identifier: T,
        corridor_identifier: T,
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

        level.add_maze(corridor_identifier);

        level.add_doors(corridor_identifier);

        level
    }

    fn create_rooms(
        width: usize,
        height: usize,
        max_rooms: usize,
        max_attempts: usize,
        min_size: usize,
        max_size: usize,
        room_identifier: T,
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

                // try to place the room...
                for x_check in x..=(x + x_extent) {
                    for y_check in y..=(y + y_extent) {
                        if map[x_check][y_check] != T::default() {
                            // field is already taken by another room, try again!
                            continue 'attempts;
                        }
                    }
                }

                let mut room_tiles = Vec::new();

                for x_check in x..=(x + x_extent) {
                    for y_check in y..=(y + y_extent) {
                        map[x_check][y_check] = room_identifier;
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

    fn get_neighbours(&self, cell: (usize, usize)) -> Vec<(usize, usize)> {
        let x = cell.0;
        let y = cell.1;

        let map = self.map.clone();

        let in_bounds = |x: usize, y: usize| x < map.len() && y < map[0].len();

        let mut neighbours = Vec::new();
        // north
        if y >= 2 {
            neighbours.push((x, y - 2))
        }
        // south
        if in_bounds(x, y + 2) {
            neighbours.push((x, y + 2));
        }
        // west
        if x >= 2 {
            neighbours.push((x - 2, y));
        }
        // east
        if in_bounds(x + 2, y) {
            neighbours.push((x + 2, y));
        }

        neighbours
    }

    /// creates a maze using randomized depth-first search
    fn add_maze(&mut self, corridor_identifier: T) {
        let width = self.map.len();
        let height = self.map[0].len();

        let mut rng = thread_rng();

        let mut corridors = Vec::new();

        for x in (0..width).filter(Integer::is_odd) {
            for y in (0..height).filter(Integer::is_odd) {
                if self.map[x][y] != T::default() {
                    continue;
                }

                let mut visited_cells = Vec::new();

                self.map[x][y] = corridor_identifier;
                visited_cells.push((x, y));

                let mut corridor = Vec::new();

                corridor.push((x, y));

                while !visited_cells.is_empty() {
                    let cur_cell = visited_cells.pop().unwrap();

                    let neighbours = self.get_neighbours(cur_cell);

                    if neighbours.is_empty() {
                        continue;
                    }

                    let unvisited_neighbours = neighbours
                        .into_iter()
                        .filter(|(x, y)| self.map[*x][*y] == T::default())
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
                    self.map[wall_to_remove.0 as usize][wall_to_remove.1 as usize] =
                        corridor_identifier;
                    // create neighbour cell
                    self.map[rand_neighbour.0][rand_neighbour.1] = corridor_identifier;

                    visited_cells.push(rand_neighbour);
                    corridor.push(rand_neighbour);
                }

                corridors.push(corridor);
            }
        }

        self.corridors = corridors;
    }

    fn add_doors(&mut self, door: T) {
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

                    self.map[x][y] = door;

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

        // TODO: Remove dead ends
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
