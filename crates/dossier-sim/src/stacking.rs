use dossier_beatmap::{Difficulty, Point};

use crate::timeline::TimedObject;

const STACK_DISTANCE: f64 = 3.0;

const STACK_SHIFT_PER_STEP: f64 = -6.4 / 64.0;

const FIRST_MODERN_STACKING_VERSION: u32 = 6;

pub(crate) fn apply(
    objects: &mut [TimedObject],
    difficulty: &Difficulty,
    stack_leniency: f64,
    format_version: u32,
) {
    if objects.len() < 2 {
        return;
    }
    if format_version < FIRST_MODERN_STACKING_VERSION {
        apply_old(objects, difficulty, stack_leniency);
        return;
    }

    let threshold = difficulty.preempt_ms() * stack_leniency;

    let starts: Vec<Point> = objects.iter().map(|o| o.pos).collect();
    let ends: Vec<Point> = objects.iter().map(end_position).collect();

    let mut heights = vec![0i32; objects.len()];

    for i in (1..objects.len()).rev() {
        if heights[i] != 0 || objects[i].is_spinner() {
            continue;
        }

        let mut current = i;
        let mut n = i as i64;

        if objects[i].is_slider() {
            loop {
                n -= 1;
                if n < 0 {
                    break;
                }
                let n = n as usize;
                if objects[n].is_spinner() {
                    continue;
                }
                if objects[current].start_ms - objects[n].start_ms > threshold {
                    break;
                }
                if ends[n].distance_to(starts[current]) < STACK_DISTANCE {
                    heights[n] = heights[current] + 1;
                    current = n;
                }
            }
        } else {
            loop {
                n -= 1;
                if n < 0 {
                    break;
                }
                let n = n as usize;
                if objects[n].is_spinner() {
                    continue;
                }
                if objects[current].start_ms - objects[n].end_ms > threshold {
                    break;
                }

                if objects[n].is_slider() && ends[n].distance_to(starts[current]) < STACK_DISTANCE {
                    let offset = heights[current] - heights[n] + 1;
                    for j in (n + 1)..=i {
                        if ends[n].distance_to(starts[j]) < STACK_DISTANCE {
                            heights[j] -= offset;
                        }
                    }
                    break;
                }

                if starts[n].distance_to(starts[current]) < STACK_DISTANCE {
                    heights[n] = heights[current] + 1;
                    current = n;
                }
            }
        }
    }

    let step = difficulty.circle_radius() * STACK_SHIFT_PER_STEP;
    for (object, height) in objects.iter_mut().zip(heights) {
        object.stack_height = height;
        if height == 0 {
            continue;
        }
        let shift = f64::from(height) * step;
        object.translate(shift, shift);
    }
}

fn apply_old(objects: &mut [TimedObject], difficulty: &Difficulty, stack_leniency: f64) {
    let threshold = difficulty.preempt_ms() * stack_leniency;
    let starts: Vec<Point> = objects.iter().map(|o| o.pos).collect();
    let path_ends: Vec<Point> = objects.iter().map(path_end).collect();
    let mut heights = vec![0i32; objects.len()];

    for i in 0..objects.len() {
        if heights[i] != 0 && !objects[i].is_slider() {
            continue;
        }
        let mut start_time = objects[i].end_ms;
        let mut slider_stack = 0i32;

        for j in (i + 1)..objects.len() {
            if objects[j].start_ms - threshold > start_time {
                break;
            }
            if starts[j].distance_to(starts[i]) < STACK_DISTANCE {
                heights[i] += 1;
                start_time = objects[j].start_ms;
            } else if starts[j].distance_to(path_ends[i]) < STACK_DISTANCE {
                slider_stack += 1;
                heights[j] -= slider_stack;
                start_time = objects[j].start_ms;
            }
        }
    }

    let step = difficulty.circle_radius() * STACK_SHIFT_PER_STEP;
    for (object, height) in objects.iter_mut().zip(heights) {
        object.stack_height = height;
        if height == 0 {
            continue;
        }
        let shift = f64::from(height) * step;
        object.translate(shift, shift);
    }
}

fn path_end(object: &TimedObject) -> Point {
    match &object.kind {
        crate::timeline::TimedKind::Slider { path, .. } => {
            path.position_at(1.0).unwrap_or(object.pos)
        }
        _ => object.pos,
    }
}

fn end_position(object: &TimedObject) -> Point {
    object.ball_at(object.end_ms).unwrap_or(object.pos)
}
