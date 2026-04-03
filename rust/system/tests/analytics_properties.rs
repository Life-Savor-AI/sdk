#![cfg(feature = "analytics")]

//! Property-based tests for `AnalyticsReporter`.
//!
//! **Property 21: AnalyticsReporter batches events and handles failures without panicking**
//!
//! **Validates: Requirements 33.2, 33.3, 33.4**

use chrono::Utc;
use lifesavor_system_sdk::analytics::{
    AnalyticsEvent, AnalyticsEventType, AnalyticsReporter,
};
use proptest::prelude::*;

/// Strategy to generate an arbitrary `AnalyticsEventType`.
fn arb_event_type() -> impl Strategy<Value = AnalyticsEventType> {
    prop_oneof![
        Just(AnalyticsEventType::Install),
        Just(AnalyticsEventType::Uninstall),
        Just(AnalyticsEventType::DailyUsagePing),
        Just(AnalyticsEventType::PageView),
    ]
}

/// Strategy to generate an arbitrary `AnalyticsEvent`.
fn arb_event() -> impl Strategy<Value = AnalyticsEvent> {
    (
        arb_event_type(),
        "[a-zA-Z0-9_-]{1,64}",
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
        "[a-z]{2,12}-[a-z0-9_]{2,12}",
    )
        .prop_map(|(event_type, component_id, version, platform)| AnalyticsEvent {
            event_type,
            component_id,
            version,
            platform,
            timestamp: Utc::now(),
            payload: None,
        })
}

/// Strategy to generate a non-empty vec of events (1..=20).
fn arb_events() -> impl Strategy<Value = Vec<AnalyticsEvent>> {
    prop::collection::vec(arb_event(), 1..=20)
}

proptest! {
    /// **Property 21: AnalyticsReporter batches events and handles failures without panicking**
    ///
    /// **Validates: Requirements 33.2, 33.3, 33.4**
    ///
    /// For any sequence of AnalyticsEvent instances:
    /// 1. Reporting them all increases pending_count to match the number reported
    /// 2. Flushing drops pending_count to 0
    /// 3. The reporter never panics regardless of input
    #[test]
    fn reporter_batches_events_and_flushes(events in arb_events()) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut reporter = AnalyticsReporter::new("ls_dev_test_key");
            let count = events.len();

            // Report all events — pending_count should match.
            for event in events {
                reporter.report(event);
            }
            prop_assert_eq!(
                reporter.pending_count(),
                count,
                "pending_count should equal number of reported events"
            );

            // Flush — pending_count should drop to 0.
            let result = reporter.flush().await;
            prop_assert!(result.is_ok(), "flush should succeed with mock backend");
            prop_assert_eq!(
                reporter.pending_count(),
                0,
                "pending_count should be 0 after flush"
            );
            prop_assert_eq!(
                reporter.failed_count(),
                0,
                "failed_count should be 0 after successful flush"
            );

            Ok(())
        })?;
    }

    /// Flushing an empty reporter is always safe and returns Ok.
    ///
    /// **Validates: Requirements 33.4**
    #[test]
    fn flush_empty_reporter_never_panics(api_key in "[a-zA-Z0-9_]{1,32}") {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut reporter = AnalyticsReporter::new(&api_key);
            prop_assert_eq!(reporter.pending_count(), 0);
            let result = reporter.flush().await;
            prop_assert!(result.is_ok(), "flushing empty reporter should succeed");
            Ok(())
        })?;
    }

    /// Multiple report-flush cycles never panic and always reset pending_count.
    ///
    /// **Validates: Requirements 33.2, 33.3**
    #[test]
    fn multiple_flush_cycles_are_safe(
        batch1 in arb_events(),
        batch2 in arb_events(),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut reporter = AnalyticsReporter::new("ls_dev_cycle_key");

            // First cycle
            let count1 = batch1.len();
            for event in batch1 {
                reporter.report(event);
            }
            prop_assert_eq!(reporter.pending_count(), count1);
            reporter.flush().await.unwrap();
            prop_assert_eq!(reporter.pending_count(), 0);

            // Second cycle
            let count2 = batch2.len();
            for event in batch2 {
                reporter.report(event);
            }
            prop_assert_eq!(reporter.pending_count(), count2);
            reporter.flush().await.unwrap();
            prop_assert_eq!(reporter.pending_count(), 0);

            Ok(())
        })?;
    }
}
