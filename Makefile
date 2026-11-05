.PHONY: check build build-swift-reference

check:
	python3 scripts/check_desktop.py

build:
	python3 scripts/build_desktop.py

build-swift-reference:
	python3 scripts/build_preview.py
