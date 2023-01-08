pub mod allocator;
pub mod fl;
mod globals;
pub mod pool;

#[cfg(test)]
mod tests {
    use crate::{
        colors::CAML_BLUE,
        freelist::pool::Pool,
        header::Header,
        hp_val, pool_val,
        utils::{self, field_val, whsize_wosize},
        val_hp,
        value::Value,
        word::Wsize,
    };

    use super::{allocator::NfAllocator, fl::FreeList};

    #[test]
    fn allocate_for_heap_expansion_test() {
        let request_wo_sz = 1024;
        let layout = utils::get_layout(Wsize::new(request_wo_sz));
        let memory = NfAllocator::allocate_for_heap_expansion(&layout);
        assert_eq!(
            memory.get_header().get_wosize(),
            Pool::get_field_wosz_from_pool_wosz(Wsize::new(request_wo_sz))
        );

        assert_eq!(memory.get_header().get_color(), CAML_BLUE);
        assert_eq!(
            pool_val!(memory).pool_wo_sz,
            Wsize::from_bytesize(layout.size())
        );

        unsafe { std::alloc::dealloc(pool_val!(memory) as *mut Pool as *mut u8, layout) };
    }

    #[test]
    fn test() {
        let mut allocator = NfAllocator::new();

        // nothing present in freelist
        assert!(FreeList::new(allocator.get_globals()).nf_iter().count() == 0);

        let intended_expansion_size = Wsize::new(1024 * 1024); // Expand the heap with a chunk of size
                                                               // 1024*1024 words i.e (1024**2) *
                                                               // WORD_SIZE bytes

        let (layout, _actual_expansion_size) =
            utils::get_layout_and_actual_expansion_size(intended_expansion_size);

        let actual_expansion_size = Wsize::from_bytesize(layout.size());

        // no pool block is there, there's only the one which is fixed and is not used in iter
        assert_eq!(allocator.get_pool_iter().count(), 0);

        // nf_expand_heap heap will actually allocate for size=actual_expansion_size instead of
        // intended_expansion_size
        allocator.nf_expand_heap(intended_expansion_size);

        let pool_leader_wsz =
            whsize_wosize(Pool::get_field_wosz_from_pool_wosz(actual_expansion_size));
        assert_eq!(allocator.get_globals().cur_wsz, pool_leader_wsz,);

        // 1 chunk is present in freelist after expansion
        assert_eq!(FreeList::new(allocator.get_globals()).nf_iter().count(), 1);

        // 1 pool block has been added as well
        assert_eq!(allocator.get_pool_iter().count(), 1);
        // this asserts invariant [ pool list being sorted]
        allocator.check_pool_list_invariant();

        let mut allocations = vec![
            Some(allocator.nf_allocate(Wsize::new(1024))), // allocates 1024 + 1 word
            Some(allocator.nf_allocate(Wsize::new(1024))), // allocates 1024 + 1 word
        ];

        // initial size -(1024 + 1 word( ret by whsize_wosize) allocated twice)
        let cur_wsz = pool_leader_wsz - ((whsize_wosize(Wsize::new(1024))) * 2);

        assert_eq!(allocator.get_globals().cur_wsz, cur_wsz);

        let to_be_freed = allocations.get_mut(0).unwrap().take().unwrap();
        assert!(allocations.get(0).unwrap().is_none());

        let allocatable_memory_left = FreeList::new(allocator.get_globals())
            .nf_iter()
            .fold(Wsize::new(0), |acc, x| {
                acc + x.get_cur().get_header().get_wosize()
            });

        //The following allocation will force the empty block case in nf_allocate_block
        let hp = allocator.nf_allocate(allocatable_memory_left - Wsize::new(1));

        assert_eq!(
            val_hp!(hp).get_header().get_wosize(),
            allocatable_memory_left - Wsize::new(1)
        );
        //Assert the size of empty block that lies 1 word before hp
        assert_eq!(
            Value(hp as usize).get_header().get_wosize(), // treat hp as val, it'll treat empty
            // block as it's header
            Wsize::new(0)
        );
        allocations.push(Some(hp));

        // This must've made the free list empty
        assert_eq!(allocator.get_globals().cur_wsz, Wsize::new(0));
        assert_eq!(
            FreeList::new(allocator.get_globals())
                .nf_iter()
                .fold(Wsize::new(0), |acc, x| {
                    acc + x.get_cur().get_header().get_wosize()
                }),
            Wsize::new(0)
        );
        assert_eq!(FreeList::new(allocator.get_globals()).nf_iter().count(), 0);

        // Freeing the first allocation
        let to_be_freed_header = val_hp!(to_be_freed).get_header().clone();
        allocator.nf_deallocate(val_hp!(to_be_freed));

        assert_eq!(
            allocator.get_globals().cur_wsz,
            to_be_freed_header.get_wosize() + Wsize::new(1)
        );

        let allocatable_memory_left = to_be_freed_header.get_wosize();

        FreeList::new(allocator.get_globals()).nf_iter().count();

        // Allocating exactly allocatable_memory_left will again empty the freelist
        let hp = allocator.nf_allocate(allocatable_memory_left);

        assert_ne!(hp, std::ptr::null_mut());
        assert_eq!(allocator.get_globals().cur_wsz, Wsize::new(0));
        assert_eq!(
            FreeList::new(allocator.get_globals())
                .nf_iter()
                .fold(Wsize::new(0), |acc, x| {
                    acc + x.get_cur().get_header().get_wosize()
                }),
            Wsize::new(0)
        );

        // Calling nf_expand_heap one more time.
        // Currently there's one pool block and free list is empty

        allocator.nf_expand_heap(intended_expansion_size);

        assert_eq!(FreeList::new(allocator.get_globals()).nf_iter().count(), 1);
        assert_eq!(allocator.get_globals().cur_wsz, pool_leader_wsz);

        assert_eq!(allocator.get_pool_iter().count(), 2);
        allocator.check_pool_list_invariant();

        let mut pool_block_count = 2;
        let mut freelist_node_count = 1;
        // checking the pool invariant 10 more times
        for i in 1..=10 {
            pool_block_count += 1;
            freelist_node_count += 1;
            allocator.nf_expand_heap(intended_expansion_size);
            assert_eq!(
                FreeList::new(allocator.get_globals()).nf_iter().count(),
                freelist_node_count
            );
            assert_eq!(allocator.get_globals().cur_wsz, pool_leader_wsz * (i + 1));

            assert_eq!(allocator.get_pool_iter().count(), pool_block_count);
            allocator.check_pool_list_invariant();
        }
    }
}
