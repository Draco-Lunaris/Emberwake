//! Dashboard, tile, and category components.
//! SSR-rendered with minimal hydration — mostly static content.

pub mod status_tile;
pub mod weather_widget;

use leptos::prelude::*;

use crate::domain::{Bookmark, CategoryWithBookmarks, DashboardView};
use status_tile::StatusTile;

/// Dashboard component: renders pinned services and bookmark groups.
#[component]
pub fn Dashboard(data: DashboardView) -> impl IntoView {
    let has_services = !data.pinned_services.is_empty();
    let has_categories = !data.pinned_categories.is_empty();

    view! {
        <div class="dashboard">
            <section class="pinned-services">
                <h2>"Services"</h2>
                {if has_services {
                    view! {
                        <div class="tiles">
                            {data
                                .pinned_services
                                .into_iter()
                                .map(|svc| view! { <StatusTile service=svc /> })
                                .collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="empty-state">
                            <p>"No services yet. Click Add Service to get started."</p>
                        </div>
                    }.into_any()
                }}
            </section>
            <section class="pinned-categories">
                {if has_categories {
                    data
                        .pinned_categories
                        .into_iter()
                        .map(|group| view! { <CategorySection group=group /> })
                        .collect::<Vec<_>>()
                        .into_any()
                } else {
                    view! {
                        <div class="empty-state">
                            <p>"No categories yet. Click Add Category to organize your bookmarks."</p>
                        </div>
                    }.into_any()
                }}
            </section>
        </div>
    }
}

/// Category section with bookmark groups.
#[component]
fn CategorySection(group: CategoryWithBookmarks) -> impl IntoView {
    view! {
        <div class="category">
            <h3>{group.category.name.clone()}</h3>
            <ul class="bookmarks">
                {group
                    .bookmarks
                    .into_iter()
                    .map(|bm| view! { <BookmarkItem bookmark=bm /> })
                    .collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

/// Bookmark list item.
#[component]
fn BookmarkItem(bookmark: Bookmark) -> impl IntoView {
    view! {
        <li>
            <a href=bookmark.url.clone()>{bookmark.name.clone()}</a>
        </li>
    }
}
