use core::alloc::LayoutError;
use core::cmp;
use core::ops::Drop;
use core::ptr::{self, NonNull};
use core::slice;
use std::alloc::{handle_alloc_error, Layout};

pub struct TryReserveError {
    pub kind: TryReserveErrorKind,
}

impl TryReserveError {
    pub fn kind(&self) -> TryReserveErrorKind {
        self.kind.clone()
    }
}

impl From<TryReserveErrorKind> for TryReserveError {
    fn from(kind: TryReserveErrorKind) -> Self {
        TryReserveError { kind }
    }
}

#[derive(Clone)]
pub enum TryReserveErrorKind {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,

    /// The memory allocator returned an error
    AllocError {
        /// The layout of allocation request that failed
        layout: Layout,
        non_exhaustive: (),
    },
}

#[cfg(not(no_global_oom_handling))]
#[allow(dead_code)]
enum AllocInit {
    /// The contents of the new memory are uninitialized.
    Uninitialized,
    /// The new memory is guaranteed to be zeroed.
    Zeroed,
}

/// A low-level utility for more ergonomically allocating, reallocating, and deallocating
/// a buffer of memory on the heap without having to worry about all the corner cases
/// involved. This type is excellent for building your own data structures like Vec and VecDeque.
/// In particular:
///
/// * Produces `Unique::dangling()` on zero-sized types.
/// * Produces `Unique::dangling()` on zero-length allocations.
/// * Avoids freeing `Unique::dangling()`.
/// * Catches all overflows in capacity computations (promotes them to "capacity overflow" panics).
/// * Guards against 32-bit systems allocating more than isize::MAX bytes.
/// * Guards against overflowing your length.
/// * Calls `handle_alloc_error` for fallible allocations.
/// * Contains a `ptr::Unique` and thus endows the user with all related benefits.
/// * Uses the excess returned from the allocator to use the largest available capacity.
///
/// This type does not in anyway inspect the memory that it manages. When dropped it *will*
/// free its memory, but it *won't* try to drop its contents. It is up to the user of `RawVec`
/// to handle the actual things *stored* inside of a `RawVec`.
///
/// Note that the excess of a zero-sized types is always infinite, so `capacity()` always returns
/// `usize::MAX`. This means that you need to be careful when round-tripping this type with a
/// `Box<[T]>`, since `capacity()` won't yield the length.
#[allow(missing_debug_implementations)]
pub struct RawVec {
    ptr: *mut u8,
    cap: usize,
}

#[allow(dead_code)]
impl RawVec {
    /// HACK(Centril): This exists because stable `const fn` can only call stable `const fn`, so
    /// they cannot call `Self::new()`.
    ///
    /// If you change `RawVec<T>::new` or dependencies, please take care to not introduce anything
    /// that would truly const-call something unstable.
    #[allow(dead_code)]
    pub const NEW: Self = Self::new();

    /// Creates the biggest possible `RawVec` (on the system heap)
    /// without allocating. If `T` has positive size, then this makes a
    /// `RawVec` with capacity `0`. If `T` is zero-sized, then it makes a
    /// `RawVec` with capacity `usize::MAX`. Useful for implementing
    /// delayed allocation.
    #[must_use]
    pub const fn new() -> Self {
        Self::new_in()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity)
    }

    pub fn with_capacity_zeroed(capacity: usize) -> Self {
        Self::with_capacity_zeroed_in(capacity)
    }
}

#[allow(dead_code)]
impl RawVec {
    // Tiny Vecs are dumb. Skip to:
    // - 8 if the element size is 1, because any heap allocators is likely
    //   to round up a request of less than 8 bytes to at least 8 bytes.
    // - 4 if elements are moderate-sized (<= 1 KiB).
    // - 1 otherwise, to avoid wasting too much space for very short Vecs.
    pub(crate) const MIN_NON_ZERO_CAP: usize = 8;

    /// Like `new`, but parameterized over the choice of allocator for
    /// the returned `RawVec`.
    pub const fn new_in() -> Self {
        // `cap: 0` means "unallocated". zero-sized types are ignored.
        Self { ptr: 0 as *mut u8, cap: 0 }
    }

    /// Like `with_capacity`, but parameterized over the choice of
    /// allocator for the returned `RawVec`.
    #[cfg(not(no_global_oom_handling))]
    #[inline]
    pub fn with_capacity_in(capacity: usize) -> Self {
        Self::allocate_in(capacity, AllocInit::Uninitialized)
    }

    /// Like `with_capacity_zeroed`, but parameterized over the choice
    /// of allocator for the returned `RawVec`.
    #[cfg(not(no_global_oom_handling))]
    #[inline]
    pub fn with_capacity_zeroed_in(capacity: usize) -> Self {
        Self::allocate_in(capacity, AllocInit::Zeroed)
    }


    #[cfg(not(no_global_oom_handling))]
    fn allocate_in(capacity: usize, init: AllocInit) -> Self {
        // Don't allocate here because `Drop` will not deallocate when `capacity` is 0.

        // We avoid `unwrap_or_else` here because it bloats the amount of
        // LLVM IR generated.
        let layout = match Layout::array::<u8>(capacity) {
            Ok(layout) => layout,
            Err(_) => capacity_overflow(),
        };
        match alloc_guard(layout.size()) {
            Ok(_) => {}
            Err(_) => capacity_overflow(),
        }

        let result = match init {
            AllocInit::Uninitialized => unsafe { std::alloc::alloc(layout) },
            AllocInit::Zeroed => unsafe { std::alloc::alloc_zeroed(layout) },
        };
        let ptr = result;

        // Allocators currently return a `NonNull<[u8]>` whose length
        // matches the size requested. If that ever changes, the capacity
        // here should change to `ptr.len() / mem::size_of::<T>()`.
        Self {
            ptr: ptr as *mut u8,
            cap: capacity,
        }
    }

    /// Gets a raw pointer to the start of the allocation. Note that this is
    /// `Unique::dangling()` if `capacity == 0` or `T` is zero-sized. In the former case, you must
    /// be careful.
    #[inline]
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Gets the capacity of the allocation.
    ///
    /// This will always be `usize::MAX` if `T` is zero-sized.
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.cap
    }


    fn current_memory(&self) -> Option<(NonNull<u8>, Layout)> {
        unsafe {
            let layout = Layout::array::<u8>(self.cap).unwrap_unchecked();
            Some((NonNull::new(self.ptr).unwrap_unchecked(), layout))
        }
        // We have an allocated chunk of memory, so we can bypass runtime
        // checks to get our current layout.
        // unsafe {
        //     let layout = Layout::array::<u8>(self.cap).unwrap_unchecked();
        //     Some((NonNull::new(self.ptr).unwrap_unchecked(), layout))
        // }
    }

    /// Ensures that the buffer contains at least enough space to hold `len +
    /// additional` elements. If it doesn't already have enough capacity, will
    /// reallocate enough space plus comfortable slack space to get amortized
    /// *O*(1) behavior. Will limit this behavior if it would needlessly cause
    /// itself to panic.
    ///
    /// If `len` exceeds `self.capacity()`, this may fail to actually allocate
    /// the requested space. This is not really unsafe, but the unsafe
    /// code *you* write that relies on the behavior of this function may break.
    ///
    /// This is ideal for implementing a bulk-push operation like `extend`.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Aborts
    ///
    /// Aborts on OOM.
    #[cfg(not(no_global_oom_handling))]
    #[inline]
    pub fn reserve(&mut self, len: usize, additional: usize) {
        // Callers expect this function to be very cheap when there is already sufficient capacity.
        // Therefore, we move all the resizing and error-handling logic from grow_amortized and
        // handle_reserve behind a call, while making sure that this function is likely to be
        // inlined as just a comparison and a call if the comparison fails.
        #[cold]
        fn do_reserve_and_handle(
            slf: &mut RawVec,
            len: usize,
            additional: usize,
        ) {
            handle_reserve(slf.grow_amortized(len, additional));
        }

        if self.needs_to_grow(len, additional) {
            do_reserve_and_handle(self, len, additional);
        }
    }

    /// A specialized version of `reserve()` used only by the hot and
    /// oft-instantiated `Vec::push()`, which does its own capacity check.
    #[cfg(not(no_global_oom_handling))]
    #[inline(never)]
    pub fn reserve_for_push(&mut self, len: usize) {
        handle_reserve(self.grow_amortized(len, 1));
    }

    /// The same as `reserve`, but returns on errors instead of panicking or aborting.
    pub fn try_reserve(&mut self, len: usize, additional: usize) -> Result<(), TryReserveError> {
        if self.needs_to_grow(len, additional) {
            self.grow_amortized(len, additional)
        } else {
            Ok(())
        }
    }

    /// Ensures that the buffer contains at least enough space to hold `len +
    /// additional` elements. If it doesn't already, will reallocate the
    /// minimum possible amount of memory necessary. Generally this will be
    /// exactly the amount of memory necessary, but in principle the allocator
    /// is free to give back more than we asked for.
    ///
    /// If `len` exceeds `self.capacity()`, this may fail to actually allocate
    /// the requested space. This is not really unsafe, but the unsafe code
    /// *you* write that relies on the behavior of this function may break.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Aborts
    ///
    /// Aborts on OOM.
    #[cfg(not(no_global_oom_handling))]
    pub fn reserve_exact(&mut self, len: usize, additional: usize) {
        handle_reserve(self.try_reserve_exact(len, additional));
    }

    /// The same as `reserve_exact`, but returns on errors instead of panicking or aborting.
    pub fn try_reserve_exact(
        &mut self,
        len: usize,
        additional: usize,
    ) -> Result<(), TryReserveError> {
        if self.needs_to_grow(len, additional) { self.grow_exact(len, additional) } else { Ok(()) }
    }

    // Shrinks the buffer down to the specified capacity. If the given amount
    // is 0, actually completely deallocates.
    //
    // # Panics
    //
    // Panics if the given amount is *larger* than the current capacity.
    //
    // # Aborts
    //
    // Aborts on OOM.
    // #[cfg(not(no_global_oom_handling))]
    // pub fn shrink_to_fit(&mut self, cap: usize) {
    //     handle_reserve(self.shrink(cap));
    // }
}

impl RawVec {
    /// Returns if the buffer needs to grow to fulfill the needed extra capacity.
    /// Mainly used to make inlining reserve-calls possible without inlining `grow`.
    fn needs_to_grow(&self, len: usize, additional: usize) -> bool {
        additional > self.capacity().wrapping_sub(len)
    }

    fn set_ptr_and_cap(&mut self, ptr: NonNull<[u8]>, cap: usize) {
        // Allocators currently return a `NonNull<[u8]>` whose length matches
        // the size requested. If that ever changes, the capacity here should
        // change to `ptr.len() / mem::size_of::<T>()`.
        self.ptr = ptr.as_ptr() as *mut u8;
        self.cap = cap;
    }

    // This method is usually instantiated many times. So we want it to be as
    // small as possible, to improve compile times. But we also want as much of
    // its contents to be statically computable as possible, to make the
    // generated code run faster. Therefore, this method is carefully written
    // so that all of the code that depends on `T` is within it, while as much
    // of the code that doesn't depend on `T` as possible is in functions that
    // are non-generic over `T`.
    fn grow_amortized(&mut self, len: usize, additional: usize) -> Result<(), TryReserveError> {
        // This is ensured by the calling contexts.
        debug_assert!(additional > 0);

        // Nothing we can really do about these checks, sadly.
        let required_cap = match len.checked_add(additional) {
            None => return Err(TryReserveError { kind: TryReserveErrorKind::CapacityOverflow }),
            Some(c) => c,
        };

        // This guarantees exponential growth. The doubling cannot overflow
        // because `cap <= isize::MAX` and the type of `cap` is `usize`.
        let cap = cmp::max(self.cap * 2, required_cap);
        let cap = cmp::max(Self::MIN_NON_ZERO_CAP, cap);

        let new_layout = Layout::array::<u8>(cap);

        // `finish_grow` is non-generic over `T`.
        let ptr = finish_grow(new_layout, self.current_memory())?;
        self.set_ptr_and_cap(ptr, cap);
        Ok(())
    }

    // The constraints on this method are much the same as those on
    // `grow_amortized`, but this method is usually instantiated less often so
    // it's less critical.
    fn grow_exact(&mut self, len: usize, additional: usize) -> Result<(), TryReserveError> {
        let cap = match len.checked_add(additional) {
            None => return Err(TryReserveError { kind: TryReserveErrorKind::CapacityOverflow }),
            Some(cap) => cap,
        };

        let new_layout = Layout::array::<u8>(cap);

        // `finish_grow` is non-generic over `T`.
        let ptr = finish_grow(new_layout, self.current_memory())?;
        self.set_ptr_and_cap(ptr, cap);
        Ok(())
    }

    // #[cfg(not(no_global_oom_handling))]
    // fn shrink(&mut self, cap: usize) -> Result<(), TryReserveError> {
    //     assert!(cap <= self.capacity(), "Tried to shrink to a larger capacity");
    //
    //     let (ptr, layout) = if let Some(mem) = self.current_memory() { mem } else { return Ok(()); };
    //
    //     let ptr = unsafe {
    //         // `Layout::array` cannot overflow here because it would have
    //         // overflowed earlier when capacity was larger.
    //         let new_layout = Layout::array::<u8>(cap).unwrap_unchecked();
    //         std::alloc::shr
    //         self.alloc
    //             .shrink(ptr, layout, new_layout)
    //             .map_err(|_| TryReserveErrorKind::AllocError { layout: new_layout, non_exhaustive: () })?
    //     };
    //     self.set_ptr_and_cap(ptr, cap);
    //     Ok(())
    // }
}

// This function is outside `RawVec` to minimize compile times. See the comment
// above `RawVec::grow_amortized` for details. (The `A` parameter isn't
// significant, because the number of different `A` types seen in practice is
// much smaller than the number of `T` types.)
#[inline(never)]
fn finish_grow(
    new_layout: Result<Layout, LayoutError>,
    current_memory: Option<(NonNull<u8>, Layout)>,
) -> Result<NonNull<[u8]>, TryReserveError>
{
    // Check for the error here to minimize the size of `RawVec::grow_*`.
    let new_layout = new_layout.map_err(|_| TryReserveErrorKind::CapacityOverflow)?;

    alloc_guard(new_layout.size())?;

    if let Some((ptr, old_layout)) = current_memory {
        debug_assert_eq!(old_layout.align(), new_layout.align());
        // The allocator checks for alignment equality
        // intrinsics::assume(old_layout.align() == new_layout.align());
        grow_global_allocator(ptr, old_layout, new_layout)
    } else {
        unsafe {
            let raw_ptr = std::alloc::alloc(new_layout);
            let ptr = NonNull::new(raw_ptr).ok_or(TryReserveErrorKind::AllocError { layout: new_layout, non_exhaustive: () })?;
            let slice = slice::from_raw_parts_mut(ptr.as_ptr(), new_layout.size());
            Ok(NonNull::new(slice).unwrap_unchecked())
        }
    }
}

fn grow_global_allocator(
    ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, TryReserveError> {
    debug_assert!(
        new_layout.size() >= old_layout.size(),
        "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
    );

    let new_ptr = unsafe { std::alloc::alloc(new_layout) };

    // SAFETY: because `new_layout.size()` must be greater than or equal to
    // `old_layout.size()`, both the old and new memory allocation are valid for reads and
    // writes for `old_layout.size()` bytes. Also, because the old allocation wasn't yet
    // deallocated, it cannot overlap `new_ptr`. Thus, the call to `copy_nonoverlapping` is
    // safe. The safety contract for `dealloc` must be upheld by the caller.
    unsafe {
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr, old_layout.size());
        deallocate_global_allocator(ptr, old_layout);
    }
    unsafe {
        let ptr = NonNull::new(new_ptr).ok_or(TryReserveErrorKind::AllocError { layout: new_layout, non_exhaustive: () })?;
        let slice = slice::from_raw_parts_mut(ptr.as_ptr(), new_layout.size());
        Ok(NonNull::new(slice).unwrap_unchecked())
    }
}

fn deallocate_global_allocator(ptr: NonNull<u8>, layout: Layout) {
    // SAFETY: the safety contract must be upheld by the caller
    unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) };
}

impl Drop for RawVec {
    /// Frees the memory owned by the `RawVec` *without* trying to drop its contents.
    fn drop(&mut self) {
        if let Some((ptr, layout)) = self.current_memory() {
            deallocate_global_allocator(ptr, layout)
        }
    }
}

#[inline]
fn handle_reserve(result: Result<(), TryReserveError>) {
    match result.map_err(|e| e.kind()) {
        Err(TryReserveErrorKind::CapacityOverflow) => capacity_overflow(),
        Err(TryReserveErrorKind::AllocError { layout, .. }) => handle_alloc_error(layout),
        Ok(()) => { /* yay */ }
    }
}

// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects.
// * We don't overflow `usize::MAX` and actually allocate too little.
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit and 16-bit we need to add
// an extra guard for this in case we're running on a platform which can use
// all 4GB in user-space, e.g., PAE or x32.

#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), TryReserveError> {
    if usize::BITS < 64 && alloc_size > isize::MAX as usize {
        Err(TryReserveErrorKind::CapacityOverflow.into())
    } else {
        Ok(())
    }
}

// One central function responsible for reporting capacity overflows. This'll
// ensure that the code generation related to these panics is minimal as there's
// only one location which panics rather than a bunch throughout the module.
#[cfg(not(no_global_oom_handling))]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}
