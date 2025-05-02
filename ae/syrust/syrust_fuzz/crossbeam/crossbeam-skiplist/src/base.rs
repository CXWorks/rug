//! TODO: docs
use alloc::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use core::borrow::Borrow;
use core::cmp;
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Bound, Deref, Index, RangeBounds};
use core::ptr;
use core::sync::atomic::{fence, AtomicUsize, Ordering};
use crate::epoch::{self, Atomic, Collector, Guard, Shared};
use crate::utils::CachePadded;
/// Number of bits needed to store height.
const HEIGHT_BITS: usize = 5;
/// Maximum height of a skip list tower.
const MAX_HEIGHT: usize = 1 << HEIGHT_BITS;
/// The bits of `refs_and_height` that keep the height.
const HEIGHT_MASK: usize = (1 << HEIGHT_BITS) - 1;
/// The tower of atomic pointers.
///
/// The actual size of the tower will vary depending on the height that a node
/// was allocated with.
#[repr(C)]
struct Tower<K, V> {
    pointers: [Atomic<Node<K, V>>; 0],
}
impl<K, V> Index<usize> for Tower<K, V> {
    type Output = Atomic<Node<K, V>>;
    fn index(&self, index: usize) -> &Atomic<Node<K, V>> {
        unsafe { self.pointers.get_unchecked(index) }
    }
}
/// Tower at the head of a skip list.
///
/// This is located in the `SkipList` struct itself and holds a full height
/// tower.
#[repr(C)]
struct Head<K, V> {
    pointers: [Atomic<Node<K, V>>; MAX_HEIGHT],
}
impl<K, V> Head<K, V> {
    /// Initializes a `Head`.
    #[inline]
    fn new() -> Head<K, V> {
        Head {
            pointers: Default::default(),
        }
    }
}
impl<K, V> Deref for Head<K, V> {
    type Target = Tower<K, V>;
    fn deref(&self) -> &Tower<K, V> {
        unsafe { &*(self as *const _ as *const Tower<K, V>) }
    }
}
/// A skip list node.
///
/// This struct is marked with `repr(C)` so that the specific order of fields is enforced.
/// It is important that the tower is the last field since it is dynamically sized. The key,
/// reference count, and height are kept close to the tower to improve cache locality during
/// skip list traversal.
#[repr(C)]
struct Node<K, V> {
    /// The value.
    value: V,
    /// The key.
    key: K,
    /// Keeps the reference count and the height of its tower.
    ///
    /// The reference count is equal to the number of `Entry`s pointing to this node, plus the
    /// number of levels in which this node is installed.
    refs_and_height: AtomicUsize,
    /// The tower of atomic pointers.
    tower: Tower<K, V>,
}
impl<K, V> Node<K, V> {
    /// Allocates a node.
    ///
    /// The returned node will start with reference count of `ref_count` and the tower will be initialized
    /// with null pointers. However, the key and the value will be left uninitialized, and that is
    /// why this function is unsafe.
    unsafe fn alloc(height: usize, ref_count: usize) -> *mut Self {
        let layout = Self::get_layout(height);
        let ptr = alloc(layout) as *mut Self;
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        ptr::write(
            &mut (*ptr).refs_and_height,
            AtomicUsize::new((height - 1) | ref_count << HEIGHT_BITS),
        );
        ptr::write_bytes((*ptr).tower.pointers.as_mut_ptr(), 0, height);
        ptr
    }
    /// Deallocates a node.
    ///
    /// This function will not run any destructors.
    unsafe fn dealloc(ptr: *mut Self) {
        let height = (*ptr).height();
        let layout = Self::get_layout(height);
        dealloc(ptr as *mut u8, layout);
    }
    /// Returns the layout of a node with the given `height`.
    unsafe fn get_layout(height: usize) -> Layout {
        assert!(1 <= height && height <= MAX_HEIGHT);
        let size_self = mem::size_of::<Self>();
        let align_self = mem::align_of::<Self>();
        let size_pointer = mem::size_of::<Atomic<Self>>();
        Layout::from_size_align_unchecked(size_self + size_pointer * height, align_self)
    }
    /// Returns the height of this node's tower.
    #[inline]
    fn height(&self) -> usize {
        (self.refs_and_height.load(Ordering::Relaxed) & HEIGHT_MASK) + 1
    }
    /// Marks all pointers in the tower and returns `true` if the level 0 was not marked.
    fn mark_tower(&self) -> bool {
        let height = self.height();
        for level in (0..height).rev() {
            let tag = unsafe {
                self
                    .tower[level]
                    .fetch_or(1, Ordering::SeqCst, epoch::unprotected())
                    .tag()
            };
            if level == 0 && tag == 1 {
                return false;
            }
        }
        true
    }
    /// Returns `true` if the node is removed.
    #[inline]
    fn is_removed(&self) -> bool {
        let tag = unsafe {
            self.tower[0].load(Ordering::Relaxed, epoch::unprotected()).tag()
        };
        tag == 1
    }
    /// Attempts to increment the reference count of a node and returns `true` on success.
    ///
    /// The reference count can be incremented only if it is non-zero.
    ///
    /// # Panics
    ///
    /// Panics if the reference count overflows.
    #[inline]
    unsafe fn try_increment(&self) -> bool {
        let mut refs_and_height = self.refs_and_height.load(Ordering::Relaxed);
        loop {
            if refs_and_height & !HEIGHT_MASK == 0 {
                return false;
            }
            let new_refs_and_height = refs_and_height
                .checked_add(1 << HEIGHT_BITS)
                .expect("SkipList reference count overflow");
            match self
                .refs_and_height
                .compare_exchange_weak(
                    refs_and_height,
                    new_refs_and_height,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
            {
                Ok(_) => return true,
                Err(current) => refs_and_height = current,
            }
        }
    }
    /// Decrements the reference count of a node, destroying it if the count becomes zero.
    #[inline]
    unsafe fn decrement(&self, guard: &Guard) {
        if self.refs_and_height.fetch_sub(1 << HEIGHT_BITS, Ordering::Release)
            >> HEIGHT_BITS == 1
        {
            fence(Ordering::Acquire);
            guard.defer_unchecked(move || Self::finalize(self));
        }
    }
    /// Decrements the reference count of a node, pinning the thread and destoying the node
    /// if the count become zero.
    #[inline]
    unsafe fn decrement_with_pin<F>(&self, parent: &SkipList<K, V>, pin: F)
    where
        F: FnOnce() -> Guard,
    {
        if self.refs_and_height.fetch_sub(1 << HEIGHT_BITS, Ordering::Release)
            >> HEIGHT_BITS == 1
        {
            fence(Ordering::Acquire);
            let guard = &pin();
            parent.check_guard(guard);
            guard.defer_unchecked(move || Self::finalize(self));
        }
    }
    /// Drops the key and value of a node, then deallocates it.
    #[cold]
    unsafe fn finalize(ptr: *const Self) {
        let ptr = ptr as *mut Self;
        ptr::drop_in_place(&mut (*ptr).key);
        ptr::drop_in_place(&mut (*ptr).value);
        Node::dealloc(ptr);
    }
}
impl<K, V> fmt::Debug for Node<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Node").field(&self.key).field(&self.value).finish()
    }
}
/// A search result.
///
/// The result indicates whether the key was found, as well as what were the adjacent nodes to the
/// key on each level of the skip list.
struct Position<'a, K, V> {
    /// Reference to a node with the given key, if found.
    ///
    /// If this is `Some` then it will point to the same node as `right[0]`.
    found: Option<&'a Node<K, V>>,
    /// Adjacent nodes with smaller keys (predecessors).
    left: [&'a Tower<K, V>; MAX_HEIGHT],
    /// Adjacent nodes with equal or greater keys (successors).
    right: [Shared<'a, Node<K, V>>; MAX_HEIGHT],
}
/// Frequently modified data associated with a skip list.
struct HotData {
    /// The seed for random height generation.
    seed: AtomicUsize,
    /// The number of entries in the skip list.
    len: AtomicUsize,
    /// Highest tower currently in use. This value is used as a hint for where
    /// to start lookups and never decreases.
    max_height: AtomicUsize,
}
/// A lock-free skip list.
pub struct SkipList<K, V> {
    /// The head of the skip list (just a dummy node, not a real entry).
    head: Head<K, V>,
    /// The `Collector` associated with this skip list.
    collector: Collector,
    /// Hot data associated with the skip list, stored in a dedicated cache line.
    hot_data: CachePadded<HotData>,
}
unsafe impl<K: Send + Sync, V: Send + Sync> Send for SkipList<K, V> {}
unsafe impl<K: Send + Sync, V: Send + Sync> Sync for SkipList<K, V> {}
impl<K, V> SkipList<K, V> {
    /// Returns a new, empty skip list.
    pub fn new(collector: Collector) -> SkipList<K, V> {
        SkipList {
            head: Head::new(),
            collector,
            hot_data: CachePadded::new(HotData {
                seed: AtomicUsize::new(1),
                len: AtomicUsize::new(0),
                max_height: AtomicUsize::new(1),
            }),
        }
    }
    /// Returns `true` if the skip list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Returns the number of entries in the skip list.
    ///
    /// If the skip list is being concurrently modified, consider the returned number just an
    /// approximation without any guarantees.
    pub fn len(&self) -> usize {
        let len = self.hot_data.len.load(Ordering::Relaxed);
        if len > isize::max_value() as usize { 0 } else { len }
    }
    /// Ensures that all `Guard`s used with the skip list come from the same
    /// `Collector`.
    fn check_guard(&self, guard: &Guard) {
        if let Some(c) = guard.collector() {
            assert!(c == & self.collector);
        }
    }
}
impl<K, V> SkipList<K, V>
where
    K: Ord,
{
    /// Returns the entry with the smallest key.
    pub fn front<'a: 'g, 'g>(&'a self, guard: &'g Guard) -> Option<Entry<'a, 'g, K, V>> {
        self.check_guard(guard);
        let n = self.next_node(&self.head, Bound::Unbounded, guard)?;
        Some(Entry {
            parent: self,
            node: n,
            guard,
        })
    }
    /// Returns the entry with the largest key.
    pub fn back<'a: 'g, 'g>(&'a self, guard: &'g Guard) -> Option<Entry<'a, 'g, K, V>> {
        self.check_guard(guard);
        let n = self.search_bound(Bound::Unbounded, true, guard)?;
        Some(Entry {
            parent: self,
            node: n,
            guard,
        })
    }
    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key<Q>(&self, key: &Q, guard: &Guard) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get(key, guard).is_some()
    }
    /// Returns an entry with the specified `key`.
    pub fn get<'a: 'g, 'g, Q>(
        &'a self,
        key: &Q,
        guard: &'g Guard,
    ) -> Option<Entry<'a, 'g, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.check_guard(guard);
        let n = self.search_bound(Bound::Included(key), false, guard)?;
        if n.key.borrow() != key {
            return None;
        }
        Some(Entry {
            parent: self,
            node: n,
            guard,
        })
    }
    /// Returns an `Entry` pointing to the lowest element whose key is above
    /// the given bound. If no such element is found then `None` is
    /// returned.
    pub fn lower_bound<'a: 'g, 'g, Q>(
        &'a self,
        bound: Bound<&Q>,
        guard: &'g Guard,
    ) -> Option<Entry<'a, 'g, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.check_guard(guard);
        let n = self.search_bound(bound, false, guard)?;
        Some(Entry {
            parent: self,
            node: n,
            guard,
        })
    }
    /// Returns an `Entry` pointing to the highest element whose key is below
    /// the given bound. If no such element is found then `None` is
    /// returned.
    pub fn upper_bound<'a: 'g, 'g, Q>(
        &'a self,
        bound: Bound<&Q>,
        guard: &'g Guard,
    ) -> Option<Entry<'a, 'g, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.check_guard(guard);
        let n = self.search_bound(bound, true, guard)?;
        Some(Entry {
            parent: self,
            node: n,
            guard,
        })
    }
    /// Finds an entry with the specified key, or inserts a new `key`-`value` pair if none exist.
    pub fn get_or_insert(&self, key: K, value: V, guard: &Guard) -> RefEntry<'_, K, V> {
        self.insert_internal(key, value, false, guard)
    }
    /// Returns an iterator over all entries in the skip list.
    pub fn iter<'a: 'g, 'g>(&'a self, guard: &'g Guard) -> Iter<'a, 'g, K, V> {
        self.check_guard(guard);
        Iter {
            parent: self,
            head: None,
            tail: None,
            guard,
        }
    }
    /// Returns an iterator over all entries in the skip list.
    pub fn ref_iter(&self) -> RefIter<'_, K, V> {
        RefIter {
            parent: self,
            head: None,
            tail: None,
        }
    }
    /// Returns an iterator over a subset of entries in the skip list.
    pub fn range<'a: 'g, 'g, Q, R>(
        &'a self,
        range: R,
        guard: &'g Guard,
    ) -> Range<'a, 'g, Q, R, K, V>
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        self.check_guard(guard);
        Range {
            parent: self,
            head: None,
            tail: None,
            range,
            guard,
            _marker: PhantomData,
        }
    }
    /// Returns an iterator over a subset of entries in the skip list.
    #[allow(clippy::needless_lifetimes)]
    pub fn ref_range<'a, Q, R>(&'a self, range: R) -> RefRange<'a, Q, R, K, V>
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        RefRange {
            parent: self,
            range,
            head: None,
            tail: None,
            _marker: PhantomData,
        }
    }
    /// Generates a random height and returns it.
    fn random_height(&self) -> usize {
        let mut num = self.hot_data.seed.load(Ordering::Relaxed);
        num ^= num << 13;
        num ^= num >> 17;
        num ^= num << 5;
        self.hot_data.seed.store(num, Ordering::Relaxed);
        let mut height = cmp::min(MAX_HEIGHT, num.trailing_zeros() as usize + 1);
        unsafe {
            while height >= 4
                && self
                    .head[height - 2]
                    .load(Ordering::Relaxed, epoch::unprotected())
                    .is_null()
            {
                height -= 1;
            }
        }
        let mut max_height = self.hot_data.max_height.load(Ordering::Relaxed);
        while height > max_height {
            match self
                .hot_data
                .max_height
                .compare_exchange_weak(
                    max_height,
                    height,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
            {
                Ok(_) => break,
                Err(h) => max_height = h,
            }
        }
        height
    }
    /// If we encounter a deleted node while searching, help with the deletion
    /// by attempting to unlink the node from the list.
    ///
    /// If the unlinking is successful then this function returns the next node
    /// with which the search should continue on the current level.
    #[cold]
    unsafe fn help_unlink<'a>(
        &'a self,
        pred: &'a Atomic<Node<K, V>>,
        curr: &'a Node<K, V>,
        succ: Shared<'a, Node<K, V>>,
        guard: &'a Guard,
    ) -> Option<Shared<'a, Node<K, V>>> {
        match pred
            .compare_and_set(
                Shared::from(curr as *const _),
                succ.with_tag(0),
                Ordering::Release,
                guard,
            )
        {
            Ok(_) => {
                curr.decrement(guard);
                Some(succ.with_tag(0))
            }
            Err(_) => None,
        }
    }
    /// Returns the successor of a node.
    ///
    /// This will keep searching until a non-deleted node is found. If a deleted
    /// node is reached then a search is performed using the given key.
    fn next_node<'a>(
        &'a self,
        pred: &'a Tower<K, V>,
        lower_bound: Bound<&K>,
        guard: &'a Guard,
    ) -> Option<&'a Node<K, V>> {
        unsafe {
            let mut curr = pred[0].load_consume(guard);
            if curr.tag() == 1 {
                return self.search_bound(lower_bound, false, guard);
            }
            while let Some(c) = curr.as_ref() {
                let succ = c.tower[0].load_consume(guard);
                if succ.tag() == 1 {
                    if let Some(c) = self.help_unlink(&pred[0], c, succ, guard) {
                        curr = c;
                        continue;
                    } else {
                        return self.search_bound(lower_bound, false, guard);
                    }
                }
                return Some(c);
            }
            None
        }
    }
    /// Searches for first/last node that is greater/less/equal to a key in the skip list.
    ///
    /// If `upper_bound == true`: the last node less than (or equal to) the key.
    ///
    /// If `upper_bound == false`: the first node greater than (or equal to) the key.
    ///
    /// This is unsafe because the returned nodes are bound to the lifetime of
    /// the `SkipList`, not the `Guard`.
    fn search_bound<'a, Q>(
        &'a self,
        bound: Bound<&Q>,
        upper_bound: bool,
        guard: &'a Guard,
    ) -> Option<&'a Node<K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        unsafe {
            'search: loop {
                let mut level = self.hot_data.max_height.load(Ordering::Relaxed);
                while level >= 1
                    && self.head[level - 1].load(Ordering::Relaxed, guard).is_null()
                {
                    level -= 1;
                }
                let mut result = None;
                let mut pred = &*self.head;
                while level >= 1 {
                    level -= 1;
                    let mut curr = pred[level].load_consume(guard);
                    if curr.tag() == 1 {
                        continue 'search;
                    }
                    while let Some(c) = curr.as_ref() {
                        let succ = c.tower[level].load_consume(guard);
                        if succ.tag() == 1 {
                            if let Some(c)
                                = self.help_unlink(&pred[level], c, succ, guard)
                            {
                                curr = c;
                                continue;
                            } else {
                                continue 'search;
                            }
                        }
                        if upper_bound {
                            if !below_upper_bound(&bound, c.key.borrow()) {
                                break;
                            }
                            result = Some(c);
                        } else if above_lower_bound(&bound, c.key.borrow()) {
                            result = Some(c);
                            break;
                        }
                        pred = &c.tower;
                        curr = succ;
                    }
                }
                return result;
            }
        }
    }
    /// Searches for a key in the skip list and returns a list of all adjacent nodes.
    fn search_position<'a, Q>(&'a self, key: &Q, guard: &'a Guard) -> Position<'a, K, V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        unsafe {
            'search: loop {
                let mut result = Position {
                    found: None,
                    left: [&*self.head; MAX_HEIGHT],
                    right: [Shared::null(); MAX_HEIGHT],
                };
                let mut level = self.hot_data.max_height.load(Ordering::Relaxed);
                while level >= 1
                    && self.head[level - 1].load(Ordering::Relaxed, guard).is_null()
                {
                    level -= 1;
                }
                let mut pred = &*self.head;
                while level >= 1 {
                    level -= 1;
                    let mut curr = pred[level].load_consume(guard);
                    if curr.tag() == 1 {
                        continue 'search;
                    }
                    while let Some(c) = curr.as_ref() {
                        let succ = c.tower[level].load_consume(guard);
                        if succ.tag() == 1 {
                            if let Some(c)
                                = self.help_unlink(&pred[level], c, succ, guard)
                            {
                                curr = c;
                                continue;
                            } else {
                                continue 'search;
                            }
                        }
                        match c.key.borrow().cmp(key) {
                            cmp::Ordering::Greater => break,
                            cmp::Ordering::Equal => {
                                result.found = Some(c);
                                break;
                            }
                            cmp::Ordering::Less => {}
                        }
                        pred = &c.tower;
                        curr = succ;
                    }
                    result.left[level] = pred;
                    result.right[level] = curr;
                }
                return result;
            }
        }
    }
    /// Inserts an entry with the specified `key` and `value`.
    ///
    /// If `replace` is `true`, then any existing entry with this key will first be removed.
    fn insert_internal(
        &self,
        key: K,
        value: V,
        replace: bool,
        guard: &Guard,
    ) -> RefEntry<'_, K, V> {
        self.check_guard(guard);
        unsafe {
            let guard = &*(guard as *const _);
            let mut search;
            loop {
                search = self.search_position(&key, guard);
                let r = match search.found {
                    Some(r) => r,
                    None => break,
                };
                if replace {
                    if r.mark_tower() {
                        self.hot_data.len.fetch_sub(1, Ordering::Relaxed);
                    }
                } else {
                    if let Some(e) = RefEntry::try_acquire(self, r) {
                        return e;
                    }
                    break;
                }
            }
            let height = self.random_height();
            let (node, n) = {
                let n = Node::<K, V>::alloc(height, 2);
                ptr::write(&mut (*n).key, key);
                ptr::write(&mut (*n).value, value);
                (Shared::<Node<K, V>>::from(n as *const _), &*n)
            };
            self.hot_data.len.fetch_add(1, Ordering::Relaxed);
            loop {
                n.tower[0].store(search.right[0], Ordering::Relaxed);
                if search
                    .left[0][0]
                    .compare_and_set(search.right[0], node, Ordering::SeqCst, guard)
                    .is_ok()
                {
                    break;
                }
                {
                    let sg = scopeguard::guard(
                        (),
                        |_| {
                            Node::finalize(node.as_raw());
                        },
                    );
                    search = self.search_position(&n.key, guard);
                    mem::forget(sg);
                }
                if let Some(r) = search.found {
                    if replace {
                        if r.mark_tower() {
                            self.hot_data.len.fetch_sub(1, Ordering::Relaxed);
                        }
                    } else {
                        if let Some(e) = RefEntry::try_acquire(self, r) {
                            Node::finalize(node.as_raw());
                            self.hot_data.len.fetch_sub(1, Ordering::Relaxed);
                            return e;
                        }
                    }
                }
            }
            let entry = RefEntry { parent: self, node: n };
            'build: for level in 1..height {
                loop {
                    let pred = search.left[level];
                    let succ = search.right[level];
                    let next = n.tower[level].load(Ordering::SeqCst, guard);
                    if next.tag() == 1 {
                        break 'build;
                    }
                    if succ.as_ref().map(|s| &s.key) == Some(&n.key) {
                        search = self.search_position(&n.key, guard);
                        continue;
                    }
                    if n
                        .tower[level]
                        .compare_and_set(next, succ, Ordering::SeqCst, guard)
                        .is_err()
                    {
                        break 'build;
                    }
                    n.refs_and_height.fetch_add(1 << HEIGHT_BITS, Ordering::Relaxed);
                    if pred[level]
                        .compare_and_set(succ, node, Ordering::SeqCst, guard)
                        .is_ok()
                    {
                        break;
                    }
                    (*n).refs_and_height.fetch_sub(1 << HEIGHT_BITS, Ordering::Relaxed);
                    search = self.search_position(&n.key, guard);
                }
            }
            if n.tower[height - 1].load(Ordering::SeqCst, guard).tag() == 1 {
                self.search_bound(Bound::Included(&n.key), false, guard);
            }
            entry
        }
    }
}
impl<K, V> SkipList<K, V>
where
    K: Ord + Send + 'static,
    V: Send + 'static,
{
    /// Inserts a `key`-`value` pair into the skip list and returns the new entry.
    ///
    /// If there is an existing entry with this key, it will be removed before inserting the new
    /// one.
    pub fn insert(&self, key: K, value: V, guard: &Guard) -> RefEntry<'_, K, V> {
        self.insert_internal(key, value, true, guard)
    }
    /// Removes an entry with the specified `key` from the map and returns it.
    pub fn remove<Q>(&self, key: &Q, guard: &Guard) -> Option<RefEntry<'_, K, V>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.check_guard(guard);
        unsafe {
            let guard = &*(guard as *const _);
            loop {
                let search = self.search_position(key, guard);
                let n = search.found?;
                let entry = match RefEntry::try_acquire(self, n) {
                    Some(e) => e,
                    None => continue,
                };
                if n.mark_tower() {
                    self.hot_data.len.fetch_sub(1, Ordering::Relaxed);
                    for level in (0..n.height()).rev() {
                        let succ = n
                            .tower[level]
                            .load(Ordering::SeqCst, guard)
                            .with_tag(0);
                        if search
                            .left[level][level]
                            .compare_and_set(
                                Shared::from(n as *const _),
                                succ,
                                Ordering::SeqCst,
                                guard,
                            )
                            .is_ok()
                        {
                            n.decrement(guard);
                        } else {
                            self.search_bound(Bound::Included(key), false, guard);
                            break;
                        }
                    }
                    return Some(entry);
                }
            }
        }
    }
    /// Removes an entry from the front of the skip list.
    pub fn pop_front(&self, guard: &Guard) -> Option<RefEntry<'_, K, V>> {
        self.check_guard(guard);
        loop {
            let e = self.front(guard)?;
            if let Some(e) = e.pin() {
                if e.remove(guard) {
                    return Some(e);
                }
            }
        }
    }
    /// Removes an entry from the back of the skip list.
    pub fn pop_back(&self, guard: &Guard) -> Option<RefEntry<'_, K, V>> {
        self.check_guard(guard);
        loop {
            let e = self.back(guard)?;
            if let Some(e) = e.pin() {
                if e.remove(guard) {
                    return Some(e);
                }
            }
        }
    }
    /// Iterates over the map and removes every entry.
    pub fn clear(&self, guard: &mut Guard) {
        self.check_guard(guard);
        /// Number of steps after which we repin the current thread and unlink removed nodes.
        const BATCH_SIZE: usize = 100;
        loop {
            {
                let mut entry = self.lower_bound(Bound::Unbounded, guard);
                for _ in 0..BATCH_SIZE {
                    let e = match entry {
                        None => return,
                        Some(e) => e,
                    };
                    let next = e.next();
                    if e.node.mark_tower() {
                        self.hot_data.len.fetch_sub(1, Ordering::Relaxed);
                    }
                    entry = next;
                }
            }
            guard.repin();
        }
    }
}
impl<K, V> Drop for SkipList<K, V> {
    fn drop(&mut self) {
        unsafe {
            let mut node = self
                .head[0]
                .load(Ordering::Relaxed, epoch::unprotected())
                .as_ref();
            while let Some(n) = node {
                let next = n
                    .tower[0]
                    .load(Ordering::Relaxed, epoch::unprotected())
                    .as_ref();
                Node::finalize(n);
                node = next;
            }
        }
    }
}
impl<K, V> fmt::Debug for SkipList<K, V>
where
    K: Ord + fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("SkipList { .. }")
    }
}
impl<K, V> IntoIterator for SkipList<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> IntoIter<K, V> {
        unsafe {
            let front = self
                .head[0]
                .load(Ordering::Relaxed, epoch::unprotected())
                .as_raw();
            for level in 0..MAX_HEIGHT {
                self.head[level].store(Shared::null(), Ordering::Relaxed);
            }
            IntoIter {
                node: front as *mut Node<K, V>,
            }
        }
    }
}
/// An entry in a skip list, protected by a `Guard`.
///
/// The lifetimes of the key and value are the same as that of the `Guard`
/// used when creating the `Entry` (`'g`). This lifetime is also constrained to
/// not outlive the `SkipList`.
pub struct Entry<'a: 'g, 'g, K, V> {
    parent: &'a SkipList<K, V>,
    node: &'g Node<K, V>,
    guard: &'g Guard,
}
impl<'a: 'g, 'g, K: 'a, V: 'a> Entry<'a, 'g, K, V> {
    /// Returns `true` if the entry is removed from the skip list.
    pub fn is_removed(&self) -> bool {
        self.node.is_removed()
    }
    /// Returns a reference to the key.
    pub fn key(&self) -> &'g K {
        &self.node.key
    }
    /// Returns a reference to the value.
    pub fn value(&self) -> &'g V {
        &self.node.value
    }
    /// Returns a reference to the parent `SkipList`
    pub fn skiplist(&self) -> &'a SkipList<K, V> {
        self.parent
    }
    /// Attempts to pin the entry with a reference count, ensuring that it
    /// remains accessible even after the `Guard` is dropped.
    ///
    /// This method may return `None` if the reference count is already 0 and
    /// the node has been queued for deletion.
    pub fn pin(&self) -> Option<RefEntry<'a, K, V>> {
        unsafe { RefEntry::try_acquire(self.parent, self.node) }
    }
}
impl<'a: 'g, 'g, K, V> Entry<'a, 'g, K, V>
where
    K: Ord + Send + 'static,
    V: Send + 'static,
{
    /// Removes the entry from the skip list.
    ///
    /// Returns `true` if this call removed the entry and `false` if it was already removed.
    pub fn remove(&self) -> bool {
        if self.node.mark_tower() {
            self.parent.hot_data.len.fetch_sub(1, Ordering::Relaxed);
            self.parent.search_bound(Bound::Included(&self.node.key), false, self.guard);
            true
        } else {
            false
        }
    }
}
impl<'a: 'g, 'g, K, V> Clone for Entry<'a, 'g, K, V> {
    fn clone(&self) -> Entry<'a, 'g, K, V> {
        Entry {
            parent: self.parent,
            node: self.node,
            guard: self.guard,
        }
    }
}
impl<K, V> fmt::Debug for Entry<'_, '_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Entry").field(self.key()).field(self.value()).finish()
    }
}
impl<'a: 'g, 'g, K, V> Entry<'a, 'g, K, V>
where
    K: Ord,
{
    /// Moves to the next entry in the skip list.
    pub fn move_next(&mut self) -> bool {
        match self.next() {
            None => false,
            Some(n) => {
                *self = n;
                true
            }
        }
    }
    /// Returns the next entry in the skip list.
    pub fn next(&self) -> Option<Entry<'a, 'g, K, V>> {
        let n = self
            .parent
            .next_node(&self.node.tower, Bound::Excluded(&self.node.key), self.guard)?;
        Some(Entry {
            parent: self.parent,
            node: n,
            guard: self.guard,
        })
    }
    /// Moves to the previous entry in the skip list.
    pub fn move_prev(&mut self) -> bool {
        match self.prev() {
            None => false,
            Some(n) => {
                *self = n;
                true
            }
        }
    }
    /// Returns the previous entry in the skip list.
    pub fn prev(&self) -> Option<Entry<'a, 'g, K, V>> {
        let n = self
            .parent
            .search_bound(Bound::Excluded(&self.node.key), true, self.guard)?;
        Some(Entry {
            parent: self.parent,
            node: n,
            guard: self.guard,
        })
    }
}
/// A reference-counted entry in a skip list.
///
/// You *must* call `release` to free this type, otherwise the node will be
/// leaked. This is because releasing the entry requires a `Guard`.
pub struct RefEntry<'a, K, V> {
    parent: &'a SkipList<K, V>,
    node: &'a Node<K, V>,
}
impl<'a, K: 'a, V: 'a> RefEntry<'a, K, V> {
    /// Returns `true` if the entry is removed from the skip list.
    pub fn is_removed(&self) -> bool {
        self.node.is_removed()
    }
    /// Returns a reference to the key.
    pub fn key(&self) -> &K {
        &self.node.key
    }
    /// Returns a reference to the value.
    pub fn value(&self) -> &V {
        &self.node.value
    }
    /// Returns a reference to the parent `SkipList`
    pub fn skiplist(&self) -> &'a SkipList<K, V> {
        self.parent
    }
    /// Releases the reference on the entry.
    pub fn release(self, guard: &Guard) {
        self.parent.check_guard(guard);
        unsafe { self.node.decrement(guard) }
    }
    /// Releases the reference of the entry, pinning the thread only when
    /// the reference count of the node becomes 0.
    pub fn release_with_pin<F>(self, pin: F)
    where
        F: FnOnce() -> Guard,
    {
        unsafe { self.node.decrement_with_pin(self.parent, pin) }
    }
    /// Tries to create a new `RefEntry` by incrementing the reference count of
    /// a node.
    unsafe fn try_acquire(
        parent: &'a SkipList<K, V>,
        node: &Node<K, V>,
    ) -> Option<RefEntry<'a, K, V>> {
        if node.try_increment() {
            Some(RefEntry {
                parent,
                node: &*(node as *const _),
            })
        } else {
            None
        }
    }
}
impl<K, V> RefEntry<'_, K, V>
where
    K: Ord + Send + 'static,
    V: Send + 'static,
{
    /// Removes the entry from the skip list.
    ///
    /// Returns `true` if this call removed the entry and `false` if it was already removed.
    pub fn remove(&self, guard: &Guard) -> bool {
        self.parent.check_guard(guard);
        if self.node.mark_tower() {
            self.parent.hot_data.len.fetch_sub(1, Ordering::Relaxed);
            self.parent.search_bound(Bound::Included(&self.node.key), false, guard);
            true
        } else {
            false
        }
    }
}
impl<'a, K, V> Clone for RefEntry<'a, K, V> {
    fn clone(&self) -> RefEntry<'a, K, V> {
        unsafe {
            Node::try_increment(self.node);
        }
        RefEntry {
            parent: self.parent,
            node: self.node,
        }
    }
}
impl<K, V> fmt::Debug for RefEntry<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RefEntry").field(self.key()).field(self.value()).finish()
    }
}
impl<'a, K, V> RefEntry<'a, K, V>
where
    K: Ord,
{
    /// Moves to the next entry in the skip list.
    pub fn move_next(&mut self, guard: &Guard) -> bool {
        match self.next(guard) {
            None => false,
            Some(e) => {
                mem::replace(self, e).release(guard);
                true
            }
        }
    }
    /// Returns the next entry in the skip list.
    pub fn next(&self, guard: &Guard) -> Option<RefEntry<'a, K, V>> {
        self.parent.check_guard(guard);
        unsafe {
            let mut n = self.node;
            loop {
                n = self.parent.next_node(&n.tower, Bound::Excluded(&n.key), guard)?;
                if let Some(e) = RefEntry::try_acquire(self.parent, n) {
                    return Some(e);
                }
            }
        }
    }
    /// Moves to the previous entry in the skip list.
    pub fn move_prev(&mut self, guard: &Guard) -> bool {
        match self.prev(guard) {
            None => false,
            Some(e) => {
                mem::replace(self, e).release(guard);
                true
            }
        }
    }
    /// Returns the previous entry in the skip list.
    pub fn prev(&self, guard: &Guard) -> Option<RefEntry<'a, K, V>> {
        self.parent.check_guard(guard);
        unsafe {
            let mut n = self.node;
            loop {
                n = self.parent.search_bound(Bound::Excluded(&n.key), true, guard)?;
                if let Some(e) = RefEntry::try_acquire(self.parent, n) {
                    return Some(e);
                }
            }
        }
    }
}
/// An iterator over the entries of a `SkipList`.
pub struct Iter<'a: 'g, 'g, K, V> {
    parent: &'a SkipList<K, V>,
    head: Option<&'g Node<K, V>>,
    tail: Option<&'g Node<K, V>>,
    guard: &'g Guard,
}
impl<'a: 'g, 'g, K: 'a, V: 'a> Iterator for Iter<'a, 'g, K, V>
where
    K: Ord,
{
    type Item = Entry<'a, 'g, K, V>;
    fn next(&mut self) -> Option<Entry<'a, 'g, K, V>> {
        self
            .head = match self.head {
            Some(n) => {
                self.parent.next_node(&n.tower, Bound::Excluded(&n.key), self.guard)
            }
            None => {
                self.parent.next_node(&self.parent.head, Bound::Unbounded, self.guard)
            }
        };
        if let (Some(h), Some(t)) = (self.head, self.tail) {
            if h.key >= t.key {
                self.head = None;
                self.tail = None;
            }
        }
        self.head
            .map(|n| Entry {
                parent: self.parent,
                node: n,
                guard: self.guard,
            })
    }
}
impl<'a: 'g, 'g, K: 'a, V: 'a> DoubleEndedIterator for Iter<'a, 'g, K, V>
where
    K: Ord,
{
    fn next_back(&mut self) -> Option<Entry<'a, 'g, K, V>> {
        self
            .tail = match self.tail {
            Some(n) => {
                self.parent.search_bound(Bound::Excluded(&n.key), true, self.guard)
            }
            None => self.parent.search_bound(Bound::Unbounded, true, self.guard),
        };
        if let (Some(h), Some(t)) = (self.head, self.tail) {
            if h.key >= t.key {
                self.head = None;
                self.tail = None;
            }
        }
        self.tail
            .map(|n| Entry {
                parent: self.parent,
                node: n,
                guard: self.guard,
            })
    }
}
impl<K, V> fmt::Debug for Iter<'_, '_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iter")
            .field("head", &self.head.map(|n| (&n.key, &n.value)))
            .field("tail", &self.tail.map(|n| (&n.key, &n.value)))
            .finish()
    }
}
/// An iterator over reference-counted entries of a `SkipList`.
pub struct RefIter<'a, K, V> {
    parent: &'a SkipList<K, V>,
    head: Option<RefEntry<'a, K, V>>,
    tail: Option<RefEntry<'a, K, V>>,
}
impl<K, V> fmt::Debug for RefIter<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("RefIter");
        match &self.head {
            None => d.field("head", &None::<(&K, &V)>),
            Some(e) => d.field("head", &(e.key(), e.value())),
        };
        match &self.tail {
            None => d.field("tail", &None::<(&K, &V)>),
            Some(e) => d.field("tail", &(e.key(), e.value())),
        };
        d.finish()
    }
}
impl<'a, K: 'a, V: 'a> RefIter<'a, K, V>
where
    K: Ord,
{
    /// TODO
    pub fn next(&mut self, guard: &Guard) -> Option<RefEntry<'a, K, V>> {
        self.parent.check_guard(guard);
        self
            .head = match self.head {
            Some(ref e) => {
                let next_head = e.next(guard);
                unsafe {
                    e.node.decrement(guard);
                }
                next_head
            }
            None => try_pin_loop(|| self.parent.front(guard)),
        };
        let mut finished = false;
        if let (&Some(ref h), &Some(ref t)) = (&self.head, &self.tail) {
            if h.key() >= t.key() {
                finished = true;
            }
        }
        if finished {
            self.head = None;
            self.tail = None;
        }
        self.head.clone()
    }
    /// TODO
    pub fn next_back(&mut self, guard: &Guard) -> Option<RefEntry<'a, K, V>> {
        self.parent.check_guard(guard);
        self
            .tail = match self.tail {
            Some(ref e) => {
                let next_tail = e.prev(guard);
                unsafe {
                    e.node.decrement(guard);
                }
                next_tail
            }
            None => try_pin_loop(|| self.parent.back(guard)),
        };
        let mut finished = false;
        if let (&Some(ref h), &Some(ref t)) = (&self.head, &self.tail) {
            if h.key() >= t.key() {
                finished = true;
            }
        }
        if finished {
            self.head = None;
            self.tail = None;
        }
        self.tail.clone()
    }
}
/// An iterator over a subset of entries of a `SkipList`.
pub struct Range<'a: 'g, 'g, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    parent: &'a SkipList<K, V>,
    head: Option<&'g Node<K, V>>,
    tail: Option<&'g Node<K, V>>,
    range: R,
    guard: &'g Guard,
    _marker: PhantomData<fn() -> Q>,
}
impl<'a: 'g, 'g, Q, R, K: 'a, V: 'a> Iterator for Range<'a, 'g, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    type Item = Entry<'a, 'g, K, V>;
    fn next(&mut self) -> Option<Entry<'a, 'g, K, V>> {
        self
            .head = match self.head {
            Some(n) => {
                self.parent.next_node(&n.tower, Bound::Excluded(&n.key), self.guard)
            }
            None => self.parent.search_bound(self.range.start_bound(), false, self.guard),
        };
        if let Some(h) = self.head {
            let bound = match self.tail {
                Some(t) => Bound::Excluded(t.key.borrow()),
                None => self.range.end_bound(),
            };
            if !below_upper_bound(&bound, h.key.borrow()) {
                self.head = None;
                self.tail = None;
            }
        }
        self.head
            .map(|n| Entry {
                parent: self.parent,
                node: n,
                guard: self.guard,
            })
    }
}
impl<'a: 'g, 'g, Q, R, K: 'a, V: 'a> DoubleEndedIterator for Range<'a, 'g, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    fn next_back(&mut self) -> Option<Entry<'a, 'g, K, V>> {
        self
            .tail = match self.tail {
            Some(n) => {
                self
                    .parent
                    .search_bound(Bound::Excluded(&n.key.borrow()), true, self.guard)
            }
            None => self.parent.search_bound(self.range.end_bound(), true, self.guard),
        };
        if let Some(t) = self.tail {
            let bound = match self.head {
                Some(h) => Bound::Excluded(h.key.borrow()),
                None => self.range.start_bound(),
            };
            if !above_lower_bound(&bound, t.key.borrow()) {
                self.head = None;
                self.tail = None;
            }
        }
        self.tail
            .map(|n| Entry {
                parent: self.parent,
                node: n,
                guard: self.guard,
            })
    }
}
impl<Q, R, K, V> fmt::Debug for Range<'_, '_, Q, R, K, V>
where
    K: Ord + Borrow<Q> + fmt::Debug,
    V: fmt::Debug,
    R: RangeBounds<Q> + fmt::Debug,
    Q: Ord + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Range")
            .field("range", &self.range)
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}
/// An iterator over reference-counted subset of entries of a `SkipList`.
pub struct RefRange<'a, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    parent: &'a SkipList<K, V>,
    pub(crate) head: Option<RefEntry<'a, K, V>>,
    pub(crate) tail: Option<RefEntry<'a, K, V>>,
    pub(crate) range: R,
    _marker: PhantomData<fn() -> Q>,
}
unsafe impl<Q, R, K, V> Send for RefRange<'_, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{}
unsafe impl<Q, R, K, V> Sync for RefRange<'_, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{}
impl<Q, R, K, V> fmt::Debug for RefRange<'_, Q, R, K, V>
where
    K: Ord + Borrow<Q> + fmt::Debug,
    V: fmt::Debug,
    R: RangeBounds<Q> + fmt::Debug,
    Q: Ord + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefRange")
            .field("range", &self.range)
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}
impl<'a, Q, R, K: 'a, V: 'a> RefRange<'a, Q, R, K, V>
where
    K: Ord + Borrow<Q>,
    R: RangeBounds<Q>,
    Q: Ord + ?Sized,
{
    /// TODO
    pub fn next(&mut self, guard: &Guard) -> Option<RefEntry<'a, K, V>> {
        self.parent.check_guard(guard);
        self
            .head = match self.head {
            Some(ref e) => e.next(guard),
            None => {
                try_pin_loop(|| self.parent.lower_bound(self.range.start_bound(), guard))
            }
        };
        let mut finished = false;
        if let Some(ref h) = self.head {
            let bound = match self.tail {
                Some(ref t) => Bound::Excluded(t.key().borrow()),
                None => self.range.end_bound(),
            };
            if !below_upper_bound(&bound, h.key().borrow()) {
                finished = true;
            }
        }
        if finished {
            self.head = None;
            self.tail = None;
        }
        self.head.clone()
    }
    /// TODO: docs
    pub fn next_back(&mut self, guard: &Guard) -> Option<RefEntry<'a, K, V>> {
        self.parent.check_guard(guard);
        self
            .tail = match self.tail {
            Some(ref e) => e.prev(guard),
            None => {
                try_pin_loop(|| self.parent.upper_bound(self.range.start_bound(), guard))
            }
        };
        let mut finished = false;
        if let Some(ref t) = self.tail {
            let bound = match self.head {
                Some(ref h) => Bound::Excluded(h.key().borrow()),
                None => self.range.end_bound(),
            };
            if !above_lower_bound(&bound, t.key().borrow()) {
                finished = true;
            }
        }
        if finished {
            self.head = None;
            self.tail = None;
        }
        self.tail.clone()
    }
}
/// An owning iterator over the entries of a `SkipList`.
pub struct IntoIter<K, V> {
    /// The current node.
    ///
    /// All preceeding nods have already been destroyed.
    node: *mut Node<K, V>,
}
impl<K, V> Drop for IntoIter<K, V> {
    fn drop(&mut self) {
        while !self.node.is_null() {
            unsafe {
                let next = (*self.node)
                    .tower[0]
                    .load(Ordering::Relaxed, epoch::unprotected());
                Node::finalize(self.node);
                self.node = next.as_raw() as *mut Node<K, V>;
            }
        }
    }
}
impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<(K, V)> {
        loop {
            if self.node.is_null() {
                return None;
            }
            unsafe {
                let key = ptr::read(&(*self.node).key);
                let value = ptr::read(&(*self.node).value);
                let next = (*self.node)
                    .tower[0]
                    .load(Ordering::Relaxed, epoch::unprotected());
                Node::dealloc(self.node);
                self.node = next.as_raw() as *mut Node<K, V>;
                if next.tag() == 0 {
                    return Some((key, value));
                }
            }
        }
    }
}
impl<K, V> fmt::Debug for IntoIter<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("IntoIter { .. }")
    }
}
/// Helper function to retry an operation until pinning succeeds or `None` is
/// returned.
pub(crate) fn try_pin_loop<'a: 'g, 'g, F, K, V>(mut f: F) -> Option<RefEntry<'a, K, V>>
where
    F: FnMut() -> Option<Entry<'a, 'g, K, V>>,
{
    loop {
        if let Some(e) = f()?.pin() {
            return Some(e);
        }
    }
}
/// Helper function to check if a value is above a lower bound
fn above_lower_bound<T: Ord + ?Sized>(bound: &Bound<&T>, other: &T) -> bool {
    match *bound {
        Bound::Unbounded => true,
        Bound::Included(key) => other >= key,
        Bound::Excluded(key) => other > key,
    }
}
/// Helper function to check if a value is below an upper bound
fn below_upper_bound<T: Ord + ?Sized>(bound: &Bound<&T>, other: &T) -> bool {
    match *bound {
        Bound::Unbounded => true,
        Bound::Included(key) => other <= key,
        Bound::Excluded(key) => other < key,
    }
}
#[cfg(test)]
mod tests_rug_516 {
    use super::*;
    use crate::base::Bound;
    #[test]
    fn test_above_lower_bound() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(u64, u64) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Bound<&u64> = Bound::Included(&rug_fuzz_0);
        let p1: u64 = rug_fuzz_1;
        debug_assert_eq!(above_lower_bound(& p0, & p1), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_517 {
    use super::*;
    use std::collections::Bound;
    #[test]
    fn test_below_upper_bound() {

    extern crate bolero;
    extern crate arbitrary;
    bolero::check!()
        .for_each(|rug_data| {
            if let Ok((mut rug_fuzz_0, mut rug_fuzz_1)) = <(i32, i32) as arbitrary::Arbitrary>::arbitrary(&mut arbitrary::Unstructured::new(rug_data)){

        let p0: Bound<&i32> = Bound::Included(&rug_fuzz_0);
        let p1: i32 = rug_fuzz_1;
        debug_assert_eq!(below_upper_bound(& p0, & p1), true);
             }
});    }
}
#[cfg(test)]
mod tests_rug_519 {
    use super::*;
    use crate::base::Head;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_519_rrrruuuugggg_test_rug = 0;
        Head::<i32, i32>::new();
        let _rug_ed_tests_rug_519_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_520 {
    use super::*;
    use crate::base::{Head, Tower};
    use std::ops::Deref;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_520_rrrruuuugggg_test_rug = 0;
        let mut p0: Head<i32, i32> = Head::new();
        p0.deref();
        let _rug_ed_tests_rug_520_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_522 {
    use super::*;
    use crate::base;
    use std::alloc::{dealloc, Layout};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_522_rrrruuuugggg_test_rug = 0;
        let mut v126: *mut base::Node<u64, u64> = std::ptr::null_mut();
        unsafe {
            base::Node::<u64, u64>::dealloc(v126);
        }
        let _rug_ed_tests_rug_522_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_560 {
    use super::*;
    use crate::base;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_560_rrrruuuugggg_test_rug = 0;
        let mut p0: base::Entry<'_, '_, i32, i32> = unimplemented!();
        p0.key();
        let _rug_ed_tests_rug_560_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_562 {
    use super::*;
    use crate::base::{SkipList, Entry};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_562_rrrruuuugggg_test_rug = 0;
        let mut p0: Entry<'_, '_, i32, i32> = unimplemented!();
        p0.skiplist();
        let _rug_ed_tests_rug_562_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_568 {
    use super::*;
    use crate::base::Entry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_568_rrrruuuugggg_test_rug = 0;
        let mut p0: Entry<'_, '_, i32, i32> = unimplemented!();
        Entry::<'_, '_, i32, i32>::move_prev(&mut p0);
        let _rug_ed_tests_rug_568_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_570 {
    use super::*;
    use crate::base::RefEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_570_rrrruuuugggg_test_rug = 0;
        let mut p0: RefEntry<'_, i32, String> = unimplemented!();
        p0.is_removed();
        let _rug_ed_tests_rug_570_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_571 {
    use super::*;
    use crate::base::RefEntry;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_571_rrrruuuugggg_test_rug = 0;
        let mut p0: RefEntry<'_, i32, String> = unimplemented!();
        <RefEntry<'_, i32, String>>::key(&p0);
        let _rug_ed_tests_rug_571_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_575 {
    use super::*;
    use crate::base::{RefEntry, Guard};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_575_rrrruuuugggg_test_rug = 0;
        let mut p0: RefEntry<'_, i32, i32> = unimplemented!();
        let p1 = || unimplemented!() as Guard;
        RefEntry::release_with_pin(p0, p1);
        let _rug_ed_tests_rug_575_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_592 {
    use super::*;
    use crate::base::IntoIter;
    use std::ptr;
    use std::sync::atomic::Ordering;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_592_rrrruuuugggg_test_rug = 0;
        let mut p0: IntoIter<i32, String> = todo!();
        p0.next();
        let _rug_ed_tests_rug_592_rrrruuuugggg_test_rug = 0;
    }
}
