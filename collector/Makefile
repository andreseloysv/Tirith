.PHONY: install test collect demo status fixtures

install:
	uv sync

test:
	uv run pytest -q

collect:
	uv run tirith collect

demo:
	uv run python demo.py

status:
	uv run tirith status

fixtures:  # regenerate fixtures/state*.js from the JSON fixtures
	uv run python fixture_js.py
