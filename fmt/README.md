# SBNF-FMT

Auto-formatting tool for SBNF.

## Usage

```bash
sbnf-fmt grammar.sbnf
sbnf-fmt --write grammar.sbnf
sbnf-fmt --check grammar.sbnf
```

With no `--write` or `--check`, the formatted grammar is written to stdout.
Use `-` as the input path to read from stdin.

Options:

- `--no-reorder` keeps declarations in their original order instead of
  sorting global parameters, variables, and rules.
- `--max-width <WIDTH>` sets the maximum line length (default: 80).
