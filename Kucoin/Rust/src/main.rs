use std::env;
use std::fs::File;
use std::path::Path;
use std::vec::Vec;

fn cwd() -> String {
    let path = env::current_dir();
    path.expect("Cannot get CWD").display().to_string()
}

/////////////////////////////////////////////////////////  Find_Triangular_Arbitrage  /////////////////////////////////////////////////////////

fn find_triangular_arbitrage() {
    let json_file_path = cwd() + "/Triangular_pairs.catalog";
    //println!("{}", cwd() + "/Triangular_pairs.catalog");
    let json_file = Path::new(&json_file_path);
    let file = File::open(json_file).expect("file not found");
    let triangular_pairs: Vec<Vec<String>> =
        serde_json::from_reader(file).expect("error while reading");
    //println!("{:?}", triangular_pairs)
}

// Runs all modules
fn main() {
    // Part 1 -- create_valid_pairs
    // Part 2 -- websocket_spawner
    // Part 3 -- find_triangular_arbitrage
    find_triangular_arbitrage()
    // Part 4 -- execute_trades
}
