
use platform::{*};

fn main() {
    println!("file: {:?}", file!());
    println!("dir: {:?}", dir!());
    println!("project_path: {:?}", project_path!());
    println!("rel_path: {:?}", rel_path!("./../src/lib.rs"));
    println!("canonical_path: {:?}", canonical_path!("./../src/lib.rs"));
}