#![warn(clippy::unused_peekable)]
#![allow(clippy::no_effect)]

use std::iter::Empty;
use std::iter::Peekable;

fn main() {
    invalid();
    valid();
}

#[allow(clippy::unused_unit)]
fn invalid() {
    let peekable = std::iter::empty::<u32>().peekable();

    // Only lint `new_local`
    let old_local = std::iter::empty::<u32>().peekable();
    let new_local = old_local;

    // Behind mut ref
    let mut by_mut_ref_test = std::iter::empty::<u32>().peekable();
    let by_mut_ref = &mut by_mut_ref_test;

    // Explicitly returns `Peekable`
    fn returns_peekable() -> Peekable<Empty<u32>> {
        std::iter::empty().peekable()
    }

    let peekable_from_fn = returns_peekable();

    // Using a method not exclusive to `Peekable`
    let mut peekable_using_iterator_method = std::iter::empty::<u32>().peekable();
    peekable_using_iterator_method.next();

    let mut peekable_in_for_loop = std::iter::empty::<u32>().peekable();
    for x in peekable_in_for_loop {}
}

fn valid() {
    fn takes_peekable(_peek: Peekable<Empty<u32>>) {}

    // Passed to another function
    let passed_along = std::iter::empty::<u32>().peekable();
    takes_peekable(passed_along);

    // `peek` called in another block
    let mut peekable_in_block = std::iter::empty::<u32>().peekable();
    {
        peekable_in_block.peek();
    }

    // Check the other `Peekable` methods :)
    {
        let mut peekable_with_peek_mut = std::iter::empty::<u32>().peekable();
        peekable_with_peek_mut.peek_mut();

        let mut peekable_with_next_if = std::iter::empty::<u32>().peekable();
        peekable_with_next_if.next_if(|_| true);

        let mut peekable_with_next_if_eq = std::iter::empty::<u32>().peekable();
        peekable_with_next_if_eq.next_if_eq(&3);
    }

    let mut peekable_in_closure = std::iter::empty::<u32>().peekable();
    let call_peek = |p: &mut Peekable<Empty<u32>>| {
        p.peek();
    };
    call_peek(&mut peekable_in_closure);

    // From a macro
    macro_rules! make_me_a_peekable_please {
        () => {
            std::iter::empty::<u32>().peekable()
        };
    }

    let _unsuspecting_macro_user = make_me_a_peekable_please!();

    // Generic Iterator returned
    fn return_an_iter() -> impl Iterator<Item = u32> {
        std::iter::empty::<u32>().peekable()
    }

    let _unsuspecting_user = return_an_iter();

    // Call `peek` in a macro
    macro_rules! peek_iter {
        ($iter:ident) => {
            $iter.peek();
        };
    }

    let mut peek_in_macro = std::iter::empty::<u32>().peekable();
    peek_iter!(peek_in_macro);

    // Behind mut ref
    let mut by_mut_ref_test = std::iter::empty::<u32>().peekable();
    let by_mut_ref = &mut by_mut_ref_test;
    by_mut_ref.peek();

    // Behind ref
    let mut by_ref_test = std::iter::empty::<u32>().peekable();
    let by_ref = &by_ref_test;
    by_ref_test.peek();

    // In struct
    struct PeekableWrapper {
        f: Peekable<Empty<u32>>,
    }

    let struct_test = std::iter::empty::<u32>().peekable();
    PeekableWrapper { f: struct_test };

    // `peek` called in another block as the last expression
    let mut peekable_last_expr = std::iter::empty::<u32>().peekable();
    {
        peekable_last_expr.peek();
    }
}
