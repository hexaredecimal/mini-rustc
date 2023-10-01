
extern "C" {
    fn malloc(size: i32) -> *const i32; 
    fn free(ptr: *const i32) -> (); 
}

fn main() -> () {
    let _unit: () = return ();
}
