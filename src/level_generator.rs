use std::cmp::min;

use num::{signum, Integer};
use rand::{thread_rng, Rng};

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
    pub map: Vec<Vec<T>>,
}

impl<T: Default + Eq + Copy> Level<T> {
    fn init_map(width: usize, height: usize) -> Vec<Vec<T>> {
        if width.is_even() || height.is_even() {
            panic!(
                "width and height of map must be odd! width: {}, height: {}",
                width, height
            );
        }

        let mut map: Vec<Vec<T>> = Vec::new();

        map.resize_with(width, || {
            let mut inner_vec: Vec<T> = Vec::new();
            inner_vec.resize_with(height, || T::default());
            inner_vec
        });

        map
    }

    pub fn create_dungeon(
        width: usize,
        height: usize,
        room_options: RoomOptions,
        room_identifier: T,
        corridor_identifier: T,
    ) -> Self {
        let rooms = Level::create_rooms(
            width,
            height,
            room_options.max_rooms,
            room_options.max_attempts,
            room_options.min_size,
            room_options.max_size,
            room_identifier,
        );

        let map = Level::init_map(width, height);

        let maze = Level::create_maze(&map, corridor_identifier);

        // let map = &mut rooms.map;
        //
        // for x in 0..width {
        //     for y in 0..height {
        //         if map[x][y] == T::default() && maze.map[x][y] != T::default() {
        //             map[x][y] = maze.map[x][y];
        //         }
        //     }
        // }

        // rooms

        maze
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

        for _ in 0..max_rooms {
            'attempts: for _ in 0..max_attempts {
                let x = gen_odd_range(0, width - 1);
                let x_extent = gen_even_range(min_size, max_size);
                let x_extent = min(x_extent, width - x - 1);

                let y = gen_odd_range(0, height - 1);
                let y_extent = gen_even_range(min_size, max_size);
                let y_extent = min(y_extent, height - y - 1);

                // try to place the room...
                for x_check in x..=(x + x_extent) {
                    for y_check in y..=(y + y_extent) {
                        if map[x_check][y_check] != T::default() {
                            // field is already taken by another room, try again!
                            continue 'attempts;
                        }
                    }
                }

                for x_check in x..=(x + x_extent) {
                    for y_check in y..=(y + y_extent) {
                        map[x_check][y_check] = room_identifier;
                    }
                }
                // attempt successful. Create the next room!
                break 'attempts;
            }
        }

        Level { map }
    }

    fn create_maze(rooms: &Vec<Vec<T>>, corridor_identifier: T) -> Self {
        let mut map = rooms.clone();

        let mut rng = thread_rng();

        let mut visited_cells = Vec::<(usize, usize)>::new();

        map[1][1] = corridor_identifier;
        visited_cells.push((1, 1));

        let mut iterations = 0;
        while !visited_cells.is_empty() {
            iterations += 1;

            let cur_cell = visited_cells.pop().unwrap();

            let in_bounds = |x: usize, y: usize| x < map.len() && y < map[0].len();

            let x = cur_cell.0;
            let y = cur_cell.1;

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

            if neighbours.is_empty() {
                continue;
            }

            let unvisited_neighbours = neighbours
                .into_iter()
                .filter(|(x, y)| map[*x][*y] != corridor_identifier)
                .collect::<Vec<(usize, usize)>>();

            if unvisited_neighbours.is_empty() {
                continue;
            }

            visited_cells.push(cur_cell);

            let rand_neighbour = unvisited_neighbours[rng.gen_range(0, unvisited_neighbours.len())];

            let wall_to_remove = (
                rand_neighbour.0 as i32 - cur_cell.0 as i32,
                rand_neighbour.1 as i32 - cur_cell.1 as i32,
            );

            let wall_to_remove = (
                cur_cell.0 as i32 + signum(wall_to_remove.0),
                cur_cell.1 as i32 + signum(wall_to_remove.1),
            );

            // break in wall
            map[wall_to_remove.0 as usize][wall_to_remove.1 as usize] = corridor_identifier;
            // create neighbour cell
            map[rand_neighbour.0][rand_neighbour.1] = corridor_identifier;

            visited_cells.push(rand_neighbour);
        }

        println!("Maze creation took {} iterations", iterations);

        Level { map }
    }
}
