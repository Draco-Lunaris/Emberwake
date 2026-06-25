//! Dashboard, tile, and category components.
//! SSR-rendered with three sections: Services, Applications, Bookmarks.
//! Each section can be enabled/disabled and has configurable column count.

pub mod status_tile;
pub mod weather_widget;

use leptos::prelude::*;

use crate::domain::{Application, Bookmark, CategoryWithBookmarks, DashboardView};
use status_tile::StatusTile;

/// Dashboard component: renders three sections (Services → Applications → Bookmarks).
/// Each section is gated on its enabled setting and uses its own column count.
#[component]
pub fn Dashboard(data: DashboardView) -> impl IntoView {
    let settings = data.settings.clone();
    let has_services = !data.pinned_services.is_empty();
    let has_applications = !data.applications.is_empty();
    let has_categories = !data.pinned_categories.is_empty();

    view! {
        <div class="dashboard">
            {if settings.services_enabled {
                view! {
                    <section class="pinned-services" style=format!("--section-columns: {}", settings.services_columns)>
                        <h2>"Services"</h2>
                        {if has_services {
                            view! {
                                <div class="tiles">
                                    {data.pinned_services.into_iter().map(|svc| view! { <StatusTile service=svc /> }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="empty-state"><p>"No services yet. Click Add Service to get started."</p></div> }.into_any()
                        }}
                    </section>
                }.into_any()
            } else {
                ().into_any()
            }}
            {if settings.applications_enabled {
                view! {
                    <section class="applications-section" style=format!("--section-columns: {}", settings.applications_columns)>
                        <h2>"Applications"</h2>
                        {if has_applications {
                            view! {
                                <div class="tiles">
                                    {data.applications.into_iter().map(|app| view! { <ApplicationTile app=app /> }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <div class="empty-state"><p>"No applications yet. Click Add Application to get started."</p></div> }.into_any()
                        }}
                    </section>
                }.into_any()
            } else {
                ().into_any()
            }}
            {if settings.bookmarks_enabled {
                view! {
                    <section class="pinned-categories" style=format!("--section-columns: {}", settings.bookmarks_columns)>
                        {if has_categories {
                            data.pinned_categories.into_iter().map(|group| view! { <CategorySection group=group /> }).collect::<Vec<_>>().into_any()
                        } else {
                            view! { <div class="empty-state"><p>"No categories yet. Click Add Category to organize your bookmarks."</p></div> }.into_any()
                        }}
                    </section>
                }.into_any()
            } else {
                ().into_any()
            }}
        </div>
    }
}

/// Application tile — like ServiceTile but without status indicator.
#[component]
fn ApplicationTile(app: Application) -> impl IntoView {
    view! {
        <a class="tile application-tile" href=app.url.clone() target="_blank" rel="noopener noreferrer">
            {if let Some(icon) = &app.icon {
                if !icon.is_empty() {
                    view! { <img class="tile-icon" src=icon.clone() alt=app.name.clone() /> }.into_any()
                } else {
                    view! { <span class="tile-icon-placeholder">{app.name.chars().next().unwrap_or('A')}</span> }.into_any()
                }
            } else {
                view! { <span class="tile-icon-placeholder">{app.name.chars().next().unwrap_or('A')}</span> }.into_any()
            }}
            <span class="tile-name">{app.name.clone()}</span>
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
                {group.bookmarks.into_iter().map(|bm| view! { <BookmarkItem bookmark=bm /> }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

/// Bookmark list item.
#[component]
fn BookmarkItem(bookmark: Bookmark) -> impl IntoView {
    view! {
        <li>
            <a href=bookmark.url.clone() target="_blank" rel="noopener noreferrer">{bookmark.name.clone()}</a>
        </li>
    }
}
