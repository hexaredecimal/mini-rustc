
extern "C" {
    fn printf(fmt: &'static str,  ...) -> (); 
    fn scanf(fmt: &'static str, ...) -> i32; 
}

struct Person {
    name: &'static str, 
    age: i32, 
}

fn add(x: i32) -> i32 {
    x + 1
}

fn main() -> () {
    let me = Person {name: "Vincent", age:21}; 
    
    unsafe {
        printf("Hello, %s\n", me.name);
    }
    return (); 
}
