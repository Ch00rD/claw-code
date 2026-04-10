
## Testing
Unset LLM_BASE_URL, LLM_API_KEY, and LLM_PROVIDER before running tests:

```bash
unset LLM_BASE_URL LLM_API_KEY LLM_PROVIDER
cargo test --workspace
```

### Known flaky tests (pre-existing upstream issues, pass in isolation)
These tests fail under parallel execution due to filesystem/process state interference:
- `init::tests::initialize_repo_creates_expected_files_and_gitignore_entries`
- `tests::repl_executes_python_code`

Skip them in full suite runs:

```bash
cargo test --workspace -- \
  --skip init::tests::initialize_repo_creates_expected_files_and_gitignore_entries \
  --skip tests::repl_executes_python_code
```
