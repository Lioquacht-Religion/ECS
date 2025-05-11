// all_tuples.rs

// TODO:

#[macro_export]
macro_rules! all_tuples {
    ( $m:ident, $head:ident, $($tail:ident), *) => {
        $m!($head, $($tail), *);
        all_tuples!($m, $($tail), *);
    };
    ( $m:ident, $($t:ident), *) => {
        $m!($($t), *);
    };
}
