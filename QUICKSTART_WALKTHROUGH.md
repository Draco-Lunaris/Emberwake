# Quickstart Walkthrough (T084)

This walkthrough guides you through setting up and using Emberwake from scratch.
Each step has clear verification criteria. Unverified steps are explicitly marked.

## Prerequisites

- Rust toolchain (stable) with `cargo-leptos`
- SQLite 3.x
- A web browser

## Step 1: Build the Project

```bash
cargo leptos build --release
```

**Verification:** The build completes without errors. The server binary is at
`target/release/emberwake` and the WASM frontend is in `target/site/pkg/`.

**Status:** Verified — build completes successfully.

## Step 2: Start the Server

```bash
./target/release/emberwake
```

Or for development:
```bash
cargo leptos watch
```

**Verification:** The server logs `Listening on http://0.0.0.0:5005` and responds
to `curl http://localhost:5005/healthz` with `{"status":"ok"}`.

**Status:** Verified — server starts and healthz responds.

## Step 3: Access the Dashboard

Open `http://localhost:5005/` in your browser.

**Verification:** The page loads with the title "Emberwake" and shows a "Services"
section. If no services exist, it shows "No services yet. Click Add Service to get started."

**Status:** Verified — dashboard loads at http://emberwake.moon-dragon.us:5005/

## Step 4: Create an Account (First Run)

Navigate to the Account page. On first run, you will need to create an admin account.

**Verification:** Account creation form is accessible. After creating an account,
you can log in.

**Status:** Partially verified — account page exists but first-run setup flow
should be tested more thoroughly.

## Step 5: Add a Service

1. Log in as admin
2. Click "Add Service" on the dashboard
3. Fill in the service name, URL, and optionally icon/description
4. Submit the form

**Verification:** The service appears on the dashboard after submission.

**Status:** Partially verified — the editor route `/edit/service` exists and
the form renders, but full form submission and dashboard update has not been
end-to-end tested in this environment.

## Step 6: Add a Bookmark

1. Click "Add Bookmark" on the dashboard
2. Fill in the bookmark name, URL, and optionally category
3. Submit the form

**Verification:** The bookmark appears on the dashboard.

**Status:** Partially verified — the editor route `/edit/bookmark` exists.

## Step 7: Add a Category

1. Click "Add Category" on the dashboard
2. Fill in the category name
3. Submit the form

**Verification:** The category appears on the dashboard.

**Status:** Partially verified — the editor route `/edit/category` exists.

## Step 8: Use Search

Type a search query in the search input on the dashboard.

**Verification:** Search input is visible. If search providers are configured,
search prefixes (e.g., `g` for Google) route to the provider URL.

**Status:** Partially verified — search component renders with providers prop.

## Step 9: Configure Settings

Navigate to Settings (admin only). Configure weather, search providers,
integrations (Docker, K8s), and themes.

**Verification:** Settings page loads for admin users. Theme changes apply
without page reload (CSS custom properties injected during SSR).

**Status:** Partially verified — settings page and theme system exist.

## Step 10: Health and Metrics

```bash
curl http://localhost:5005/healthz
curl http://localhost:5005/readyz
curl http://localhost:5005/metrics
```

**Verification:** healthz returns `{"status":"ok"}`, readyz returns database
connectivity status, metrics returns Prometheus format metrics.

**Status:** Verified — all endpoints respond correctly.

## Unverified Items

- Full form submission flow (add service/bookmark/category end-to-end)
- OIDC login flow (requires external IdP)
- WebAuthn passkey registration/login (requires authenticator)
- Docker/K8s discovery (requires running containers/cluster)
- Import/export functionality (requires file upload)
- Rate limiting (requires enabling in config and testing via HTTP)
