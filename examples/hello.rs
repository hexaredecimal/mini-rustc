extern "C" {
    // fn puts(c: *const i32) -> i32;
    fn printf(fmt: &'static str, ...) -> (); 
    fn puts(c: &'static str) -> i32;
    fn malloc(size: i32) -> *const i32;
    fn free(ptr: *const i32) -> (); 
}

struct Person {
    name: &'static str, 
    age: i32, 
}

fn println(msg: &'static str) -> () {
    unsafe {
        printf("%s\n", msg); 
    }
}

fn main() -> () {
    let me = Person {name: "Vincent", age: 21}; 
    let mut memory: *const i32 = unsafe {
        malloc(10) as *const i32
    }; 

    unsafe {
      *memory = 100;   
    }; 

    unsafe {
        printf("Hello %s, your are now %d years old!", me.name, me.age); 
    }
}
