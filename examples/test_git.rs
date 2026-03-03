use circleci_tui_rs::git;

fn main() {
    match git::get_current_branch() {
        Some(branch) => println!("✓ Detected branch: {}", branch),
        None => println!("✗ No branch detected (detached HEAD or not in git repo)"),
    }
}
