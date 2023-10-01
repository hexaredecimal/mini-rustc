

struct Foo {
    bar: &'static str,
}

impl Foo {
    fn new(init: &'static str) -> Foo {
        Foo {bar: init}
    }
}

fn main() -> () {
    let foo: Foo = Foo::new("Hello"); 
}

