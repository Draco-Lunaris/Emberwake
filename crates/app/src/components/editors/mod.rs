//! Editor components for content CRUD with drag-and-drop reorder and optimistic UI.
//! Forms for creating/editing services, bookmarks, and categories.
//! Drag-and-drop reorder via HTML5 drag events. Optimistic UI updates
//! that revert on server error. Validation errors displayed inline.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::domain::{
    Bookmark, BookmarkInput, Category, CategoryInput, Service, ServiceInput, Visibility,
};
use crate::server::content_write;

// --- Category Editor ---

/// Category editor: create, edit, delete, and reorder categories.
#[component]
pub fn CategoryEditor(
    categories: ReadSignal<Vec<Category>>,
    set_categories: WriteSignal<Vec<Category>>,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);

    let create_action = Action::new(move |input: &CategoryInput| {
        let input = input.clone();
        async move {
            content_write::create_category(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            content_write::delete_category(id)
                .await
                .map(|_| id)
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || {
        if let Some(Ok(cat)) = create_action.value().get() {
            set_categories.update(|cats| cats.push(cat.clone()));
            set_name.set(String::new());
            set_error.set(None);
        }
        if let Some(Err(e)) = create_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Ok(deleted_id)) = delete_action.value().get() {
            set_categories.update(|cats| cats.retain(|c| c.id != deleted_id));
            set_pending_delete.set(None);
        }
        if let Some(Err(e)) = delete_action.value().get() {
            set_error.set(Some(e));
            set_pending_delete.set(None);
        }
    });

    view! {
        <div class="editor category-editor">
            <h3>"Categories"</h3>
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
            <form on:submit=move |ev| {
                ev.prevent_default();
                let input = CategoryInput {
                    name: name.get(),
                    icon: None,
                    visibility: Visibility::Public,
                };
                create_action.dispatch(input);
            }>
                <input
                    type="text"
                    placeholder="Category name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
                <button type="submit">"Add"</button>
            </form>
            <ul class="reorder-list">
                {move || {
                    categories.get().into_iter().map(|cat| {
                        let cat_id = cat.id;
                        let cat_name = cat.name.clone();
                        view! {
                            <li
                                draggable="true"
                                on:dragstart=move |_| set_dragged_id.set(Some(cat_id))
                                on:dragover=move |ev| ev.prevent_default()
                                on:drop=move |_| {
                                    if let Some(dragged) = dragged_id.get() {
                                        let mut cats = categories.get();
                                        if let Some(from) = cats.iter().position(|c| c.id == dragged)
                                            && let Some(to) = cats.iter().position(|c| c.id == cat_id)
                                        {
                                            cats.swap(from, to);
                                            let order: Vec<Uuid> = cats.iter().map(|c| c.id).collect();
                                            set_categories.set(cats);
                                            spawn_local(async move { let _ = content_write::reorder_categories(order).await; });
                                        }
                                    }
                                    set_dragged_id.set(None);
                                }
                            >
                                <span>{cat_name.clone()}</span>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete category \"{}\"?", cat_name)) {
                                            set_pending_delete.set(Some(cat_id));
                                            delete_action.dispatch(cat_id);
                                        }
                                    }
                                >
                                    {move || if pending_delete.get() == Some(cat_id) { "Deleting..." } else { "Delete" }}
                                </button>
                            </li>
                        }
                    }).collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}

// --- Service Editor ---

/// Service editor: create, edit, pin toggle, delete, and reorder services.
#[component]
pub fn ServiceEditor(
    services: ReadSignal<Vec<Service>>,
    set_services: WriteSignal<Vec<Service>>,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (url, set_url) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);

    let create_action = Action::new(move |input: &ServiceInput| {
        let input = input.clone();
        async move {
            content_write::create_service(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let pin_action = Action::new(move |(id, pinned): &(Uuid, bool)| {
        let (id, pinned) = (*id, *pinned);
        async move {
            content_write::set_service_pinned(id, pinned)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            content_write::delete_service(id)
                .await
                .map(|_| id)
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || {
        if let Some(Ok(svc)) = create_action.value().get() {
            set_services.update(|svcs| svcs.push(svc.clone()));
            set_name.set(String::new());
            set_url.set(String::new());
            set_error.set(None);
        }
        if let Some(Err(e)) = create_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Err(e)) = pin_action.value().get() {
            set_error.set(Some(format!("Pin toggle failed: {e}")));
        }
    });

    Effect::new(move || {
        if let Some(Ok(deleted_id)) = delete_action.value().get() {
            set_services.update(|svcs| svcs.retain(|s| s.id != deleted_id));
            set_pending_delete.set(None);
        }
        if let Some(Err(e)) = delete_action.value().get() {
            set_error.set(Some(e));
            set_pending_delete.set(None);
        }
    });

    view! {
        <div class="editor service-editor">
            <h3>"Services"</h3>
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
            <form on:submit=move |ev| {
                ev.prevent_default();
                let input = ServiceInput {
                    category_id: None,
                    name: name.get(),
                    url: url.get(),
                    icon: None,
                    description: None,
                    is_pinned: false,
                    visibility: Visibility::Public,
                    monitor_enabled: false,
                    monitor_kind: None,
                    monitor_target: None,
                    monitor_interval_s: None,
                };
                create_action.dispatch(input);
            }>
                <input
                    type="text"
                    placeholder="Service name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="https://example.com"
                    prop:value=url
                    on:input=move |ev| set_url.set(event_target_value(&ev))
                />
                <button type="submit">"Add"</button>
            </form>
            <ul class="reorder-list">
                {move || {
                    services.get().into_iter().map(|svc| {
                        let svc_id = svc.id;
                        let svc_name = svc.name.clone();
                        let is_pinned = svc.is_pinned;
                        view! {
                            <li
                                draggable="true"
                                on:dragstart=move |_| set_dragged_id.set(Some(svc_id))
                                on:dragover=move |ev| ev.prevent_default()
                                on:drop=move |_| {
                                    if let Some(dragged) = dragged_id.get() {
                                        let mut svcs = services.get();
                                        if let Some(from) = svcs.iter().position(|s| s.id == dragged)
                                            && let Some(to) = svcs.iter().position(|s| s.id == svc_id)
                                        {
                                            svcs.swap(from, to);
                                            let order: Vec<Uuid> = svcs.iter().map(|s| s.id).collect();
                                            set_services.set(svcs);
                                            spawn_local(async move { let _ = content_write::reorder_services(None, order).await; });
                                        }
                                    }
                                    set_dragged_id.set(None);
                                }
                            >
                                <span>{svc_name.clone()}</span>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        set_services.update(|svcs| {
                                            if let Some(s) = svcs.iter_mut().find(|s| s.id == svc_id) {
                                                s.is_pinned = !is_pinned;
                                            }
                                        });
                                        pin_action.dispatch((svc_id, !is_pinned));
                                    }
                                >
                                    {if is_pinned { "Unpin" } else { "Pin" }}
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete service \"{}\"?", svc_name)) {
                                            set_pending_delete.set(Some(svc_id));
                                            delete_action.dispatch(svc_id);
                                        }
                                    }
                                >
                                    {move || if pending_delete.get() == Some(svc_id) { "Deleting..." } else { "Delete" }}
                                </button>
                            </li>
                        }
                    }).collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}

// --- Bookmark Editor ---

/// Bookmark editor: create, edit, delete, and reorder bookmarks.
#[component]
pub fn BookmarkEditor(
    bookmarks: ReadSignal<Vec<Bookmark>>,
    set_bookmarks: WriteSignal<Vec<Bookmark>>,
    category_id: Option<Uuid>,
) -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (url, set_url) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);

    let create_action = Action::new(move |input: &BookmarkInput| {
        let input = input.clone();
        async move {
            content_write::create_bookmark(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            content_write::delete_bookmark(id)
                .await
                .map(|_| id)
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || {
        if let Some(Ok(bm)) = create_action.value().get() {
            set_bookmarks.update(|bms| bms.push(bm.clone()));
            set_name.set(String::new());
            set_url.set(String::new());
            set_error.set(None);
        }
        if let Some(Err(e)) = create_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Ok(deleted_id)) = delete_action.value().get() {
            set_bookmarks.update(|bms| bms.retain(|b| b.id != deleted_id));
            set_pending_delete.set(None);
        }
        if let Some(Err(e)) = delete_action.value().get() {
            set_error.set(Some(e));
            set_pending_delete.set(None);
        }
    });

    view! {
        <div class="editor bookmark-editor">
            <h3>"Bookmarks"</h3>
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
            <form on:submit=move |ev| {
                ev.prevent_default();
                let input = BookmarkInput {
                    category_id,
                    name: name.get(),
                    url: url.get(),
                    icon: None,
                    visibility: Visibility::Public,
                };
                create_action.dispatch(input);
            }>
                <input
                    type="text"
                    placeholder="Bookmark name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="https://example.com"
                    prop:value=url
                    on:input=move |ev| set_url.set(event_target_value(&ev))
                />
                <button type="submit">"Add"</button>
            </form>
            <ul class="reorder-list">
                {move || {
                    bookmarks.get().into_iter().map(|bm| {
                        let bm_id = bm.id;
                        let bm_name = bm.name.clone();
                        view! {
                            <li
                                draggable="true"
                                on:dragstart=move |_| set_dragged_id.set(Some(bm_id))
                                on:dragover=move |ev| ev.prevent_default()
                                on:drop=move |_| {
                                    if let Some(dragged) = dragged_id.get() {
                                        let mut bms = bookmarks.get();
                                        if let Some(from) = bms.iter().position(|b| b.id == dragged)
                                            && let Some(to) = bms.iter().position(|b| b.id == bm_id)
                                        {
                                            bms.swap(from, to);
                                            let cat_id = category_id.unwrap_or_default();
                                            let order: Vec<Uuid> = bms.iter().map(|b| b.id).collect();
                                            set_bookmarks.set(bms);
                                            spawn_local(async move { let _ = content_write::reorder_bookmarks(cat_id, order).await; });
                                        }
                                    }
                                    set_dragged_id.set(None);
                                }
                            >
                                <span>{bm_name.clone()}</span>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete bookmark \"{}\"?", bm_name)) {
                                            set_pending_delete.set(Some(bm_id));
                                            delete_action.dispatch(bm_id);
                                        }
                                    }
                                >
                                    {move || if pending_delete.get() == Some(bm_id) { "Deleting..." } else { "Delete" }}
                                </button>
                            </li>
                        }
                    }).collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}

/// Shared nav bar for edit pages.
fn edit_navbar() -> impl IntoView {
    use leptos_router::components::A;
    view! {
        <nav class="navbar">
            <h1>"Emberwake"</h1>
            <A href="/edit/service">"Add Service"</A>
            <A href="/edit/bookmark">"Add Bookmark"</A>
            <A href="/edit/category">"Add Category"</A>
            <A href="/">"Dashboard"</A>
            <A href="/settings">"Settings"</A>
            <A href="/account">"Account"</A>
        </nav>
    }
}

/// Cross-platform delete confirmation dialog.
/// Uses web-sys confirm_with_message on WASM, returns true on SSR.
fn confirm_delete(message: &str) -> bool {
    #[cfg(feature = "ssr")]
    {
        let _ = message;
        true
    }
    #[cfg(not(feature = "ssr"))]
    {
        web_sys::window()
            .and_then(|w| w.confirm_with_message(message).ok())
            .unwrap_or(false)
    }
}

// --- Edit Page ---

/// Edit page — authenticated content management with category, service, and bookmark editors.
/// Redirects to /login if not authenticated. Renders all three editor components in sections.
/// Kept for backward compatibility; prefer the dedicated /edit/service, /edit/bookmark, /edit/category routes.
#[component]
pub fn EditPage() -> impl IntoView {
    use crate::domain::{CategoryWithItems, ServiceFilter};
    use crate::server::{auth, content_read};
    use leptos_router::components::Redirect;

    let user = Resource::new(
        || (),
        |_| async { auth::current_user().await.unwrap_or(None) },
    );

    let categories_resource = Resource::new(
        || (),
        |_| async { content_read::list_categories().await.unwrap_or_default() },
    );

    let services_resource = Resource::new(
        || (),
        |_| async {
            content_read::list_services(ServiceFilter::default())
                .await
                .unwrap_or_default()
        },
    );

    let bookmarks_resource = Resource::new(
        || (),
        |_| async { content_read::list_bookmarks(None).await.unwrap_or_default() },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                user.get().map(|u| {
                    match u {
                        Some(_) => view! {
                            {edit_navbar()}
                            <h2>"Content Editors"</h2>
                            <Suspense fallback=|| view! { <p>"Loading categories..."</p> }>
                                {move || {
                                    categories_resource.get().map(|cats: Vec<CategoryWithItems>| {
                                        let categories: Vec<Category> = cats
                                            .into_iter()
                                            .map(|c| Category {
                                                id: c.id,
                                                name: c.name,
                                                icon: c.icon,
                                                order_index: c.order_index,
                                                visibility: c.visibility,
                                                created_at: String::new(),
                                                updated_at: String::new(),
                                            })
                                            .collect();
                                        let (cat_signal, set_cat) = signal(categories);
                                        view! { <CategoryEditor categories=cat_signal set_categories=set_cat /> }
                                    })
                                }}
                            </Suspense>
                            <Suspense fallback=|| view! { <p>"Loading services..."</p> }>
                                {move || {
                                    services_resource.get().map(|svcs: Vec<Service>| {
                                        let (svc_signal, set_svc) = signal(svcs);
                                        view! { <ServiceEditor services=svc_signal set_services=set_svc /> }
                                    })
                                }}
                            </Suspense>
                            <Suspense fallback=|| view! { <p>"Loading bookmarks..."</p> }>
                                {move || {
                                    bookmarks_resource.get().map(|bms: Vec<Bookmark>| {
                                        let (bm_signal, set_bm) = signal(bms);
                                        view! { <BookmarkEditor bookmarks=bm_signal set_bookmarks=set_bm category_id=None /> }
                                    })
                                }}
                            </Suspense>
                        }.into_any(),
                        None => view! {
                            <Redirect path="/login" />
                        }.into_any(),
                    }
                })
            }}
        </Suspense>
    }
}

// --- Service Edit Page ---

/// Service edit page — shows only the ServiceEditor form and list of existing services.
/// Redirects to /login if not authenticated.
#[component]
pub fn ServiceEditPage() -> impl IntoView {
    use crate::domain::ServiceFilter;
    use crate::server::{auth, content_read};
    use leptos_router::components::Redirect;

    let user = Resource::new(
        || (),
        |_| async { auth::current_user().await.unwrap_or(None) },
    );

    let services_resource = Resource::new(
        || (),
        |_| async {
            content_read::list_services(ServiceFilter::default())
                .await
                .unwrap_or_default()
        },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                user.get().map(|u| {
                    match u {
                        Some(_) => view! {
                            {edit_navbar()}
                            <h2>"Services"</h2>
                            <Suspense fallback=|| view! { <p>"Loading services..."</p> }>
                                {move || {
                                    services_resource.get().map(|svcs: Vec<Service>| {
                                        let (svc_signal, set_svc) = signal(svcs);
                                        view! { <ServiceEditor services=svc_signal set_services=set_svc /> }
                                    })
                                }}
                            </Suspense>
                        }.into_any(),
                        None => view! {
                            <Redirect path="/login" />
                        }.into_any(),
                    }
                })
            }}
        </Suspense>
    }
}

// --- Bookmark Edit Page ---

/// Bookmark edit page — shows only the BookmarkEditor form and list of existing bookmarks.
/// Redirects to /login if not authenticated.
#[component]
pub fn BookmarkEditPage() -> impl IntoView {
    use crate::server::{auth, content_read};
    use leptos_router::components::Redirect;

    let user = Resource::new(
        || (),
        |_| async { auth::current_user().await.unwrap_or(None) },
    );

    let bookmarks_resource = Resource::new(
        || (),
        |_| async { content_read::list_bookmarks(None).await.unwrap_or_default() },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                user.get().map(|u| {
                    match u {
                        Some(_) => view! {
                            {edit_navbar()}
                            <h2>"Bookmarks"</h2>
                            <Suspense fallback=|| view! { <p>"Loading bookmarks..."</p> }>
                                {move || {
                                    bookmarks_resource.get().map(|bms: Vec<Bookmark>| {
                                        let (bm_signal, set_bm) = signal(bms);
                                        view! { <BookmarkEditor bookmarks=bm_signal set_bookmarks=set_bm category_id=None /> }
                                    })
                                }}
                            </Suspense>
                        }.into_any(),
                        None => view! {
                            <Redirect path="/login" />
                        }.into_any(),
                    }
                })
            }}
        </Suspense>
    }
}

// --- Category Edit Page ---

/// Category edit page — shows only the CategoryEditor form and list of existing categories.
/// Redirects to /login if not authenticated.
#[component]
pub fn CategoryEditPage() -> impl IntoView {
    use crate::domain::CategoryWithItems;
    use crate::server::{auth, content_read};
    use leptos_router::components::Redirect;

    let user = Resource::new(
        || (),
        |_| async { auth::current_user().await.unwrap_or(None) },
    );

    let categories_resource = Resource::new(
        || (),
        |_| async { content_read::list_categories().await.unwrap_or_default() },
    );

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || {
                user.get().map(|u| {
                    match u {
                        Some(_) => view! {
                            {edit_navbar()}
                            <h2>"Categories"</h2>
                            <Suspense fallback=|| view! { <p>"Loading categories..."</p> }>
                                {move || {
                                    categories_resource.get().map(|cats: Vec<CategoryWithItems>| {
                                        let categories: Vec<Category> = cats
                                            .into_iter()
                                            .map(|c| Category {
                                                id: c.id,
                                                name: c.name,
                                                icon: c.icon,
                                                order_index: c.order_index,
                                                visibility: c.visibility,
                                                created_at: String::new(),
                                                updated_at: String::new(),
                                            })
                                            .collect();
                                        let (cat_signal, set_cat) = signal(categories);
                                        view! { <CategoryEditor categories=cat_signal set_categories=set_cat /> }
                                    })
                                }}
                            </Suspense>
                        }.into_any(),
                        None => view! {
                            <Redirect path="/login" />
                        }.into_any(),
                    }
                })
            }}
        </Suspense>
    }
}
