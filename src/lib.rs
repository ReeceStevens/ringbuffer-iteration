//! A ring buffer implementation where newer samples overwrite old samples. This implementation
//! also enables the use of Rust iterators to iterate through the ring buffer in order from oldest
//! to newest entries.
//!
//! This library is implemented without the use of the standard library, and thus requires passing
//! in a "backing store" of a statically allocated array.
#![no_std]
extern crate libm;
mod test_macros;
use libm::sqrtf;
use num_traits::PrimInt;

/// An implementation of a ring buffer data structure that creates a "lossy" queue. This is a
/// simplified version of the generic heapless `Queue` with a few constraints relaxed:
///
/// - The Item type is at least Copy + Clone
/// - We do not care about overwritten data in the buffer. Should be able to write indefinitely
///   without error.
///
/// An iterator is also defined for `RingBuffer` which implements [`Iterator`],
/// [`ExactSizeIterator`], and [`DoubleEndedIterator`]. The iterator begins at the head of the ring
/// buffer and iterates until it reaches the tail.
///
/// Also uses an internal flag `overwriting` to allow iteration over the last value in the
/// ring buffer, giving the structure a capacity of `N` rather than the typical ringbuffer capacity
/// of `N-1`.
pub struct RingBuffer<T: Copy + Clone, const N: usize> {
    inner: [T; N],
    head: usize,
    tail: usize,
    overwriting: bool,
}

impl<T, const N: usize> RingBuffer<T, N>
where
    T: Copy + Clone,
{
    pub fn new(backing_store: [T; N]) -> Self {
        RingBuffer {
            inner: backing_store,
            head: 0,
            tail: 0,
            overwriting: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        (self.tail + 1) % self.inner.len() == self.head
    }

    pub fn capacity(&self) -> usize {
        self.inner.len()
    }

    /// Add a new item to the ring buffer.
    pub fn add(&mut self, item: T) {
        self.inner[self.tail] = item;
        if self.is_full() {
            // Drop the last sample
            self.head = (self.head + 1) % self.inner.len();
            self.overwriting = true;
        }
        self.tail = (self.tail + 1) % self.inner.len();
    }

    /// Pops the last item off of the ring buffer.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        if self.overwriting {
            // If overwriting, grab first value from position behind head
            let compensated_index: usize = if self.head == 0 {
                self.inner.len() - 1
            } else {
                self.head - 1
            };
            let val = self.inner[compensated_index];
            self.overwriting = false;
            Some(val)
        } else {
            let val = self.inner[self.head];
            self.head = (self.head + 1) % self.inner.len();
            self.overwriting = false;
            Some(val)
        }
    }
}

pub struct RingBufferIterator<'a, T: Copy + Clone, const N: usize> {
    inner: &'a [T; N],
    head: usize,
    tail: usize,
    cursor: usize,
    overwriting: bool,
}

impl<T, const N: usize> RingBufferIterator<'_, T, N>
where
    T: Copy + Clone,
{
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        (self.tail + 1) % self.inner.len() == self.head
    }

    pub fn capacity(&self) -> usize {
        self.inner.len()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a RingBuffer<T, N>
where
    T: Copy + Clone,
{
    type Item = T;
    type IntoIter = RingBufferIterator<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        RingBufferIterator {
            inner: &self.inner,
            head: self.head,
            tail: self.tail,
            cursor: self.head,
            overwriting: self.overwriting,
        }
    }
}

impl<T, const N: usize> Iterator for RingBufferIterator<'_, T, N>
where
    T: Copy + Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor == self.head && self.overwriting {
            // If overwriting, grab first value from position behind head
            let compensated_index: usize = if self.head == 0 {
                self.inner.len() - 1
            } else {
                self.head - 1
            };
            let val = self.inner[compensated_index];
            self.overwriting = false;
            return Some(val);
        }
        if self.cursor != self.tail {
            let val = self.inner[self.cursor];
            self.cursor = (self.cursor + 1) % self.inner.len();
            return Some(val);
        } else {
            return None;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = ring_buffer_iter_len(self);
        (size, Some(size))
    }
}

impl<T, const N: usize> ExactSizeIterator for RingBufferIterator<'_, T, N>
where
    T: Copy + Clone,
{
    fn len(&self) -> usize {
        ring_buffer_iter_len(self)
    }
}

impl<T, const N: usize> DoubleEndedIterator for RingBufferIterator<'_, T, N>
where
    T: Copy + Clone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.cursor != self.tail {
            self.tail = if self.tail == 0 {
                self.inner.len() - 1
            } else {
                self.tail - 1
            };
            let val = self.inner[self.tail];
            return Some(val);
        } else {
            return None;
        }
    }
}

fn ring_buffer_iter_len<T, const N: usize>(buf: &RingBufferIterator<T, N>) -> usize
where
    T: Copy + Clone,
{
    if buf.cursor > buf.tail {
        (buf.inner.len() - buf.cursor) + buf.tail + 1
    } else {
        buf.tail - buf.cursor
    }
}

/// Calculate the mean of the values inside `buffer`.
///
/// This implementation upcasts to an i64 in order to prevent any risk of overflow during
/// summation. The result is still returned as the original data type. This only operates over
/// primitive integer types and not floating point numbers due to this upcast logic.
pub fn mean<T, const N: usize>(buffer: &RingBuffer<T, N>) -> T
where
    T: PrimInt,
{
    _mean(buffer.into_iter())
}

/// Calculate the mean of the last `window_len` values inside `buffer`.
///
/// See [`mean`] for details.
pub fn windowed_mean<T, const N: usize>(buffer: &RingBuffer<T, N>, window_len: usize) -> T
where
    T: PrimInt,
{
    _mean(buffer.into_iter().rev().take(window_len))
}

fn _mean<I, T>(iter: I) -> T
where
    I: ExactSizeIterator<Item = T>,
    T: PrimInt,
{
    let len = iter.len();
    let sum = |acc, el: T| acc + (el.to_i64().unwrap());
    T::from(iter.fold(0_i64, sum) / (len as i64)).unwrap()
}

/// Calculate the root-mean-square of `buffer`, passing in `mean`
/// as the pre-calculated mean.
pub fn ac_rms<T, const N: usize>(buffer: &RingBuffer<T, N>, mean: T) -> f32
where
    T: PrimInt,
{
    _ac_rms(buffer.into_iter(), mean)
}

/// Calculate the AC RMS of the last `window_len` values inside `buffer`.
///
/// See [`ac_rms`] for details.
pub fn windowed_ac_rms<T, const N: usize>(
    buffer: &RingBuffer<T, N>,
    mean: T,
    window_len: usize,
) -> f32
where
    T: PrimInt,
{
    _ac_rms(buffer.into_iter().rev().take(window_len), mean)
}

fn _ac_rms<I, T>(iter: I, mean: T) -> f32
where
    I: ExactSizeIterator<Item = T>,
    T: PrimInt,
{
    // TODO: Derisk if this casting is a performance bottleneck?
    let len = iter.len();
    let sum_square =
        |acc, el: T| acc + (el - mean).to_i64().unwrap() * (el - mean).to_i64().unwrap();
    let avg_sum_squares = iter.fold(0, sum_square) / (len as i64);
    sqrtf(avg_sum_squares as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    type DataUnit = i32;

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_empty_and_full() {
        let backing_store = [0; 4];
        let mut buf = RingBuffer::new(backing_store);
        assert_eq!(buf.is_empty(), true);
        assert_eq!(buf.is_full(), false);
        buf.add(1);
        assert_eq!(buf.is_empty(), false);
        buf.add(2);
        buf.add(3);
        assert_eq!(buf.is_full(), true);
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_add_items() {
        let backing_store = [0; 4];
        let mut buf = RingBuffer::new(backing_store);
        buf.add(1);
        buf.add(2);
        buf.add(3);

        assert_eq!(buf.pop(), Some(1));
        assert_eq!(buf.pop(), Some(2));
        assert_eq!(buf.pop(), Some(3));
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_iterate_through_unfilled_buf() {
        let backing_store = [0; 6];
        let mut buf = RingBuffer::new(backing_store);
        buf.add(1);
        buf.add(2);
        buf.add(3);
        let expected_values = [1, 2, 3];
        assert_eq!(buf.into_iter().len(), expected_values.len());
        for (idx, val) in (&mut buf).into_iter().enumerate() {
            assert_eq!(val, expected_values[idx]);
        }
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_iterate_through_items() {
        let backing_store = [0; 4];
        let mut buf = RingBuffer::new(backing_store);
        buf.add(1);
        buf.add(2);
        buf.add(3);
        buf.add(4);
        buf.add(5);
        buf.add(6);
        let expected_values = [3, 4, 5, 6];
        assert_eq!(buf.into_iter().len(), expected_values.len());
        for (idx, val) in (&mut buf).into_iter().enumerate() {
            assert_eq!(val, expected_values[idx]);
        }

        assert_eq!(buf.pop(), Some(3));
        assert_eq!(buf.pop(), Some(4));
        assert_eq!(buf.pop(), Some(5));
        assert_eq!(buf.pop(), Some(6));
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_item_overwrite() {
        let backing_store = [0; 2];
        let mut buf = RingBuffer::new(backing_store);
        for i in 0..100 {
            buf.add(i);
        }
        // Test checks that no panic was triggered
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_exactsize_iter_length_counts() {
        let backing_store = [0; 4];
        let mut buf = RingBuffer::new(backing_store);
        buf.add(1);
        buf.add(2);
        buf.add(3);
        buf.add(4);
        buf.add(5);
        buf.add(6);
        assert_eq!(buf.into_iter().len(), 4);

        buf.pop();
        buf.pop();
        assert_eq!(buf.into_iter().len(), 2);

        buf.add(7);
        buf.add(8);
        assert_eq!(buf.into_iter().len(), 4);

        buf.add(9);
        assert_eq!(buf.into_iter().len(), 4);
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_reverse_iter() {
        let backing_store = [0; 4];
        let mut buf = RingBuffer::new(backing_store);
        for i in 1..5 {
            buf.add(i);
        }
        let expected_values: [DataUnit; 4] = [4, 3, 2, 1];
        for (idx, val) in (&mut buf).into_iter().rev().enumerate() {
            assert_eq!(val, expected_values[idx]);
        }
    }

    #[cfg_attr(target_os = "none", test_case)]
    #[cfg_attr(not(target_os = "none"), test)]
    fn test_non_integer_storage() {
        let backing_store: [(f32, &'static str); 5] = [(3.2, "hi"); 5];
        let mut buf = RingBuffer::new(backing_store);
        for i in 1..5 {
            buf.add((i as f32 * 4.2, "Testing"));
        }
        let expected_values = [
            (4.2, "Testing"),
            (8.4, "Testing"),
            (12.6, "Testing"),
            (16.8, "Testing"),
        ];
        for (idx, val) in (&mut buf).into_iter().enumerate() {
            assert_almost_eq!(val.0, expected_values[idx].0, 1e-3);
            assert_eq!(val.1, expected_values[idx].1);
        }
    }

    fn new_test_buffer() -> RingBuffer<DataUnit, 512> {
        let array = [0_i32; 512];
        RingBuffer::new(array)
    }

    mod mean {
        use super::*;
        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Tests that `mean` still returns accurate calculations when data changes in small
        /// increments.
        fn mean_small_delta() {
            let mut buffer: RingBuffer<DataUnit, 512> = new_test_buffer();
            let n = 15 << 10;
            for idx in n..(n + buffer.capacity()) {
                buffer.add(idx as i32);
            }
            let actual_mean = n as i32 + (buffer.capacity() as i32) / 2 - 1;
            assert_eq!(mean(&mut buffer), actual_mean);
        }

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Test that `mean` calculates the exact mean when data changes in steps of at least 1024.
        fn mean_wide_step_size() {
            let mut buffer: RingBuffer<DataUnit, 512> = new_test_buffer();
            for idx in 0..buffer.capacity() {
                buffer.add((idx as i32) << 10);
            }
            let actual_mean = ((buffer.capacity() as i32) - 1) << 9;
            assert_eq!(mean(&mut buffer), actual_mean);
        }

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Test that `mean` doesn't overflow when summing maximum values.
        fn mean_no_overflow() {
            let mut buffer: RingBuffer<DataUnit, 512> = new_test_buffer();
            let max_val = (0x1 << 30) - 1 as i32;
            for _idx in 0..buffer.capacity() {
                buffer.add(max_val);
            }
            assert_eq!(mean(&mut buffer), max_val);
        }

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Test that `mean` can be calculated over large buffer sizes
        fn mean_larger_buffer() {
            let array = [0_i32; 4096];
            let mut buffer: RingBuffer<DataUnit, 4096> = RingBuffer::new(array);

            let n = 25 << 10;
            for idx in n..(n + buffer.capacity()) {
                buffer.add(idx as i32);
            }
            let actual_mean = n as i32 + (buffer.capacity() as i32) / 2 - 1;
            assert_eq!(mean(&mut buffer), actual_mean);
        }

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Test that `mean` can be calculated over large buffer sizes
        fn test_windowed_mean() {
            let array = [0; 6];
            let mut buffer: RingBuffer<DataUnit, 6> = RingBuffer::new(array);

            let data = [0, 1, 2, 3, 4, 5];
            for val in data.iter() {
                buffer.add(*val as i32);
            }
            assert_eq!(windowed_mean(&mut buffer, 5), 3);
        }
    }

    mod ac_rms {
        use super::*;

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Test accuracy of ac_rms calculation on a "sine wave". In quotes because we are
        /// casting the sine value float to an int, so it's really a digitized sine. Expected
        /// result was calculated in Python.
        fn ac_rms_sine() {
            use libm::sin;

            let func = |val: usize| sin(val as f64) * 1000000_f64;
            let expected_result = 707128.4729105555_f32;

            let mut buffer: RingBuffer<DataUnit, 512> = new_test_buffer();
            let capacity = buffer.capacity();
            for idx in 0..capacity {
                buffer.add(func(idx) as i32);
            }
            let buf_mean = mean(&mut buffer);
            assert_almost_eq!(ac_rms(&mut buffer, buf_mean), expected_result, 0.1);
        }

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Tests that ac_rms does not change when a DC offset is added.
        fn ac_rms_sine_dc_offset() {
            use libm::sin;

            let func = |val: usize| sin(val as f64) * 1000000_f64;
            let expected_result = 707128.4729105555_f32;

            let mut buffer: RingBuffer<DataUnit, 512> = new_test_buffer();
            let capacity = buffer.capacity();
            let offset = 1000000_i32;
            for idx in 0..capacity {
                buffer.add(func(idx) as i32 + offset);
            }
            let buf_mean = mean(&mut buffer);
            assert_almost_eq!(ac_rms(&mut buffer, buf_mean), expected_result, 0.1);
        }

        #[cfg_attr(target_os = "none", test_case)]
        #[cfg_attr(not(target_os = "none"), test)]
        /// Tests that ac_rms does not overflow when processing maximum possible values.
        fn ac_rms_no_overflow() {
            let expected_result = 4194303.0_f32;

            let mut buffer: RingBuffer<DataUnit, 512> = new_test_buffer();
            let capacity = buffer.capacity();
            let max_val: i32 = (0x1 << 22) - 1;
            let min_val: i32 = -1 * ((0x1 << 22) - 1);
            for idx in 0..capacity {
                if idx % 2 == 0 {
                    buffer.add(max_val);
                } else {
                    buffer.add(min_val);
                }
            }
            let buf_mean = mean(&mut buffer);
            assert_almost_eq!(ac_rms(&mut buffer, buf_mean), expected_result, 0.005);
        }
    }
}
