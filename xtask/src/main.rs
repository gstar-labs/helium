//! Repository gate command-line entry point.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let gates = xtask::registry();
    let result = match args.as_slice() {
        [command, list] if command == "gate" && list == "--list" => {
            for gate in &gates {
                println!("{}", gate.name);
            }
            Ok(())
        }
        [command] if command == "gate" => xtask::run_gates(&gates, None),
        [command, name] if command == "branch-name" => {
            if ["feat", "fix", "rfc", "docs", "chore", "dependabot"]
                .iter()
                .any(|prefix| {
                    name.strip_prefix(prefix)
                        .is_some_and(|rest| rest.starts_with('/') && rest.len() > 1)
                })
            {
                Ok(())
            } else {
                Err(format!("invalid branch name: {name}"))
            }
        }
        [command, flag, name] if command == "gate" && flag == "--only" => {
            xtask::run_gates(&gates, Some(name))
        }
        _ => Err("usage: cargo xtask gate [--list | --only NAME]".to_owned()),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
