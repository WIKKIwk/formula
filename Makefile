.PHONY: run demo test fmt check sample sample-tsv sample-xlsx

run:
	cargo run -- --interactive

demo:
	cargo run -- --demo

test:
	cargo test

fmt:
	cargo fmt

check:
	cargo check

sample:
	cargo run -- --file examples/sample.csv

sample-tsv:
	cargo run -- --file examples/sample.tsv

sample-xlsx:
	cargo run -- --file /home/abdulfattox/Downloads/test.xlsx
