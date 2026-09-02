# Contributing

AIUsage requires Python 3.10 or newer and has no runtime dependencies. Create
an isolated development environment if desired, then install the project:

```sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -e '.[dev]'
python -m unittest discover -s tests -v
```

Provider adapters return a `ProviderUsage` containing an availability state
and zero or more `RateLimitWindow` values. A real adapter must use a reliable,
verifiable data source. It must never infer or fabricate usage, extract stored
credentials, or log authentication material. UI work may use deterministic
fixtures through demo mode.

Keep changes focused, readable, and compatible with Python 3.10. Pull requests
should include relevant tests, pass the offline test suite, avoid credentials
and machine-specific data, and update documentation when behavior changes.

