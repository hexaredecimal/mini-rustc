
extern "C" {
    fn printf(fmt: &'static str, ...) -> (); 
}

fn get(input: &i32) -> &'a i32 {
    &(*input + 100)
}

fn main() -> () {
    let n: &i32 = get(&100); 
    unsafe {
        if *n == 200 {
            printf("SUCCEEDED: %d\n", n); 
        } else {
            printf("NOT GOOD: %d\n", n); 
        }
    }; 
    
    return (); 
}

