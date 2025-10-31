#![feature(default_field_values)]

use evdev::{EventType, InputEvent, RelativeAxisCode};
use std::time::SystemTime;

/// Parameters for the anxious scroll algorithm
#[derive(Debug, Clone)]
pub struct AnxiousParams {
    /// Base sensitivity to start at
    pub base_sens: f32,
    /// Max sensitivity to taper off towards
    pub max_sens: f32,
    /// How fast to ramp up the logistic function
    pub ramp_up_rate: f32,
}

impl Default for AnxiousParams {
    fn default() -> Self {
        Self {
            base_sens: 1.0,
            max_sens: 15.0,
            ramp_up_rate: 0.3,
        }
    }
}

/// State for tracking scroll velocity over time
#[derive(Debug)]
#[repr(transparent)]
pub struct AnxiousState {
    pub prev_time: SystemTime,
}

impl AnxiousState {
    pub fn new() -> Self {
        Self {
            prev_time: SystemTime::now(),
        }
    }
}

/// Constants for the exponential lookup table
/// EXP_LOOKUP_STEPS >= 2 and EXP_LOOKUP_END > EXP_LOOKUP_START is assumed
const EXP_LOOKUP_START: f32 = -20.0;
const EXP_LOOKUP_END: f32 = 20.0;
const EXP_LOOKUP_STEPS: usize = 1000;
const EXP_LOOKUP_STEP_SIZE: f32 = (EXP_LOOKUP_END - EXP_LOOKUP_START) / EXP_LOOKUP_STEPS as f32;

// exp_lut_macro::exp_lut_macro!(EXP_LOOKUP_START, EXP_LOOKUP_END, EXP_LOOKUP_STEPS);

#[inline(always)]
/// We use a logistic function as the transformation function.
/// f(vel) = max_sens / (1 + C * e^(-ramp_up_rate * vel)), where
/// C = (max_sens / (base_sens) - 1
/// Visualisation: https://www.desmos.com/calculator/grsgyudrch
pub fn apply_anxious_scroll(
    value: f32,
    timestamp: SystemTime,
    anxious_params: &AnxiousParams,
    anxious_state: &mut AnxiousState,
) -> i32 {
    let elapsed_time = match timestamp.duration_since(anxious_state.prev_time) {
        Ok(duration) => duration,
        Err(_) => {
            // If timestamp is earlier than prev_time (clock adjustment, out-of-order events, etc.),
            // use a slow scroll duration (1 second) to treat it as a gentle scroll
            std::time::Duration::from_millis(1000)
        }
    };
    anxious_state.prev_time = timestamp;

    let vel = value.abs() / elapsed_time.as_millis() as f32;
    let c = (anxious_params.max_sens / anxious_params.base_sens) - 1.0;
    // TODO: Use fast approximation for the calculation
    let sens = anxious_params.max_sens
        / (1.0 + c * (-1.0 * vel as f32 * anxious_params.ramp_up_rate).exp());
    return (value * sens) as i32;
}

#[inline(always)]
/// Process a batch of input events, applying anxious scroll transformation to wheel events
/// This is a pure function with no I/O dependencies, making it easily testable and benchmarkable
pub fn process_events(
    events: impl Iterator<Item = InputEvent>,
    anxious_params: &AnxiousParams,
    anxious_state: &mut AnxiousState,
) -> Vec<InputEvent> {
    let mut event_batch = Vec::new();

    for event in events {
        if event.event_type() == EventType::RELATIVE
            && event.code() == RelativeAxisCode::REL_WHEEL_HI_RES.0
        {
            // Create a new event with modified value
            let modified_value = apply_anxious_scroll(
                event.value() as f32,
                event.timestamp(),
                anxious_params,
                anxious_state,
            );
            // new_now() is not necessary here as the kernel will update the time field
            // when it emits the events to any programs reading the event "file".
            let modified_event =
                InputEvent::new(event.event_type().0, event.code(), modified_value);
            event_batch.push(modified_event);
        } else if event.event_type() == EventType::RELATIVE
            && event.code() == RelativeAxisCode::REL_WHEEL.0
        {
            // Drop event
            continue;
        } else {
            // Pass through all other events unchanged
            event_batch.push(event);
        }
    }

    event_batch
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn create_test_state_with_time(prev_time: SystemTime) -> AnxiousState {
        AnxiousState { prev_time }
    }

    #[test]
    fn test_zero_value() {
        let params = AnxiousParams::default();
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let mut state = create_test_state_with_time(base_time);

        let result = apply_anxious_scroll(
            0.0,
            base_time + Duration::from_millis(10),
            &params,
            &mut state,
        );
        assert_eq!(result, 0);
    }

    #[test]
    fn test_large_value() {
        let params = AnxiousParams::default();
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let mut state = create_test_state_with_time(base_time);

        let result = apply_anxious_scroll(
            1000.0,
            base_time + Duration::from_millis(1),
            &params,
            &mut state,
        );
        // Should not panic and should return a reasonable value
        assert!(result > 0);
    }

    #[test]
    fn test_negative_value() {
        let params = AnxiousParams::default();
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let mut state = create_test_state_with_time(base_time);

        let result = apply_anxious_scroll(
            -10.0,
            base_time + Duration::from_millis(10),
            &params,
            &mut state,
        );
        assert!(result < 0);
    }

    #[test]
    fn test_very_small_elapsed_time() {
        let params = AnxiousParams::default();
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let mut state = create_test_state_with_time(base_time);

        // Test with very small elapsed time (1 microsecond)
        let result = apply_anxious_scroll(
            10.0,
            base_time + Duration::from_micros(1),
            &params,
            &mut state,
        );
        // Should not panic and should return a reasonable value
        assert!(result > 0);
    }

    #[test]
    fn test_out_of_order_events() {
        let params = AnxiousParams::default();
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let mut state = create_test_state_with_time(base_time + Duration::from_millis(100));

        // Test with out-of-order event (timestamp earlier than prev_time)
        let result = apply_anxious_scroll(
            120.0,
            base_time + Duration::from_millis(50), // Earlier than prev_time
            &params,
            &mut state,
        );
        // Should not panic and should return a reasonable value
        // The fallback duration (1000ms) should result in slow scroll behavior
        assert!(result > 0);
        // With 1000ms duration, this should behave like a slow scroll (low sensitivity)
        assert!(result < 2000); // Should be reasonable for slow scroll
    }


    #[test]
    fn test_parameter_configurations() {
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let mut state = create_test_state_with_time(base_time);

        // Test default parameters
        let default_params = AnxiousParams::default();
        let result1 = apply_anxious_scroll(
            10.0,
            base_time + Duration::from_millis(10),
            &default_params,
            &mut state,
        );

        // Test high sensitivity
        let high_sens_params = AnxiousParams {
            base_sens: 1.0,
            max_sens: 30.0,
            ramp_up_rate: 0.5,
        };
        let result2 = apply_anxious_scroll(
            10.0,
            base_time + Duration::from_millis(10),
            &high_sens_params,
            &mut state,
        );

        // Test low sensitivity
        let low_sens_params = AnxiousParams {
            base_sens: 0.5,
            max_sens: 5.0,
            ramp_up_rate: 0.1,
        };
        let result3 = apply_anxious_scroll(
            10.0,
            base_time + Duration::from_millis(10),
            &low_sens_params,
            &mut state,
        );

        // All should return reasonable values
        assert!(result1 > 0);
        assert!(result2 > 0);
        assert!(result3 > 0);

        // High sensitivity should generally produce higher values than low sensitivity
        assert!(result2 > result3);
    }

    #[test]
    fn test_process_events_basic() {
        use evdev::{EventType, InputEvent, RelativeAxisCode};

        // Create events with proper timestamps to avoid SystemTime issues
        let base_time = UNIX_EPOCH + Duration::from_secs(1000000000);
        let events = vec![
            InputEvent::new_now(
                EventType::RELATIVE.0,
                RelativeAxisCode::REL_WHEEL_HI_RES.0,
                120,
            ),
            InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_WHEEL.0, 1), // Should be dropped
            InputEvent::new_now(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, 10), // Should pass through
        ];

        let params = AnxiousParams::default();
        let mut state = create_test_state_with_time(base_time);

        let result = process_events(events.iter().cloned(), &params, &mut state);

        // Should have 2 events: one processed wheel event and one pass-through event
        assert_eq!(result.len(), 2);

        // First event should be the processed wheel event
        assert_eq!(result[0].event_type(), EventType::RELATIVE);
        assert_eq!(result[0].code(), RelativeAxisCode::REL_WHEEL_HI_RES.0);

        // Second event should be the pass-through event
        assert_eq!(result[1].event_type(), EventType::RELATIVE);
        assert_eq!(result[1].code(), RelativeAxisCode::REL_X.0);
        assert_eq!(result[1].value(), 10);
    }
}
