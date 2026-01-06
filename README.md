
BUGS

- in deterministic mode, I still see variations

TODO

- improve the heuristics by adding tests for bad moves
- make it smart enough to play a descend end game


TESTS

powershell script to run all tests in a directory:

Get-ChildItem tests\chess -Filter *.rs | ForEach-Object {
$name = $_.BaseName
$args += @('--test', $name)
}
cargo test @args -- --nocapture
