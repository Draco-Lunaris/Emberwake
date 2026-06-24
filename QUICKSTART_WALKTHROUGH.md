# Quickstart Walkthrough (T084)

This walkthrough guides you through setting up and using Emberwake from scratch.
Each step has clear verification criteria and is marked as Verified or Unverified.

## Prerequisites

- Rust toolchain (stable) with `cargo-leptos`
- SQLite 3.x
- A web browser

## Step 1: Build the Project

```bash
cargo leptos build --release
```

**Verification criteria:** Build completes without errors. Server binary at
`target/release/emberwake`, WASM frontend in `target/site/pkg/`.

**Status:** Verified — build completes successfully in CI and locally.

## Step 2: Start the Server

```bash
./target/release/emberwake
```

Or for development:
```bash
cargo leptos watch
```

**Verification criteria:** Server logs `Listening on http://0.0.0.0:5005` and
`curl http://localhost:5005/healthz` returns `{"status":"ok"}`.

**Status:** Verified — server starts and healthz responds. Live instance at
http://emberwake.moon-dragon.us:5005/ confirms this.

## Step 3: Access the Dashboard

Open `http://localhost:5005/` in your browser.

**Verification criteria:** Page loads with title "Emberwake" and shows
navigation links: Add Service, Add Bookmark, Add Category, Settings, Account,
Admin, Logout. A search input is visible. If no services exist, shows
"No services yet. Click Add Service to get started."

**Status:** Verified — dashboard loads at http://emberwake.moon-dragon.us:5005/
with all navigation links and search input visible.

## Step 4: Create an Account (First Run)

Navigate to `/setup` on first run. Fill in username, password, and email
to create the admin account.

**Verification criteria:** Setup page at `/setup` shows admin creation form.
After submitting, success message "Admin account created" appears and you
are redirected to login.

**Status:** Unverified — setup flow exists in code but has not been
e2e-tested in this environment. The `/setup` route and SetupPage component
are implemented.

## Step 5: Log In

Navigate to `/login`. Enter credentials:
- Username: `kwslavens`
- Password: `testpass123`

**Verification criteria:** Login form at `/login` accepts credentials and
redirects to dashboard at `/`. Dashboard h1 "Emberwake" is visible.

**Status:** Verified — login works with credentials kwslavens / testpass123
on the live instance.

## Step 6: Add a Service

1. Log in as admin
2. Click "Add Service" on the dashboard (navigates to `/edit/service`)
3. Fill in the service name, URL, and optionally category/description
4. Submit the form

**Verification criteria:** Service appears on the dashboard after submission.
The `/edit/service` route renders the ServiceEditor component with form
inputs for name, URL, and category selection.

**Status:** Unverified — the `/edit/service` route exists and ServiceEditor
renders, but full form submission and dashboard update has not been
e2e-tested in this environment.

## Step 7: Add a Bookmark

1. Click "Add Bookmark" on the dashboard (navigates to `/edit/bookmark`)
2. Fill in the bookmark name, URL, and optionally category
3. Submit the form

**Verification criteria:** Bookmark appears on the dashboard after submission.
The `/edit/bookmark` route renders the BookmarkEditor component.

**Status:** Unverified — the `/edit/bookmark` route exists and BookmarkEditor
renders, but full form submission has not been e2e-tested.

## Step 8: Add a Category

1. Click "Add Category" on the dashboard (navigates to `/edit/category`)
2. Fill in the category name
3. Submit the form

**Verification criteria:** Category appears on the dashboard after submission.
The `/edit/category` route renders the CategoryEditor component.

**Status:** Unverified — the `/edit/category` route exists and CategoryEditor
renders, but full form submission has not been e2e-tested.

## Step 9: Use Search

Type a search query in the search input on the dashboard.

**Verification criteria:** Search input is visible on the dashboard. Typing
a query shows matching services and bookmarks. Search prefixes (e.g., `g`
for Google) route to configured search provider URLs.

**Status:** Unverified — search component renders with providers prop, but
live search results have not been verified in this environment.

## Step 10: Configure Settings

Navigate to `/settings` (admin only). Configure weather, search providers,
integrations (Docker, K8s), and themes.

**Verification criteria:** Settings page at `/settings` loads for admin users.
Theme changes apply without page reload (CSS custom properties injected
during SSR).

**Status:** Unverified — settings page and theme system exist in code but
have not been verified in the live environment.

## Step 11: Account Management

Navigate to `/account` to view account information and sign out.

**Verification criteria:** Account page at `/account` shows user info and
a "Sign Out" button. Clicking Sign Out redirects to `/login`.

**Status:** Unverified — account page exists in code but has not been
e2e-tested in this environment.

## Step 12: Admin Panel

Navigate to `/admin` (admin only). Manage users, view system info.

**Verification criteria:** Admin page at `/admin` loads for admin users
and shows user management interface.

**Status:** Unverified — admin page exists in code but has not been verified
in the live environment.

## Step 13: Health and Metrics

```bash
curl http://localhost:5005/healthz
curl http://localhost:5005/readyz
curl http://localhost:5005/metrics
```

**Verification criteria:** healthz returns `{"status":"ok"}`, readyz returns
database connectivity status, metrics returns Prometheus format metrics.

**Status:** Verified — all endpoints respond correctly on the live instance.

## Unverified Items Summary

The following items exist in code but have not been end-to-end tested in
the live environment:
- First-run setup flow (`/setup`)
- Full form submission for service/bookmark/category creation
- Search results display
- Settings page (`/settings`)
- Account page (`/account`) sign-out flow
- Admin panel (`/admin`)
- OIDC login flow (requires external IdP)
- WebAuthn passkey registration/login (requires authenticator)
- Docker/K8s discovery (requires running containers/cluster)
- Import/export functionality (requires file upload)
- Rate limiting (requires enabling in config and testing via HTTP)

These items would be verified by running the E2E test suite (`cargo test -p e2e -- --ignored`)
with a WebDriver server and live Emberwake instance.
