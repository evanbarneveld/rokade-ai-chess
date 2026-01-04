
BUGS

- in deterministic mode, I still see variations
- improve the heuristics by adding tests for bad moves

TODO

- make it smart enough to play a descend end game


TESTS

Filter by test name substring (across unit + integration tests):

cargo test perft_position3_depth

other test commands:

cargo test
cargo test --test '*'
cargo test --test perft_depth_1_2_and_3 --test move_tests --test pv_test --test blunder_tests --no-capture


powershell script to run all tests in a directory:

Get-ChildItem tests\chess -Filter *.rs | ForEach-Object {
$name = $_.BaseName
$args += @('--test', $name)
}
cargo test @args -- --nocapture
