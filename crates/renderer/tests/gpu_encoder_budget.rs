//! GPU tier: a `FrameEncoder` flushes before a draw that would pass its
//! budget of draws or render passes, and every draw still lands.

#[path = "support/frame.rs"]
mod frame;

use frame::{last_submission, pixel, quad, square, target};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{EncoderBudget, FrameEncoder, QuadPipeline};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

#[test]
fn gpu_seven_draws_with_a_budget_of_three_submit_three_encoders_and_all_land() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, 8);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let budget = EncoderBudget {
        passes: 100,
        draws: 3,
    };
    let encoder = FrameEncoder::with_budget(&gpu.device, budget);
    let before = last_submission(&gpu);

    // When: a draw per pixel of the top row's first seven
    for x in 0..7u8 {
        let left = f32::from(x);
        let on_view = target(&gpu, &encoder, &view, 8);
        pipeline.draw(&on_view, &[quad([left, 0.0, left + 1.0, 1.0], RED)]);
    }
    let passes = encoder.render_passes();
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let submits = last_submission(&gpu) - before - 1;
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
    let row: Vec<[u8; 4]> = (0..7).map(|x| pixel(&pixels, 8, x, 0)).collect();
    assert_eq!((passes, submits), (3, 3));
    assert_eq!(row, vec![RED; 7]);
}

#[test]
fn gpu_five_passes_with_a_budget_of_two_submit_three_encoders_and_all_land() {
    // Given
    let gpu = gpu();
    let ((first, first_view), (second, second_view)) = (square(&gpu, 4), square(&gpu, 4));
    let mut pipeline = QuadPipeline::new(&gpu.device, first.format());
    let budget = EncoderBudget {
        passes: 2,
        draws: 100,
    };
    let encoder = FrameEncoder::with_budget(&gpu.device, budget);
    let before = last_submission(&gpu);

    // When: first, second, first, second, first; a column each time
    for column in 0..5u8 {
        let view = if column % 2 == 0 {
            &first_view
        } else {
            &second_view
        };
        let left = f32::from(column / 2);
        let on = target(&gpu, &encoder, view, 4);
        pipeline.draw(&on, &[quad([left, 0.0, left + 1.0, 4.0], BLUE)]);
    }
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then: first holds columns 0, 1, 2; second columns 0, 1
    let submits = last_submission(&gpu) - before - 1;
    let first = read_rgba8(&gpu.device, &gpu.queue, &first);
    let second = read_rgba8(&gpu.device, &gpu.queue, &second);
    let seen = [pixel(&first, 4, 2, 3), pixel(&second, 4, 1, 3)];
    assert_eq!(submits, 3);
    assert_eq!(seen, [BLUE, BLUE]);
}

#[test]
fn gpu_five_draws_on_one_view_with_a_budget_of_one_pass_stay_in_one_encoder() {
    // Given
    let gpu = gpu();
    let (texture, view) = square(&gpu, 8);
    let mut pipeline = QuadPipeline::new(&gpu.device, texture.format());
    let budget = EncoderBudget {
        passes: 1,
        draws: 100,
    };
    let encoder = FrameEncoder::with_budget(&gpu.device, budget);
    let before = last_submission(&gpu);

    // When
    for x in 0..5u8 {
        let left = f32::from(x);
        let on_view = target(&gpu, &encoder, &view, 8);
        pipeline.draw(&on_view, &[quad([left, 0.0, left + 1.0, 1.0], RED)]);
    }
    let passes = encoder.render_passes();
    encoder.submit(&gpu.queue).expect("no draw refused");

    // Then
    let submits = last_submission(&gpu) - before - 1;
    assert_eq!((passes, submits), (1, 1));
}

#[test]
#[should_panic(expected = "EncoderBudget { passes: 0, draws: 10 }: a budget of 0")]
fn gpu_a_budget_of_no_passes_is_refused() {
    let gpu = gpu();
    let _ = FrameEncoder::with_budget(
        &gpu.device,
        EncoderBudget {
            passes: 0,
            draws: 10,
        },
    );
}

#[test]
#[should_panic(expected = "EncoderBudget { passes: 10, draws: 0 }: a budget of 0")]
fn gpu_a_budget_of_no_draws_is_refused() {
    let gpu = gpu();
    let _ = FrameEncoder::with_budget(
        &gpu.device,
        EncoderBudget {
            passes: 10,
            draws: 0,
        },
    );
}
