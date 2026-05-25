cd /home/mod/Dev/Rust/cadence && cargo build 2>&1 && cd /tmp && rm -rf cadence_test && mkdir cadence_test && cd cadence_test &&
 /home/mod/Dev/Rust/cadence/target/debug/cadence init && mkdir -p src && echo '// $$todo test' > src/main.rs &&
 /home/mod/Dev/Rust/cadence/target/debug/cadence add src/main.rs && /home/mod/Dev/Rust/cadence/target/debug/cadence add src/main.rs
 && cat .cadence/staged.json && /home/mod/Dev/Rust/cadence/target/debug/cadence reset && cat .cadence/staged.json
