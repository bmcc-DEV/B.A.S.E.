use crate::tracer::{DeviceTrace, TraceEvent};
/// Alinhamento temporal com sliding window para melhor correspondência
pub struct TraceAligner;

impl TraceAligner {
    /// Alinha dois traces usando nearest-neighbor com sliding window
    pub fn align(
        original: &DeviceTrace,
        actual: &DeviceTrace,
        max_time_delta_ns: u64,
    ) -> Vec<(usize, Option<usize>)> {
        let mut pairs: Vec<(usize, Option<usize>)> = Vec::new();
        let mut used: Vec<bool> = vec![false; actual.events.len()];

        for (oi, orig_event) in original.events.iter().enumerate() {
            let window_start = orig_event.timestamp_ns.saturating_sub(max_time_delta_ns);
            let window_end = orig_event.timestamp_ns.saturating_add(max_time_delta_ns);

            let best = actual.events.iter().enumerate()
                .filter(|(ai, ae)|
                    !used[*ai]
                    && ae.event_type == orig_event.event_type
                    && ae.timestamp_ns >= window_start
                    && ae.timestamp_ns <= window_end
                )
                .min_by_key(|(_, ae)| {
                    let diff = if ae.timestamp_ns > orig_event.timestamp_ns {
                        ae.timestamp_ns - orig_event.timestamp_ns
                    } else {
                        orig_event.timestamp_ns - ae.timestamp_ns
                    };
                    diff
                });

            match best {
                Some((ai, _)) => {
                    used[ai] = true;
                    pairs.push((oi, Some(ai)));
                }
                None => {
                    pairs.push((oi, None));
                }
            }
        }

        pairs
    }

    /// Calcula estatísticas do alinhamento
    pub fn alignment_stats(pairs: &[(usize, Option<usize>)]) -> AlignmentStats {
        let matched = pairs.iter().filter(|(_, m)| m.is_some()).count();
        let missed = pairs.len() - matched;
        let match_rate = if pairs.is_empty() { 0.0 } else { matched as f64 / pairs.len() as f64 };

        AlignmentStats { matched, missed, total: pairs.len(), match_rate }
    }
}

#[derive(Debug, Clone)]
pub struct AlignmentStats {
    pub matched: usize,
    pub missed: usize,
    pub total: usize,
    pub match_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::{EventType, TraceEvent};

    fn make_trace(events: Vec<(u64, EventType, u64)>) -> DeviceTrace {
        DeviceTrace {
            source: "test".into(),
            device_name: "dev".into(),
            events: events.into_iter().map(|(t, ty, a)| TraceEvent {
                timestamp_ns: t, channel: "CH0".into(), event_type: ty, address: a, value: None,
            }).collect(),
        }
    }

    #[test]
    fn test_perfect_alignment() {
        let orig = make_trace(vec![
            (1000, EventType::MmioWrite, 0x1000),
            (2000, EventType::MmioWrite, 0x1004),
        ]);
        let actual = make_trace(vec![
            (1050, EventType::MmioWrite, 0x1000),
            (2050, EventType::MmioWrite, 0x1004),
        ]);

        let pairs = TraceAligner::align(&orig, &actual, 500);
        let stats = TraceAligner::alignment_stats(&pairs);
        assert_eq!(stats.matched, 2);
        assert_eq!(stats.match_rate, 1.0);
    }

    #[test]
    fn test_missed_event() {
        let orig = make_trace(vec![
            (1000, EventType::MmioWrite, 0x1000),
            (2000, EventType::MmioWrite, 0x1004),
            (3000, EventType::MmioWrite, 0x1008),
        ]);
        let actual = make_trace(vec![
            (1050, EventType::MmioWrite, 0x1000),
            (3050, EventType::MmioWrite, 0x1008),
        ]);

        let pairs = TraceAligner::align(&orig, &actual, 500);
        let stats = TraceAligner::alignment_stats(&pairs);
        assert_eq!(stats.matched, 2);
        assert_eq!(stats.missed, 1);
    }

    #[test]
    fn test_type_filter() {
        let orig = make_trace(vec![
            (1000, EventType::MmioWrite, 0x1000),
            (2000, EventType::Interrupt, 16),
        ]);
        let actual = make_trace(vec![
            (2000, EventType::MmioWrite, 0x2000), // wrong type won't match
            (2050, EventType::Interrupt, 16),
        ]);

        let pairs = TraceAligner::align(&orig, &actual, 500);
        let stats = TraceAligner::alignment_stats(&pairs);
        assert_eq!(stats.matched, 1); // only Interrupt matches
        assert_eq!(stats.missed, 1); // MmioWrite has no match (address differs)
    }
}
