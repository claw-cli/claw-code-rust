use std::time::{Duration, Instant};

/// Buffered burst of rapidly arriving pasted keypresses.
///
/// Some terminals emit large pastes as a stream of individual keypress events instead of one
/// bracketed paste event. This helper coalesces those fast keypresses into one string so the TUI
/// can update the composer once per burst instead of once per character.
#[derive(Debug, Default)]
pub(crate) struct PasteBurst {
    pending: String,
    last_key_at: Option<Instant>,
    active: bool,
}

impl PasteBurst {
    /// Maximum gap between sequential characters that should still be treated as one paste burst.
    const BURST_GAP: Duration = Duration::from_millis(18);

    /// Records one plain character. Returns `true` when the caller should defer insertion because
    /// the character has been absorbed into the burst buffer.
    pub(crate) fn push_char(&mut self, ch: char, now: Instant) -> bool {
        match self.last_key_at {
            None => {
                self.last_key_at = Some(now);
                false
            }
            Some(previous) if self.active && now.duration_since(previous) <= Self::BURST_GAP => {
                self.pending.push(ch);
                self.last_key_at = Some(now);
                true
            }
            Some(previous) if now.duration_since(previous) <= Self::BURST_GAP => {
                self.pending.push(ch);
                self.last_key_at = Some(now);
                self.active = true;
                true
            }
            Some(_) => {
                self.pending.clear();
                self.last_key_at = Some(now);
                self.active = false;
                false
            }
        }
    }

    /// Records a newline only when a paste burst is already underway.
    pub(crate) fn push_newline(&mut self, now: Instant) -> bool {
        if !self.active {
            return false;
        }
        self.pending.push('\n');
        self.last_key_at = Some(now);
        self.active = true;
        true
    }

    /// Drains the buffered paste text if either forced or the burst has gone idle.
    pub(crate) fn take_if_due(&mut self, now: Instant, force: bool) -> Option<String> {
        let due = force
            || self
                .last_key_at
                .is_some_and(|previous| now.duration_since(previous) > Self::BURST_GAP);
        if !due || self.pending.is_empty() {
            return None;
        }

        self.last_key_at = None;
        self.active = false;
        Some(std::mem::take(&mut self.pending))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use pretty_assertions::assert_eq;

    use super::PasteBurst;

    #[test]
    fn burst_groups_rapid_characters_until_due() {
        let start = Instant::now();
        let mut burst = PasteBurst::default();

        assert!(!burst.push_char('a', start));
        assert!(burst.push_char('b', start + Duration::from_millis(5)));
        assert!(burst.push_char('c', start + Duration::from_millis(10)));

        assert_eq!(
            burst.take_if_due(start + Duration::from_millis(15), false),
            None
        );
        assert_eq!(
            burst.take_if_due(start + Duration::from_millis(40), false),
            Some("bc".to_string())
        );
    }

    #[test]
    fn non_burst_input_does_not_flush_buffer() {
        let start = Instant::now();
        let mut burst = PasteBurst::default();

        assert!(!burst.push_char('a', start));
        assert!(!burst.push_char('b', start + Duration::from_millis(40)));

        assert_eq!(
            burst.take_if_due(start + Duration::from_millis(80), false),
            None
        );
    }

    #[test]
    fn force_flush_returns_pending_newline_burst() {
        let start = Instant::now();
        let mut burst = PasteBurst::default();

        assert!(!burst.push_char('a', start));
        assert!(burst.push_char('b', start + Duration::from_millis(5)));
        assert!(burst.push_newline(start + Duration::from_millis(10)));

        assert_eq!(
            burst.take_if_due(start + Duration::from_millis(11), true),
            Some("b\n".to_string())
        );
    }
}
