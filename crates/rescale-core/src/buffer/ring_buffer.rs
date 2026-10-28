//! Ring Buffer (Circular Buffer) generic fixed-size zero-allocation.

#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    write_pos: usize,
    capacity: usize,
}

impl<T: Copy + Default> RingBuffer<T> {
    /// Membuat RingBuffer baru dengan kapasitas tetap
    ///
    /// # Panics
    ///
    /// Panik jika `capacity` bernilai nol.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than 0");
        Self {
            buffer: vec![T::default(); capacity],
            write_pos: 0,
            capacity,
        }
    }

    /// Menulis satu elemen ke posisi write saat ini dan memajukan pointer
    #[inline(always)]
    pub fn push(&mut self, item: T) {
        self.buffer[self.write_pos] = item;
        self.write_pos = (self.write_pos + 1) % self.capacity;
    }

    /// Membaca elemen dari masa lalu sejauh `delay_samples` yang lalu.
    /// delay_samples = 0 berarti elemen yang baru saja di-push.
    ///
    /// # Panics
    ///
    /// Panik jika `delay_samples` lebih besar atau sama dengan kapasitas.
    #[inline(always)]
    pub fn read_past(&self, delay_samples: usize) -> T {
        assert!(
            delay_samples < self.capacity,
            "Delay exceeds buffer capacity"
        );
        let read_pos =
            (self.write_pos + self.capacity - 1 - (delay_samples % self.capacity)) % self.capacity;
        self.buffer[read_pos]
    }

    /// Mengembalikan kapasitas buffer
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Reset seluruh buffer ke nilai default
    pub fn clear(&mut self) {
        self.buffer.fill(T::default());
        self.write_pos = 0;
    }
}
