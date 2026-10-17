//! Work-stealing segment map.
//!
//! A segment is a byte range owned by at most one connection. When a
//! connection becomes free it first takes an unowned segment; if there is none
//! it splits the segment with the most bytes left and takes the second half
//! (the "dynamic segmentation" idea). The victim keeps streaming its original
//! response and simply stops at the new, shorter end.
//!
//! Every segment has three cursors: `start <= written <= reserved <= end`.
//! `reserved` advances (under the map lock) before bytes are buffered, so a
//! split can never hand out bytes a worker already claimed. `written` advances
//! after those bytes reach the file and is what gets checkpointed.

use serde::{Deserialize, Serialize};

pub type SegmentId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    pub const fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub id: SegmentId,
    pub start: u64,
    pub written: u64,
    pub reserved: u64,
    pub end: u64,
    pub active: bool,
    pub failures: u32,
}

impl Segment {
    /// Bytes nobody has claimed yet.
    pub fn unclaimed(&self) -> u64 {
        self.end.saturating_sub(self.reserved)
    }

    pub fn is_done(&self) -> bool {
        self.written >= self.end
    }
}

/// Read-only view for progress displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentView {
    pub start: u64,
    pub end: u64,
    pub written: u64,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assignment {
    pub id: SegmentId,
    pub start: u64,
    pub end: u64,
    /// True when this assignment was carved out of another active segment.
    pub stolen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservation {
    pub offset: u64,
    pub take: u64,
    pub exhausted: bool,
}

#[derive(Debug, Clone)]
pub struct SegmentMap {
    total: u64,
    /// Ranges finished in earlier sessions; they never change.
    restored: Vec<ByteRange>,
    segments: Vec<Segment>,
    next_id: SegmentId,
    split_alignment: u64,
}

impl SegmentMap {
    /// Build a map for `total` bytes where `completed` is already on disk.
    pub fn new(total: u64, completed: &[ByteRange]) -> Self {
        let restored = normalize(completed, total);
        let mut map = Self {
            total,
            restored: restored.clone(),
            segments: Vec::new(),
            next_id: 0,
            split_alignment: 64 * 1024,
        };
        for gap in complement(&restored, total) {
            map.push(gap.start, gap.end);
        }
        map
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn is_complete(&self) -> bool {
        self.segments.iter().all(Segment::is_done)
    }

    pub fn active_count(&self) -> usize {
        self.segments.iter().filter(|s| s.active).count()
    }

    pub fn written_bytes(&self) -> u64 {
        let restored: u64 = self.restored.iter().map(ByteRange::len).sum();
        restored
            + self
                .segments
                .iter()
                .map(|s| s.written - s.start)
                .sum::<u64>()
    }

    /// Whether [`assign`](Self::assign) would hand out work right now.
    pub fn has_assignable_work(&self, min_split: u64) -> bool {
        self.segments
            .iter()
            .any(|s| !s.active && !s.is_done() && s.failures == 0)
            || self.best_victim(min_split).is_some()
    }

    /// Give a free connection something to do: an idle segment first,
    /// otherwise half of the largest active one. Segments that recently
    /// failed are skipped here; the controller re-arms them after backoff.
    pub fn assign(&mut self, min_split: u64) -> Option<Assignment> {
        if let Some(seg) = self
            .segments
            .iter_mut()
            .filter(|s| !s.active && !s.is_done() && s.failures == 0)
            .max_by_key(|s| s.unclaimed())
        {
            seg.active = true;
            return Some(Assignment {
                id: seg.id,
                start: seg.reserved,
                end: seg.end,
                stolen: false,
            });
        }
        self.steal(min_split)
    }

    /// Re-activate a specific idle segment (used for retries).
    pub fn claim(&mut self, id: SegmentId) -> Option<Assignment> {
        let seg = self.get_mut(id)?;
        if seg.active || seg.is_done() {
            return None;
        }
        seg.active = true;
        Some(Assignment {
            id,
            start: seg.reserved,
            end: seg.end,
            stolen: false,
        })
    }

    fn best_victim(&self, min_split: u64) -> Option<usize> {
        let min_split = min_split.max(1);
        self.segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.active && s.unclaimed() >= min_split.saturating_mul(2))
            .max_by_key(|(_, s)| s.unclaimed())
            .map(|(i, _)| i)
    }

    fn steal(&mut self, min_split: u64) -> Option<Assignment> {
        let idx = self.best_victim(min_split)?;
        let victim = &self.segments[idx];
        let mut mid = victim.reserved + victim.unclaimed() / 2;
        let aligned = mid - mid % self.split_alignment;
        if aligned >= victim.reserved + min_split.max(1) {
            mid = aligned;
        }
        let end = victim.end;
        self.segments[idx].end = mid;
        let id = self.push(mid, end);
        let seg = self.get_mut(id)?;
        seg.active = true;
        Some(Assignment {
            id,
            start: mid,
            end,
            stolen: true,
        })
    }

    /// Claim up to `len` bytes for the segment's owner. `take < len` or
    /// `exhausted` means the segment end was reached (possibly because the
    /// segment was split meanwhile) and the owner should stop reading.
    pub fn reserve(&mut self, id: SegmentId, len: u64) -> Reservation {
        let Some(seg) = self.get_mut(id) else {
            return Reservation {
                offset: 0,
                take: 0,
                exhausted: true,
            };
        };
        let offset = seg.reserved;
        let take = len.min(seg.unclaimed());
        seg.reserved += take;
        Reservation {
            offset,
            take,
            exhausted: seg.unclaimed() == 0,
        }
    }

    /// Bytes up to `upto` are now in the file.
    pub fn commit(&mut self, id: SegmentId, upto: u64) {
        if let Some(seg) = self.get_mut(id) {
            debug_assert!(upto <= seg.reserved, "commit beyond reservation");
            let upto = upto.min(seg.reserved);
            if upto > seg.written {
                seg.written = upto;
                // Progress resets the retry budget: a flaky connection that
                // keeps delivering data should not exhaust it.
                seg.failures = 0;
            }
        }
    }

    /// The owner is gone. Anything reserved but not written is handed back.
    pub fn release(&mut self, id: SegmentId, failed: bool) -> u32 {
        let Some(seg) = self.get_mut(id) else {
            return 0;
        };
        seg.active = false;
        seg.reserved = seg.written;
        if failed {
            seg.failures += 1;
        }
        seg.failures
    }

    /// Clear the failure mark so [`assign`](Self::assign) picks it up again.
    pub fn rearm(&mut self, id: SegmentId) {
        if let Some(seg) = self.get_mut(id) {
            seg.failures = 0;
        }
    }

    /// Clear every failure mark on idle segments.
    pub fn rearm_all(&mut self) {
        for seg in self.segments.iter_mut().filter(|s| !s.active) {
            seg.failures = 0;
        }
    }

    pub fn failures(&self, id: SegmentId) -> u32 {
        self.segments
            .iter()
            .find(|s| s.id == id)
            .map_or(0, |s| s.failures)
    }

    pub fn is_segment_done(&self, id: SegmentId) -> bool {
        self.segments
            .iter()
            .find(|s| s.id == id)
            .is_none_or(Segment::is_done)
    }

    /// Everything durably on disk, merged. This is what goes into the meta.
    pub fn completed_ranges(&self) -> Vec<ByteRange> {
        let mut ranges = self.restored.clone();
        ranges.extend(
            self.segments
                .iter()
                .map(|s| ByteRange::new(s.start, s.written))
                .filter(|r| !r.is_empty()),
        );
        normalize(&ranges, self.total)
    }

    pub fn views(&self) -> Vec<SegmentView> {
        let mut views: Vec<SegmentView> = self
            .restored
            .iter()
            .map(|r| SegmentView {
                start: r.start,
                end: r.end,
                written: r.end,
                active: false,
            })
            .chain(self.segments.iter().map(|s| SegmentView {
                start: s.start,
                end: s.end,
                written: s.written,
                active: s.active,
            }))
            .collect();
        views.sort_by_key(|v| v.start);
        // Finished neighbours carry no information for a progress bar; merge
        // them so the view stays small however many splits happened.
        let mut merged: Vec<SegmentView> = Vec::with_capacity(views.len());
        for v in views {
            match merged.last_mut() {
                Some(last)
                    if !last.active
                        && !v.active
                        && last.written == last.end
                        && v.written == v.end
                        && last.end == v.start =>
                {
                    last.end = v.end;
                    last.written = v.end;
                }
                _ => merged.push(v),
            }
        }
        merged
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    fn push(&mut self, start: u64, end: u64) -> SegmentId {
        let id = self.next_id;
        self.next_id += 1;
        self.segments.push(Segment {
            id,
            start,
            written: start,
            reserved: start,
            end,
            active: false,
            failures: 0,
        });
        id
    }

    fn get_mut(&mut self, id: SegmentId) -> Option<&mut Segment> {
        self.segments.iter_mut().find(|s| s.id == id)
    }
}

/// Sort, clip to `[0, total)`, and merge overlapping/adjacent ranges.
pub fn normalize(ranges: &[ByteRange], total: u64) -> Vec<ByteRange> {
    let mut sorted: Vec<ByteRange> = ranges
        .iter()
        .map(|r| ByteRange::new(r.start.min(total), r.end.min(total)))
        .filter(|r| !r.is_empty())
        .collect();
    sorted.sort();
    let mut merged: Vec<ByteRange> = Vec::with_capacity(sorted.len());
    for r in sorted {
        match merged.last_mut() {
            Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
            _ => merged.push(r),
        }
    }
    merged
}

/// The gaps in `[0, total)` not covered by `ranges` (which must be normalized).
pub fn complement(ranges: &[ByteRange], total: u64) -> Vec<ByteRange> {
    let mut gaps = Vec::new();
    let mut cursor = 0;
    for r in ranges {
        if r.start > cursor {
            gaps.push(ByteRange::new(cursor, r.start));
        }
        cursor = cursor.max(r.end);
    }
    if cursor < total {
        gaps.push(ByteRange::new(cursor, total));
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    #[test]
    fn normalize_and_complement() {
        let ranges = [
            ByteRange::new(50, 60),
            ByteRange::new(0, 10),
            ByteRange::new(10, 20),
            ByteRange::new(55, 70),
            ByteRange::new(90, 200),
        ];
        let norm = normalize(&ranges, 100);
        assert_eq!(
            norm,
            vec![
                ByteRange::new(0, 20),
                ByteRange::new(50, 70),
                ByteRange::new(90, 100)
            ]
        );
        assert_eq!(
            complement(&norm, 100),
            vec![ByteRange::new(20, 50), ByteRange::new(70, 90)]
        );
        assert_eq!(complement(&[], 5), vec![ByteRange::new(0, 5)]);
        assert!(complement(&[ByteRange::new(0, 5)], 5).is_empty());
    }

    #[test]
    fn fresh_map_is_one_segment() {
        let mut map = SegmentMap::new(100 * MIB, &[]);
        let a = map.assign(MIB).unwrap();
        assert_eq!((a.start, a.end, a.stolen), (0, 100 * MIB, false));
    }

    #[test]
    fn second_connection_steals_half() {
        let mut map = SegmentMap::new(100 * MIB, &[]);
        let a = map.assign(MIB).unwrap();
        let b = map.assign(MIB).unwrap();
        assert!(b.stolen);
        assert_eq!(b.start, 50 * MIB);
        assert_eq!(b.end, 100 * MIB);
        assert_eq!(map.segments()[0].end, 50 * MIB);

        // A third connection splits whichever has more left.
        map.reserve(a.id, 40 * MIB);
        let c = map.assign(MIB).unwrap();
        assert_eq!(c.start, 75 * MIB);
        assert_eq!(c.end, 100 * MIB);
    }

    #[test]
    fn reservation_stops_at_split_point() {
        let mut map = SegmentMap::new(10 * MIB, &[]);
        let a = map.assign(MIB).unwrap();
        let r = map.reserve(a.id, MIB);
        assert_eq!((r.offset, r.take, r.exhausted), (0, MIB, false));
        let b = map.assign(MIB).unwrap();
        // Victim had 9 MiB unclaimed: split at 1 + 4.5 MiB, aligned down.
        assert_eq!(b.start, MIB + 9 * MIB / 2);
        let r = map.reserve(a.id, 100 * MIB);
        assert_eq!(r.offset, MIB);
        assert_eq!(r.take, b.start - MIB);
        assert!(r.exhausted);
        let r = map.reserve(a.id, 10);
        assert_eq!((r.offset, r.take, r.exhausted), (b.start, 0, true));
    }

    #[test]
    fn will_not_split_below_minimum() {
        let mut map = SegmentMap::new(3 * MIB, &[]);
        map.assign(2 * MIB).unwrap();
        assert!(map.assign(2 * MIB).is_none());
        assert!(!map.has_assignable_work(2 * MIB));
        assert!(map.assign(MIB).is_some());
    }

    #[test]
    fn release_returns_unwritten_bytes_and_tracks_failures() {
        let mut map = SegmentMap::new(10 * MIB, &[]);
        let a = map.assign(MIB).unwrap();
        map.reserve(a.id, 3 * MIB);
        map.commit(a.id, 2 * MIB);
        assert_eq!(map.release(a.id, true), 1);
        // Failed segments wait for an explicit retry.
        assert!(map.assign(MIB).is_none());
        let again = map.claim(a.id).unwrap();
        assert_eq!(again.start, 2 * MIB);
        map.release(a.id, false);
        map.rearm(a.id);
        assert_eq!(map.assign(MIB).unwrap().start, 2 * MIB);
    }

    #[test]
    fn restored_ranges_are_skipped_and_reported() {
        let done = [ByteRange::new(0, 4 * MIB), ByteRange::new(6 * MIB, 8 * MIB)];
        let mut map = SegmentMap::new(10 * MIB, &done);
        assert_eq!(map.written_bytes(), 6 * MIB);
        let mut starts = vec![];
        while let Some(a) = map.assign(16 * MIB) {
            starts.push((a.start, a.end));
        }
        starts.sort();
        assert_eq!(starts, vec![(4 * MIB, 6 * MIB), (8 * MIB, 10 * MIB)]);
    }

    #[test]
    fn completion_and_checkpoint_ranges() {
        let mut map = SegmentMap::new(8 * MIB, &[]);
        let a = map.assign(MIB).unwrap();
        let b = map.assign(MIB).unwrap();
        map.reserve(a.id, 4 * MIB);
        map.commit(a.id, 4 * MIB);
        map.reserve(b.id, MIB);
        map.commit(b.id, 4 * MIB + MIB);
        assert!(!map.is_complete());
        assert_eq!(map.completed_ranges(), vec![ByteRange::new(0, 5 * MIB)]);
        map.reserve(b.id, 3 * MIB);
        map.commit(b.id, 8 * MIB);
        assert!(map.is_complete());
        assert_eq!(map.completed_ranges(), vec![ByteRange::new(0, 8 * MIB)]);
        assert_eq!(map.written_bytes(), 8 * MIB);
    }

    #[test]
    fn empty_file_is_complete() {
        let mut map = SegmentMap::new(0, &[]);
        assert!(map.is_complete());
        assert!(map.assign(MIB).is_none());
    }

    #[test]
    fn views_are_sorted() {
        let mut map = SegmentMap::new(10 * MIB, &[ByteRange::new(0, MIB)]);
        map.assign(MIB);
        map.assign(MIB);
        let views = map.views();
        assert!(views.windows(2).all(|w| w[0].start <= w[1].start));
        assert_eq!(views.len(), 3);
    }
}
