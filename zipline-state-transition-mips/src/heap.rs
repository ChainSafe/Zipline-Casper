use core::alloc::{GlobalAlloc, Layout};
use core::cell::RefCell;
use core::mem::MaybeUninit;
use core::ptr::{self, NonNull};

const HEAP_SIZE: usize = 0x30000000 - 0x10040000; // 33.5MB This is the max heap size before it hits the Cannon special memory slots
                                                // const HEAP_SIZE: usize = 0x400000; // 4.2MB heap size (peppy default)
struct Alloc {
    heap: RefCell<linked_list_allocator::Heap>,
}

impl Alloc {
    const fn new() -> Self {
        Self {
            heap: RefCell::new(linked_list_allocator::Heap::empty()),
        }
    }
}

unsafe impl GlobalAlloc for Alloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.heap
            .borrow_mut()
            .allocate_first_fit(layout)
            .ok()
            .map_or(ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.heap
            .borrow_mut()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}

#[global_allocator]
static mut ALLOCATOR: Alloc = Alloc::new();

pub unsafe fn init() {
    static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    ALLOCATOR
        .heap
        .borrow_mut()
        .init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE)
}
