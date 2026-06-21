//! Dashboard, tile, and category components.
//! SSR-rendered with minimal hydration — mostly static content.

pub mod status_tile;
pub mod weather_widget;

use leptos::prelude::*;

use crate::domain::{Bookmark, CategoryWithBookmarks, DashboardView, Service};

/// Dashboard component: renders pinned services and bookmark groups.
#[component]
pub fn Dashboard(data: DashboardView) -> impl IntoView {
    view! {
        <div class="dashboard">
            <section class="pinned-services">
                <h2>"Services"</h2>
                <div class="tiles">
                    {data
                        .pinned_services
                        .into_iter()
                        .map(|svc| view! { <ServiceTile service=svc /> })
                        .collect::<Vec<_>>()}
                </div>
            </section>
            <section class="pinned-categories">
                {data
                    .pinned_categories
                    .into_iter()
                    .map(|group| view! { <CategorySection group=group /> })
                    .collect::<Vec<_>>()}
            </section>
        </div>
    }
}

/// Service tile component.
#[component]
fn ServiceTile(service: Service) -> impl IntoView {
    view! {
        <a class="tile" href=service.url.clone()>
            {if let Some(icon) = &service.icon {
                view! { <img class="tile-icon" src=icon.clone() alt=service.name.clone() /> }.into_any()
            } else {
                view! { <span class="tile-icon-placeholder">{service.name.chars().next().unwrap_or('E')}</span> }.into_any()
            }}
            <span class="tile-name">{service.name.clone()}</span>
            {if let Some(desc) = &service.description {
                view! { <span class="tile-desc">{desc.clone()}</span> }.into_any()
            } else {
                ().into_any()
            }}
        </a>
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
