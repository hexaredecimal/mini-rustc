mod a {
    struct Inner {
        x: i32,
    }
}

struct Outer {
    inner: a::Inner,
}

fn main() -> i32 {
    let resut: Outer = crate::Outer::from(a::Inner {x: 0}); 

    let l: Outer = crate::Outer {
        inner: a::Inner { x: 0 },
    };
    l.inner.x
}
