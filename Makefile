.PHONY: check build

check:
	python3 -m unittest discover -s tests
	python3 scripts/check_preview.py

build:
	python3 scripts/build_preview.py
