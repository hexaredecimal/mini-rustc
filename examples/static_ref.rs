
extern "C" {
    fn printf(fmt: &'static str,  ...) -> (); 
    fn scanf(fmt: &'static str, ...) -> i32; 
}

fn main() -> () {
    let number: &i32  = &(100 + 300); 
    let p: i32  = *number - 10; 
    unsafe {
        printf("p = %d\n", p); 
    }
    return (); 
}
