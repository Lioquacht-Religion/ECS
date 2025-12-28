// all_tuples.rs

// TODO: redo this as an easier to use proc macro
#[macro_export]
macro_rules! all_tuples {
    (
        $m:ident,
        ($head1:ident, $($tail1:ident), *),
        ($head2:ident, $($tail2:ident), *)
    ) => {
        $m!(($head1, $($tail1), *), ($head2, $($tail2), *));
        all_tuples!($m, ($($tail1), *), ($($tail2), *));
    };
    ( $m:ident, ($($t1:ident), *), ($($t2:ident), *)) => {
        $m!(($($t1), *), ($($t2), *));
    };
    ( $m:ident, $head:ident, $($tail:ident), *) => {
        $m!($head, $($tail), *);
        all_tuples!($m, $($tail), *);
    };
    ( $m:ident, $($t:ident), *) => {
        $m!($($t), *);
    };
}

#[macro_export]
macro_rules! all_tuples_wout_single {
    (
        $m:ident,
        ($head1:ident, $($tail1:ident), *),
        ($head2:ident, $($tail2:ident), *)
    ) => {
        $m!(($head1, $($tail1), *), ($head2, $($tail2), *));
        all_tuples_wout_single!($m, ($($tail1), *), ($($tail2), *));
    };
    ( $m:ident, ($($t1:ident), *), ($($t2:ident), *)) => {
    };
    ( $m:ident, $head:ident, $($tail:ident), *) => {
        $m!($head, $($tail), *);
        all_tuples_wout_single!($m, $($tail), *);
    };
    ( $m:ident, $($t:ident), *) => {
    };
}
