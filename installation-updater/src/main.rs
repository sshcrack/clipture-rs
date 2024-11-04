use std::{
    env::{args, args_os},
    fs,
    path::PathBuf,
    process::Command, thread,
};

use sysinfo::{Pid, System};
use walkdir::WalkDir;

fn main() {
    let data_dir = args().nth(1).unwrap();
    let copy_from = PathBuf::from(data_dir);

    let clipture_bin = args().nth(2).unwrap();
    let clipture_bin = PathBuf::from(clipture_bin);

    let process_id = args().nth(3).unwrap();
    let process_id = Pid::from_u32(process_id.parse().unwrap());

    let copy_to = clipture_bin.parent().unwrap();

    let mut sys = System::new();
    loop {
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[process_id]), true);
        if !sys.process(process_id).is_some() {
            break;
        }

        thread::sleep(std::time::Duration::from_millis(100));
        println!("Waiting for process to exit...");
    }

    // Copy recursivly every item of data dir to copy_to using walkdir
    let walker = WalkDir::new(&copy_from);
    for entry in walker {
        let entry = entry.unwrap();
        let src = entry.path();
        let dest = copy_to.join(src.strip_prefix(&copy_from).unwrap());
        if src.is_file() {
            fs::copy(src, dest).unwrap();
        } else {
            fs::create_dir_all(dest).unwrap();
        }
    }

    let _ = fs::remove_dir_all(copy_from);
    Command::new(clipture_bin)
        .args(args_os().skip(3))
        .spawn()
        .unwrap();
}
