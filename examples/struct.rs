
extern "C" {
    fn printf(fmt: &'static str, ...) -> (); 
}


struct S {
    a: i32,
}

fn f(s: S) -> &'a S {
    if s.a == 1 {
        &S { a: 10 }
    } else {
        &S { a: 20 }
    }
}

fn main() -> () {
    let s: &S = f(S { a: 1 }); 
    
    unsafe {
        printf("s =  S { a: i32 (%d) }\n", *s.a); 
    }
}
