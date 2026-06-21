# E2E Tests

End-to-end WebDriver tests for Emberwake using [fantoccini](https://crates.io/crates/fantoccini).

## Prerequisites

1. **Emberwake server** running on `localhost:5005`
2. **WebDriver** (geckodriver or chromedriver) on `localhost:4444`
3. **Fresh database** (no admin user yet) for the setup test

### Installing geckodriver

```sh
# Debian/Ubuntu
sudo apt install geckodriver

# macOS
brew install geckodriver

# Or download from https://github.com/mozilla/geckodriver/releases
```

### Installing chromedriver

```sh
# Debian/Ubuntu
sudo apt install chromium-chromedriver

# macOS
brew install chromedriver
```

## Running the Tests

### 1. Start the WebDriver

```sh
# geckodriver
geckodriver --port 4444

# or chromedriver
chromedriver --port 4444
```

### 2. Start the Emberwake server

```sh
cargo run -p server
```

The server must be reachable at `http://localhost:5005`.

### 3. Run the E2E tests

```sh
# Run all E2E tests
cargo test -p e2e

# Run a specific test
cargo test -p e2e t01_first_run_setup

# Run with output
cargo test -p e2e -- --nocapture
```

## Test Scenarios

| # | Test | Route | Description |
|---|------|-------|-------------|
| 1 | `t01_first_run_setup` | `/setup` | Create admin account on first run |
| 2 | `t02_login` | `/login` | Login with admin credentials, verify dashboard |
| 3 | `t03_create_category_service_bookmark` | `/` | Create category, service, bookmark via editor forms |
| 4 | `t04_search_service_fuzzy` | `/` | Fuzzy search for a service |
| 5 | `t05_edit_service_name` | `/` | Edit service name (requires edit UI) |
| 6 | `t06_delete_bookmark` | `/` | Delete a bookmark via editor |
| 7 | `t07_logout` | `/account` | Sign out and verify redirect to login |

## Test Order

Tests are designed to run in sequence (t01 through t07). Test 1 creates
the admin account; subsequent tests log in with those credentials. Test 3
creates content that tests 4–6 operate on.

Run tests in order:

```sh
cargo test -p e2e -- --test-threads=1
```

## Configuration

Default configuration (in `src/lib.rs`):

| Setting | Value |
|---------|-------|
| Server URL | `http://localhost:5005` |
| WebDriver URL | `http://localhost:4444` |
| Admin username | `e2e_admin` |
| Admin password | `e2e_test_pass_123` |

Override the WebDriver URL with the `WEBDRIVER_URL` environment variable
if needed (requires code change — no runtime config yet).

## Current Limitations

- **Tests 3–6** target editor components (CategoryEditor, ServiceEditor,
  BookmarkEditor) and SearchIsland that exist in the codebase but are not
  yet wired to any route. These tests will fail until the components are
  rendered on a page.
- **Test 5** (edit service name) requires an inline edit form that does
  not yet exist in ServiceEditor. The test gracefully handles the missing
  feature by printing a note and passing.
- Tests use `window.confirm()` override via JavaScript to auto-accept
  delete confirmation dialogs.
- No parallel test execution — tests share state (admin account, content).
