use rand::{thread_rng, Rng};
use std::cmp::min;

#[derive(Eq, PartialEq)]
pub enum Field {
    Floor,
    Empty,
}

pub struct Level {
    pub map: Vec<Vec<i32>>,
}

impl Level {
    pub fn new(width: usize, height: usize, max_rooms: usize, max_attempts: usize, min_size: usize, max_size: usize) -> Self {
        let mut rng = thread_rng();

        let mut map: Vec<Vec<i32>> = Vec::new();

        // init map with empty fields
        map.resize_with(width, || {
            let mut inner_vec: Vec<i32> = Vec::new();
            inner_vec.resize_with(height, || 0);
            inner_vec
        });

        for room_num in 0..max_rooms {
            'attempts: for _ in 0..max_attempts {
                let x = rng.gen_range(0, width - 1);
                let x_extent = rng.gen_range(min_size, max_size);
                let x_extent = min(x_extent, width - x);

                let y = rng.gen_range(0, height - 1);
                let y_extent = rng.gen_range(min_size, max_size);
                let y_extent = min(y_extent, height - y);

                // try to place the room...
                for x_check in x..(x + x_extent) {
                    for y_check in y..(y + y_extent) {
                        if map[x_check][y_check] != 0 {
                            // field is already taken by another room, try again!
                            continue 'attempts;
                        }
                    }
                }

                for x_check in x..(x + x_extent) {
                    for y_check in y..(y + y_extent) {
                        map[x_check][y_check] = room_num as i32;
                    }
                }
                // attempt successful. Create the next room!
                break 'attempts;
            }
        }

        Level { map }
    }
}
