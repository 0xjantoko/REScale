//! Pelacak nilai maksimum bergerak (sliding maximum) zero-allocation.
//!
//! Dipakai limiter lookahead untuk mengetahui puncak sinyal di seluruh window
//! lookahead, bukan hanya puncak sesaat dari envelope.

/// Antrean monotonik untuk maksimum bergerak pada window tetap.
///
/// Setiap sample di-`push`, dan [`SlidingMax::max`] mengembalikan nilai terbesar
/// dari `window_len` sample terakhir. Biaya O(1) amortisasi per sample dan tidak
/// ada alokasi memori setelah konstruksi.
pub struct SlidingMax {
    window_len: usize,
    /// Ring nilai per indeks global (`index % window_len`).
    values: Vec<f32>,
    /// Ring indeks global yang nilainya monotonik menurun dari depan ke belakang.
    deque: Vec<usize>,
    deque_head: usize,
    deque_len: usize,
    position: usize,
}

impl SlidingMax {
    /// Membuat pelacak dengan panjang window `window_len` (minimal 1).
    pub fn new(window_len: usize) -> Self {
        assert!(window_len > 0, "window_len must be greater than 0");
        Self {
            window_len,
            values: vec![0.0; window_len],
            deque: vec![0; window_len],
            deque_head: 0,
            deque_len: 0,
            position: 0,
        }
    }

    /// Memasukkan satu sample baru ke window.
    pub fn push(&mut self, value: f32) {
        let index = self.position;
        self.position += 1;
        self.values[index % self.window_len] = value;

        self.evict_expired(index);
        self.evict_smaller_from_back(value);

        let tail = (self.deque_head + self.deque_len) % self.window_len;
        self.deque[tail] = index;
        self.deque_len += 1;
    }

    /// Maksimum pada window saat ini.
    pub fn max(&self) -> f32 {
        self.values[self.deque[self.deque_head] % self.window_len]
    }

    /// Mengosongkan state tanpa mengubah kapasitas.
    pub fn reset(&mut self) {
        self.position = 0;
        self.deque_head = 0;
        self.deque_len = 0;
    }

    /// Indeks yang sudah keluar dari window `[position - window_len, position)`.
    fn evict_expired(&mut self, newest_index: usize) {
        while self.deque_len > 0 {
            let front_index = self.deque[self.deque_head];
            if newest_index - front_index < self.window_len {
                break;
            }
            self.deque_head = (self.deque_head + 1) % self.window_len;
            self.deque_len -= 1;
        }
    }

    /// Nilai lebih kecil di belakang tidak akan pernah menjadi maksimum lagi.
    fn evict_smaller_from_back(&mut self, value: f32) {
        while self.deque_len > 0 {
            let back_index = self.deque[(self.deque_head + self.deque_len - 1) % self.window_len];
            if self.values[back_index % self.window_len] > value {
                break;
            }
            self.deque_len -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_all(tracker: &mut SlidingMax, values: &[f32]) -> f32 {
        for &value in values {
            tracker.push(value);
        }
        tracker.max()
    }

    #[test]
    fn tracks_maximum_inside_the_window() {
        let mut tracker = SlidingMax::new(3);

        assert!((push_all(&mut tracker, &[1.0, 3.0, 2.0]) - 3.0).abs() < 1e-6);
        // Window 3 sample terakhir: {3.0, 2.0, 0.5} -> maksimum tetap 3.0.
        tracker.push(0.5);
        assert!((tracker.max() - 3.0).abs() < 1e-6);
        // 3.0 keluar window: {2.0, 0.5, 0.1} -> maksimum 2.0.
        tracker.push(0.1);
        assert!((tracker.max() - 2.0).abs() < 1e-6);
        // {0.5, 0.1, 0.0} -> maksimum 0.5.
        tracker.push(0.0);
        assert!((tracker.max() - 0.5).abs() < 1e-6);
        // {0.1, 0.0, 0.0} -> maksimum 0.1.
        tracker.push(0.0);
        assert!((tracker.max() - 0.1).abs() < 1e-6);
    }

    #[test]
    fn maximum_matches_brute_force_for_random_sequence() {
        let mut tracker = SlidingMax::new(5);
        let values: Vec<f32> = (0..200)
            .map(|index| ((index * 37 % 101) as f32 / 101.0).abs())
            .collect();

        for index in 0..values.len() {
            tracker.push(values[index]);
            let window_start = (index + 1).saturating_sub(5);
            let expected = values[window_start..=index]
                .iter()
                .fold(0.0_f32, |acc, &value| acc.max(value));
            assert!(
                (tracker.max() - expected).abs() < 1e-6,
                "index {index}: {} vs {expected}",
                tracker.max()
            );
        }
    }

    #[test]
    fn reset_restarts_the_window() {
        let mut tracker = SlidingMax::new(2);
        tracker.push(1.0);
        tracker.push(0.0);
        tracker.reset();
        tracker.push(0.25);
        assert!((tracker.max() - 0.25).abs() < 1e-6);
    }
}
