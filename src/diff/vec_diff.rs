use serde::{Serialize, Deserialize};
use crate::diff::Diffable;

// TODO: look into copy-and-insert based diff systems?
// TODO: bounds check, do not panic of botched application

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Op<T> where T: PartialEq + Clone {
    Insert(Vec<T>), // insert Vec<T> at index
    Delete(usize),  // delete the next n items
    Equal(usize),   // skip the next n items.
}

impl<T> Op<T> where T: PartialEq + Clone + std::fmt::Debug {
    /// If an op is of length 0, it contributes nothing to the reconstructed item
    /// this function checks for this case.
    pub fn is_no_op(&self) -> bool {
        match self {
            Op::Insert(a) if a.is_empty() => true,
            Op::Delete(n) if n == &0usize => true,
            Op::Equal(n)  if n == &0usize => true,
            _ => false,
        }
    }

    pub fn join(&self, other: &Op<T>) -> Option<Op<T>> {
        // if the two ops are the same, we combine them together.
        let joined = match self {
            Op::Insert(a) => if let Op::Insert(b)  = other {
                let mut j = a.to_vec();
                j.append(&mut b.to_vec());
                Op::Insert(j)
            } else { None? },
            Op::Delete(n) => if let Op::Delete(m) = other { Op::Delete(n + m) } else { None? },
            Op::Equal(n)  => if let Op::Equal(m)  = other { Op::Equal(n + m)  } else { None? },
        };

        return Some(joined);
    }
}

/// Calculates the diff of a vec of items.
/// Uses the Myers diffing algorithm
/// Applies pre-processing steps to increase efficiency,
/// applies post-processing steps to increase readability.
/// Note that the diff must cover the whole sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VecDiff<T>(Vec<Op<T>>) where T: PartialEq + Clone;

// make t a slice type?

impl<T> VecDiff<T> where T: PartialEq + Clone + std::fmt::Debug {
    // TODO: pre and post processing steps.
    // see: https://github.com/google/diff-match-patch
    // https://neil.fraser.name/writing/diff/myers.pdf
    // and https://neil.fraser.name/writing/diff/ (especially)

    fn new(prefix: usize, postfix: usize, middle: Vec<Op<T>>) -> VecDiff<T> {
        let mut diff = vec![Op::Equal(prefix)];
        diff.append(&mut middle.to_vec());
        diff.push(Op::Equal(postfix));
        diff = VecDiff::compress(diff);
        return VecDiff(diff);
    }

    fn common_prefix(prev: &[T], next: &[T], ) -> usize {
        let (mut prev_iter, mut next_iter) = (prev.iter(), next.iter());
        let mut len = 0;

        while let (Some(p), Some(n)) = (prev_iter.next(), next_iter.next()) {
            if p == n { len += 1; } else { break; }
        }
        return len;
    }

    fn common_postfix(prev: &[T], next: &[T], ) -> usize {
        let (mut prev_iter, mut next_iter) = (prev.iter().rev(), next.iter().rev());
        let mut len = 0;

        while let (Some(p), Some(n)) = (prev_iter.next(), next_iter.next()) {
            if p == n { len += 1; } else { break; }
        }
        return len;
    }

    fn double_edit(prev: &[T], next: &[T]) -> Option<Vec<Op<T>>> {
        let (long, short) = if prev.len() > next.len() {
            (prev, next)
        } else {
            (next, prev)
        };

        let sub_slice = long.windows(short.len()).position(|w| w == short)?;

        let front  = &long[..sub_slice];
        let middle = Op::<T>::Equal(short.len());
        let back   = &long[sub_slice + short.len()..];

        return if prev.len() > next.len() {
            Some(vec![Op::<T>::Delete(front.len()), middle, Op::<T>::Delete(back.len())])
        } else {
            Some(vec![Op::Insert(front.to_vec()), middle, Op::Insert(back.to_vec())])
        }
    }

    fn find(slice: &[T], sub_slice: &[T]) -> Option<usize> {
        if sub_slice.len() == 0 { return None; }
        slice.windows(sub_slice.len())
            .position(|w| w == sub_slice)
    }

    fn half_match_internal<'c, 'd>(long: &'c[T], short: &'d[T], i: usize) -> Option<(&'c [T], &'c [T], &'d [T], &'d [T], usize)> {
        let seed = &long[i..(i + long.len() / 4)];
        let mut best:       &[T] = &[];
        let mut long_pre:   &[T] = &[];
        let mut long_post:  &[T] = &[];
        let mut short_pre:  &[T] = &[];
        let mut short_post: &[T] = &[];
        let mut sub_slice = VecDiff::find(short, seed);

        while let Some(j) = sub_slice {
            let prefix = VecDiff::common_prefix(&long[i..], &short[j..]);
            let postfix = VecDiff::common_postfix(&long[..i], &short[..j]);

            if best.len() < prefix + postfix {
                best       = &short[(j - postfix)..(j + prefix)];
                long_pre   = &long[..(i - postfix)];
                long_post  = &long[(i + prefix)..];
                short_pre  = &short[..(j - postfix)];
                short_post = &short[(j + prefix)..];
                sub_slice = VecDiff::find(&short[(j + 1)..], seed);
            }
        }

        return if best.len() >= long.len() / 2 {
            Some((
                long_pre,
                long_post,
                short_pre,
                short_post,
                best.len(),
            ))
        } else {
            None
        }
    }

    fn half_match<'a>(prev: &'a [T], next: &'a [T]) ->
        Option<(&'a [T], &'a [T], &'a [T], &'a [T], usize)>
    {
        let flip = prev.len() < next.len();
        let (long, short) = if !flip { (prev, next) } else { (next, prev) };

        // pointless
        if long.len() <= 1 || short.len() < 1 { return None; }

        let first_hm  = VecDiff::half_match_internal(long, short, (long.len() + 3) / 4);
        let second_hm = VecDiff::half_match_internal(long, short, (long.len() + 1) / 2);

        let hm = match (first_hm, second_hm) {
            (None,     None    ) => { return None; },
            (None,     Some(hm)) => hm,
            (Some(hm), None    ) => hm,
            (Some(first_hm), Some(second_hm)) => if first_hm.4 > second_hm.4 {
                first_hm
            } else {
                second_hm
            },
        };

        return if !flip {
            Some(hm)
        } else {
            let (a, b, c, d, e) = hm;
            Some((c, d, a, b, e))
        }
    }

    // lcs based on https://blog.robertelder.org/diff-algorithm/

    // TODO: variable names
    fn sub(
        slice:   &[T],
        parity: usize,
        flip:   isize,
        thing:  usize,
    ) -> &T {
        let head = (1 - parity) * slice.len();
        let tail = flip * (thing as isize) + ((parity as isize) - 1);
        let index = (head as isize) + tail;
        return &slice[index as usize];
    }

    fn k_bound(trial: isize, length: isize) -> isize {
        trial - (0.max(trial - length) * 2)
    }

    fn modulo(a: isize, b: usize) -> usize {
        a.rem_euclid(b as isize) as usize
    }

    fn walk(
        prev:      &[T],
        next:      &[T],
        trial:     usize,
        parity:    usize,
        total_len: usize,
        space:     usize,
        g_table:   &mut [usize],
        p_table:   &mut [usize],
    ) -> Option<(usize, usize, usize, usize, usize)> {
        // order the tables depending on the parity of the current iteration
        let (c, d, m) = if parity == 1 {
            (g_table, p_table, 1)
        } else {
            (p_table, g_table, -1)
        };

        // determine the lower and upper k bounds that will be searched
        let k_range = (
              -VecDiff::<T>::k_bound(trial as _, next.len() as _)
            ..(VecDiff::<T>::k_bound(trial as _, prev.len() as _) + 1)
        ).step_by(2);

        // search the k bound range
        for k in k_range {
            // TODO: verify modulo behaviour is the same as python's
            let (a_pos, a_neg) = (c[VecDiff::<T>::modulo(k + 1, space)], c[VecDiff::<T>::modulo(k - 1, space)]);
            let mut a = if k == -(trial as isize)
                        || k != (trial as isize) && a_neg < a_pos
                        { a_pos } else { a_neg + 1 };

            let mut b = ((a as isize) - k) as usize;
            let (a_old, b_old) = (a, b);

            // determine the number of same characters
            while a < prev.len()
               && b < next.len()
               && VecDiff::sub(prev, parity, m, a)
               == VecDiff::sub(next, parity, m, b)
            {
                a = a + 1;
                b = b + 1;
            }

            c[VecDiff::<T>::modulo(k, space)] = a;
            let z = -(k - ((prev.len() as isize) - (next.len() as isize)));
            let range = (trial as isize) - (parity as isize);

            if total_len % 2 == parity
            && -range <= z && z <= range
            && c[VecDiff::<T>::modulo(k, space)] + d[VecDiff::<T>::modulo(z, space)] >= prev.len() {
                if parity == 1 {
                    return Some((2 * trial - 1, a_old, b_old, a, b));
                } else {
                    return Some((
                        2 * trial,
                        prev.len() - a,
                        next.len() - b,
                        prev.len() - a_old,
                        next.len() - b_old,
                    ));
                }
            }
        }

        return None;
    }

    // TODO: struct for return type?
    fn snake(prev: &[T], next: &[T]) -> (usize, usize, usize, usize, usize) {
        let total_len = prev.len() + next.len();
        let space = 2 * prev.len().min(next.len()) + 2;

        let mut g_table = vec![0; space];
        let mut p_table = vec![0; space];

        // TODO: divide and round?
        let trials = 1 + total_len / 2 + if total_len % 2 == 0 { 0 } else { 1 };

        for trial in 0..trials {
            for parity in &[1, 0] {

                if let Some(result) = VecDiff::walk(
                    prev, next,
                    trial, *parity,
                    total_len, space,
                    &mut g_table, &mut p_table,
                ) {
                    return result;
                }
            }
        }

        unreachable!()
    }

    // TODO: swap so longer is first?
    fn lcs(prev: &[T], next: &[T]) -> Vec<Op<T>> {
        // return early if one is empty
        if prev.is_empty() && next.is_empty() {
            return vec![];
        } else if !prev.is_empty() && next.is_empty() {
            return vec![Op::Delete(prev.len())];
        } else if !next.is_empty() && prev.is_empty() {
            return vec![Op::Insert(next.to_vec())];
        }

        // find the 'distance' between the two texts
        let (d, x, y, u, v) = VecDiff::snake(prev, next);
        let mut diff = vec![];

        // recursively divide-and-conquer the diff
        if d > 1 || (x != u && y != v) {
            diff.append(&mut VecDiff::lcs(&prev[..x], &next[..y]));
            diff.push(Op::Equal(u - x)); // same as v - y
            diff.append(&mut VecDiff::lcs(&prev[u..], &next[v..]));
        } else if next.len() > prev.len() {
            diff.push(Op::Equal(prev.len()));
            diff.append(&mut VecDiff::lcs(&vec![], &next[prev.len()..]));
        } else if prev.len() > next.len() {
            diff.push(Op::Equal(next.len()));
            diff.append(&mut VecDiff::lcs(&prev[next.len()..], &vec![]));
        } else {
            diff.push(Op::Equal(prev.len())); // same as next.len()
        }

        // return the constructed difference
        return diff;
    }

    fn compress(ops: Vec<Op<T>>) -> Vec<Op<T>> {
        let mut prev: Option<Op<T>> = None;
        let mut new = vec![];

        for op in ops.into_iter().filter(|o| !o.is_no_op()) {
            prev = if let Some(prev_op) = prev {
                if let Some(combined) = prev_op.join(&op) { Some(combined) }
                else { new.push(prev_op); Some(op) }
            } else {
                Some(op)
            }
        }

        if let Some(op) = prev { new.push(op) };
        return new;
    }

    // TODO: move pre-processing to different function?
    // TODO: if diff will take a long time to calculate, delete all then insert all.

    fn make(prev: &[T], next: &[T]) -> VecDiff<T> {
        // if they're equal, there's no change...
        if prev == next { return VecDiff(vec![]) }

        // trim any matching data at the start and end of the slices.
        let prefix = VecDiff::common_prefix(prev, next);
        let prev = &prev[prefix..];
        let next = &next[prefix..];

        let postfix = VecDiff::common_postfix(prev, next);
        let prev = &prev[..prev.len() - postfix];
        let next = &next[..next.len() - postfix];

        // single insertions and deletions are easy to handle at this point.
        if prev.is_empty() {
            return VecDiff::new(prefix, postfix, vec![Op::Insert(next.to_owned())]);
        } else if next.is_empty() {
            return VecDiff::new(prefix, postfix, vec![Op::Delete(prev.len())]);
        }

        // double insertions are a bit more complicated...
        if let Some(edits) = VecDiff::double_edit(prev, next) {
            return VecDiff::new(prefix, postfix, edits);
        }

        // half-length preprocessing step
        // recursively subdivides the problem, allowing for faster processing in certain cases
        if let Some(hm) = VecDiff::half_match(prev, next) {
            let (prev_pre, prev_post, next_pre, next_post, common) = hm;

            let mut diff = vec![];
            diff.append(&mut VecDiff::make(prev_pre, next_pre).0);
            diff.push(Op::Equal(common));
            diff.append(&mut VecDiff::make(prev_post, next_post).0);
            return VecDiff::new(prefix, postfix, diff);
        }

        // longest common subsequence,
        // based on 'An O(ND) Difference Algorithm and Its Variations'
        let middle = VecDiff::lcs(prev, next);

        // TODO: post-processing cleanup
        return VecDiff::new(prefix, postfix, middle);
    }

    fn apply(&self, prev: &[T]) -> Vec<T> {
        let mut head = 0;
        let mut next = vec![];

        for op in self.0.iter() {
            match op {
                Op::Equal(n) => {
                    next.append(&mut prev[head..(head + n)].to_vec());
                    head += n;
                },
                Op::Delete(n) => { head += n; },
                Op::Insert(i) => { next.append(&mut i.to_vec()) },
            }
        }

        return next;
    }
}

impl<T> Diffable for Vec<T> where
    T: Clone + PartialEq + std::fmt::Debug
{
    type Diff = VecDiff<T>;

    fn make(prev: &Vec<T>, next: &Vec<T>) -> VecDiff<T> {
        VecDiff::make(&prev, &next)
    }

    fn apply(prev: &Vec<T>, diff: &VecDiff<T>) -> Vec<T> {
        VecDiff::apply(diff, prev)
    }
}
