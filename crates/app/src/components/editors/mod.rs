//! Editor components for content CRUD with drag-and-drop reorder and optimistic UI.
//! Forms for creating/editing services, bookmarks, and categories.
//! Drag-and-drop reorder via HTML5 drag events. Optimistic UI updates
//! that revert on server error. Validation errors displayed inline.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::domain::{
    Application, ApplicationInput, ApplicationPatch, Bookmark, BookmarkInput, BookmarkPatch,
    Category, CategoryInput, CategoryPatch, Service, ServiceInput, ServicePatch, Visibility,
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
    let (visibility, set_visibility) = signal(Visibility::Public);
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);
    let (editing_id, set_editing_id) = signal(Option::<Uuid>::None);

    let create_action = Action::new(move |input: &CategoryInput| {
        let input = input.clone();
        async move {
            content_write::create_category(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let update_action = Action::new(move |(id, patch): &(Uuid, CategoryPatch)| {
        let (id, patch) = (*id, patch.clone());
        async move {
            content_write::update_category(id, patch)
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
        if let Some(Ok(cat)) = update_action.value().get() {
            set_categories.update(|cats| {
                if let Some(c) = cats.iter_mut().find(|c| c.id == cat.id) {
                    *c = cat.clone();
                }
            });
            set_editing_id.set(None);
            set_name.set(String::new());
            set_visibility.set(Visibility::Public);
            set_error.set(None);
        }
        if let Some(Err(e)) = update_action.value().get() {
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
                if let Some(id) = editing_id.get() {
                    let patch = CategoryPatch {
                        name: Some(name.get()),
                        icon: None,
                        visibility: Some(visibility.get()),
                    };
                    update_action.dispatch((id, patch));
                } else {
                    let input = CategoryInput {
                        name: name.get(),
                        icon: None,
                        visibility: visibility.get(),
                    };
                    create_action.dispatch(input);
                }
            }>
                <input
                    type="text"
                    placeholder="Category name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
                <select on:change=move |ev| set_visibility.set(parse_visibility(&event_target_value(&ev)))>
                    <option value="public" selected=move || visibility.get() == Visibility::Public>"Public"</option>
                    <option value="private" selected=move || visibility.get() == Visibility::Private>"Private"</option>
                    <option value="restricted" selected=move || visibility.get() == Visibility::Restricted>"Restricted"</option>
                </select>
                <button type="submit">{move || if editing_id.get().is_some() { "Update" } else { "Add" }}</button>
                {move || editing_id.get().map(|_| view! {
                    <button type="button" on:click=move |_| {
                        set_editing_id.set(None);
                        set_name.set(String::new());
                        set_visibility.set(Visibility::Public);
                        set_error.set(None);
                    }>"Cancel"</button>
                })}
            </form>
            <ul class="reorder-list">
                {move || {
                    categories.get().into_iter().map(|cat| {
                        let cat_id = cat.id;
                        let cat_name = cat.name.clone();
                        let cat_visibility = cat.visibility;
                        let edit_name = cat_name.clone();
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
                                        set_name.set(edit_name.clone());
                                        set_visibility.set(cat_visibility);
                                        set_editing_id.set(Some(cat_id));
                                    }
                                >
                                    "Edit"
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete category \"{}\"", cat_name)) {
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
    let (visibility, set_visibility) = signal(Visibility::Public);
    let (icon, set_icon) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);
    let (editing_id, set_editing_id) = signal(Option::<Uuid>::None);

    let create_action = Action::new(move |input: &ServiceInput| {
        let input = input.clone();
        async move {
            content_write::create_service(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let update_action = Action::new(move |(id, patch): &(Uuid, ServicePatch)| {
        let (id, patch) = (*id, patch.clone());
        async move {
            content_write::update_service(id, patch)
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
            set_icon.set(String::new());
            set_description.set(String::new());
            set_error.set(None);
        }
        if let Some(Err(e)) = create_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Ok(svc)) = update_action.value().get() {
            set_services.update(|svcs| {
                if let Some(s) = svcs.iter_mut().find(|s| s.id == svc.id) {
                    *s = svc.clone();
                }
            });
            set_editing_id.set(None);
            set_name.set(String::new());
            set_url.set(String::new());
            set_icon.set(String::new());
            set_description.set(String::new());
            set_visibility.set(Visibility::Public);
            set_error.set(None);
        }
        if let Some(Err(e)) = update_action.value().get() {
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
                if let Some(id) = editing_id.get() {
                    let patch = ServicePatch {
                        category_id: None,
                        name: Some(name.get()),
                        url: Some(url.get()),
                        icon: if icon.get().is_empty() { None } else { Some(icon.get()) },
                        description: Some(if description.get().is_empty() { None } else { Some(description.get()) }),
                        is_pinned: None,
                        visibility: Some(visibility.get()),
                        monitor_enabled: None,
                        monitor_kind: None,
                        monitor_target: None,
                        monitor_interval_s: None,
                    };
                    update_action.dispatch((id, patch));
                } else {
                    let input = ServiceInput {
                        category_id: None,
                        name: name.get(),
                        url: url.get(),
                        icon: if icon.get().is_empty() { None } else { Some(icon.get()) },
                        description: if description.get().is_empty() { None } else { Some(description.get()) },
                        is_pinned: false,
                        visibility: visibility.get(),
                        monitor_enabled: false,
                        monitor_kind: None,
                        monitor_target: None,
                        monitor_interval_s: None,
                    };
                    create_action.dispatch(input);
                }
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
                <input
                    type="text"
                    placeholder="Icon URL (https://example.com/icon.png)"
                    prop:value=icon
                    on:input=move |ev| set_icon.set(event_target_value(&ev))
                />
                <textarea
                    placeholder="Service description"
                    prop:value=description
                    on:input=move |ev| set_description.set(event_target_value(&ev))
                ></textarea>
                <select on:change=move |ev| set_visibility.set(parse_visibility(&event_target_value(&ev)))>
                    <option value="public" selected=move || visibility.get() == Visibility::Public>"Public"</option>
                    <option value="private" selected=move || visibility.get() == Visibility::Private>"Private"</option>
                    <option value="restricted" selected=move || visibility.get() == Visibility::Restricted>"Restricted"</option>
                </select>
                <button type="submit">{move || if editing_id.get().is_some() { "Update" } else { "Add" }}</button>
                {move || editing_id.get().map(|_| view! {
                    <button type="button" on:click=move |_| {
                        set_editing_id.set(None);
                        set_name.set(String::new());
                        set_url.set(String::new());
                        set_icon.set(String::new());
                        set_description.set(String::new());
                        set_visibility.set(Visibility::Public);
                        set_error.set(None);
                    }>"Cancel"</button>
                })}
            </form>
            <ul class="reorder-list">
                {move || {
                    services.get().into_iter().map(|svc| {
                        let svc_id = svc.id;
                        let svc_name = svc.name.clone();
                        let svc_url = svc.url.clone();
                        let svc_visibility = svc.visibility;
                        let is_pinned = svc.is_pinned;
                        let edit_name = svc_name.clone();
                        let edit_url = svc_url.clone();
                        let edit_icon = svc.icon.clone();
                        let edit_description = svc.description.clone();
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
                                        set_name.set(edit_name.clone());
                                        set_url.set(edit_url.clone());
                                        set_icon.set(edit_icon.clone().unwrap_or_default());
                                        set_description.set(edit_description.clone().unwrap_or_default());
                                        set_visibility.set(svc_visibility);
                                        set_editing_id.set(Some(svc_id));
                                    }
                                >
                                    "Edit"
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete service \"{}\"", svc_name)) {
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

// --- Application Editor ---

/// Application editor: create, edit, delete, and reorder applications.
/// Like ServiceEditor but without monitoring fields or pin toggle.
#[component]
pub fn ApplicationEditor(
    applications: ReadSignal<Vec<Application>>,
    set_applications: WriteSignal<Vec<Application>>,
) -> impl IntoView {
    use crate::domain::CategoryWithItems;
    use crate::server::content_read;

    let (name, set_name) = signal(String::new());
    let (url, set_url) = signal(String::new());
    let (visibility, set_visibility) = signal(Visibility::Public);
    let (icon, set_icon) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (category_id, set_category_id) = signal(Uuid::nil());
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);
    let (editing_id, set_editing_id) = signal(Option::<Uuid>::None);

    let categories_resource = Resource::new(
        || (),
        |_| async { content_read::list_categories().await.unwrap_or_default() },
    );

    let create_action = Action::new(move |input: &ApplicationInput| {
        let input = input.clone();
        async move {
            content_write::create_application(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let update_action = Action::new(move |(id, patch): &(Uuid, ApplicationPatch)| {
        let (id, patch) = (*id, patch.clone());
        async move {
            content_write::update_application(id, patch)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let delete_action = Action::new(move |id: &Uuid| {
        let id = *id;
        async move {
            content_write::delete_application(id)
                .await
                .map(|_| id)
                .map_err(|e| e.to_string())
        }
    });

    Effect::new(move || {
        if let Some(Ok(app)) = create_action.value().get() {
            set_applications.update(|apps| apps.push(app.clone()));
            set_name.set(String::new());
            set_url.set(String::new());
            set_icon.set(String::new());
            set_description.set(String::new());
            set_category_id.set(Uuid::nil());
            set_error.set(None);
        }
        if let Some(Err(e)) = create_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Ok(app)) = update_action.value().get() {
            set_applications.update(|apps| {
                if let Some(a) = apps.iter_mut().find(|a| a.id == app.id) {
                    *a = app.clone();
                }
            });
            set_editing_id.set(None);
            set_name.set(String::new());
            set_url.set(String::new());
            set_icon.set(String::new());
            set_description.set(String::new());
            set_category_id.set(Uuid::nil());
            set_visibility.set(Visibility::Public);
            set_error.set(None);
        }
        if let Some(Err(e)) = update_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Ok(deleted_id)) = delete_action.value().get() {
            set_applications.update(|apps| apps.retain(|a| a.id != deleted_id));
            set_pending_delete.set(None);
        }
        if let Some(Err(e)) = delete_action.value().get() {
            set_error.set(Some(e));
            set_pending_delete.set(None);
        }
    });

    view! {
        <div class="editor application-editor">
            <h3>"Applications"</h3>
            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
            <form on:submit=move |ev| {
                ev.prevent_default();
                if let Some(id) = editing_id.get() {
                    let patch = ApplicationPatch {
                        category_id: Some(if category_id.get() == Uuid::nil() { None } else { Some(category_id.get()) }),
                        name: Some(name.get()),
                        url: Some(url.get()),
                        icon: if icon.get().is_empty() { None } else { Some(icon.get()) },
                        description: Some(if description.get().is_empty() { None } else { Some(description.get()) }),
                        is_pinned: None,
                        visibility: Some(visibility.get()),
                    };
                    update_action.dispatch((id, patch));
                } else {
                    let input = ApplicationInput {
                        category_id: if category_id.get() == Uuid::nil() { None } else { Some(category_id.get()) },
                        name: name.get(),
                        url: url.get(),
                        icon: if icon.get().is_empty() { None } else { Some(icon.get()) },
                        description: if description.get().is_empty() { None } else { Some(description.get()) },
                        is_pinned: false,
                        visibility: visibility.get(),
                    };
                    create_action.dispatch(input);
                }
            }>
                <input
                    type="text"
                    placeholder="Application name"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="https://example.com"
                    prop:value=url
                    on:input=move |ev| set_url.set(event_target_value(&ev))
                />
                <input
                    type="text"
                    placeholder="Icon URL (https://example.com/icon.png)"
                    prop:value=icon
                    on:input=move |ev| set_icon.set(event_target_value(&ev))
                />
                <textarea
                    placeholder="Application description"
                    prop:value=description
                    on:input=move |ev| set_description.set(event_target_value(&ev))
                ></textarea>
                <Suspense fallback=|| view! { <p>"Loading categories..."</p> }>
                    {move || {
                        categories_resource.get().map(|cats: Vec<CategoryWithItems>| {
                            view! {
                                <select on:change=move |ev| set_category_id.set(Uuid::parse_str(&event_target_value(&ev)).unwrap_or(Uuid::nil()))>
                                    <option value="" selected=move || category_id.get() == Uuid::nil()>"No category"</option>
                                    {cats.into_iter().map(|cat| {
                                        let cid = cat.id;
                                        let cname = cat.name.clone();
                                        view! {
                                            <option value=cid.to_string() selected=move || category_id.get() == cid>
                                                {cname}
                                            </option>
                                        }
                                    }).collect::<Vec<_>>()}
                                </select>
                            }
                        })
                    }}
                </Suspense>
                <select on:change=move |ev| set_visibility.set(parse_visibility(&event_target_value(&ev)))>
                    <option value="public" selected=move || visibility.get() == Visibility::Public>"Public"</option>
                    <option value="private" selected=move || visibility.get() == Visibility::Private>"Private"</option>
                    <option value="restricted" selected=move || visibility.get() == Visibility::Restricted>"Restricted"</option>
                </select>
                <button type="submit">{move || if editing_id.get().is_some() { "Update" } else { "Add" }}</button>
                {move || editing_id.get().map(|_| view! {
                    <button type="button" on:click=move |_| {
                        set_editing_id.set(None);
                        set_name.set(String::new());
                        set_url.set(String::new());
                        set_icon.set(String::new());
                        set_description.set(String::new());
                        set_category_id.set(Uuid::nil());
                        set_visibility.set(Visibility::Public);
                        set_error.set(None);
                    }>"Cancel"</button>
                })}
            </form>
            <ul class="reorder-list">
                {move || {
                    applications.get().into_iter().map(|app| {
                        let app_id = app.id;
                        let app_name = app.name.clone();
                        let app_url = app.url.clone();
                        let app_visibility = app.visibility;
                        let edit_name = app_name.clone();
                        let edit_url = app_url.clone();
                        let edit_icon = app.icon.clone();
                        let edit_description = app.description.clone();
                        let edit_category = app.category_id;
                        view! {
                            <li
                                draggable="true"
                                on:dragstart=move |_| set_dragged_id.set(Some(app_id))
                                on:dragover=move |ev| ev.prevent_default()
                                on:drop=move |_| {
                                    if let Some(dragged) = dragged_id.get() {
                                        let mut apps = applications.get();
                                        if let Some(from) = apps.iter().position(|a| a.id == dragged)
                                            && let Some(to) = apps.iter().position(|a| a.id == app_id)
                                        {
                                            apps.swap(from, to);
                                            let order: Vec<Uuid> = apps.iter().map(|a| a.id).collect();
                                            set_applications.set(apps);
                                            spawn_local(async move { let _ = content_write::reorder_applications(None, order).await; });
                                        }
                                    }
                                    set_dragged_id.set(None);
                                }
                            >
                                <span>{app_name.clone()}</span>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        set_name.set(edit_name.clone());
                                        set_url.set(edit_url.clone());
                                        set_icon.set(edit_icon.clone().unwrap_or_default());
                                        set_description.set(edit_description.clone().unwrap_or_default());
                                        set_category_id.set(edit_category.unwrap_or(Uuid::nil()));
                                        set_visibility.set(app_visibility);
                                        set_editing_id.set(Some(app_id));
                                    }
                                >
                                    "Edit"
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete application \"{}\"", app_name)) {
                                            set_pending_delete.set(Some(app_id));
                                            delete_action.dispatch(app_id);
                                        }
                                    }
                                >
                                    {move || if pending_delete.get() == Some(app_id) { "Deleting..." } else { "Delete" }}
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
    let (visibility, set_visibility) = signal(Visibility::Public);
    let (icon, set_icon) = signal(String::new());
    let (selected_category_id, set_selected_category_id) =
        signal(category_id.unwrap_or(Uuid::nil()));
    let (error, set_error) = signal(Option::<String>::None);
    let (dragged_id, set_dragged_id) = signal(Option::<Uuid>::None);
    let (pending_delete, set_pending_delete) = signal(Option::<Uuid>::None);
    let (editing_id, set_editing_id) = signal(Option::<Uuid>::None);

    let categories_resource = Resource::new(
        || (),
        |_| async {
            crate::server::content_read::list_categories()
                .await
                .unwrap_or_default()
        },
    );

    let create_action = Action::new(move |input: &BookmarkInput| {
        let input = input.clone();
        async move {
            content_write::create_bookmark(input)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let update_action = Action::new(move |(id, patch): &(Uuid, BookmarkPatch)| {
        let (id, patch) = (*id, patch.clone());
        async move {
            content_write::update_bookmark(id, patch)
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
            set_icon.set(String::new());
            set_selected_category_id.set(Uuid::nil());
            set_error.set(None);
        }
        if let Some(Err(e)) = create_action.value().get() {
            set_error.set(Some(e));
        }
    });

    Effect::new(move || {
        if let Some(Ok(bm)) = update_action.value().get() {
            set_bookmarks.update(|bms| {
                if let Some(b) = bms.iter_mut().find(|b| b.id == bm.id) {
                    *b = bm.clone();
                }
            });
            set_editing_id.set(None);
            set_name.set(String::new());
            set_url.set(String::new());
            set_icon.set(String::new());
            set_selected_category_id.set(Uuid::nil());
            set_visibility.set(Visibility::Public);
            set_error.set(None);
        }
        if let Some(Err(e)) = update_action.value().get() {
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
                if let Some(id) = editing_id.get() {
                    let patch = BookmarkPatch {
                        category_id: Some(selected_category_id.get()),
                        name: Some(name.get()),
                        url: Some(url.get()),
                        icon: if icon.get().is_empty() { None } else { Some(icon.get()) },
                        visibility: Some(visibility.get()),
                    };
                    update_action.dispatch((id, patch));
                } else {
                    let input = BookmarkInput {
                        category_id: selected_category_id.get(),
                        name: name.get(),
                        url: url.get(),
                        icon: if icon.get().is_empty() { None } else { Some(icon.get()) },
                        visibility: visibility.get(),
                    };
                    create_action.dispatch(input);
                }
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
                <input
                    type="text"
                    placeholder="Icon URL (https://example.com/icon.png)"
                    prop:value=icon
                    on:input=move |ev| set_icon.set(event_target_value(&ev))
                />
                <Suspense fallback=|| view! { <p>"Loading categories..."</p> }>
                    {move || {
                        categories_resource.get().map(|cats: Vec<crate::domain::CategoryWithItems>| {
                            view! {
                                <select on:change=move |ev| set_selected_category_id.set(Uuid::parse_str(&event_target_value(&ev)).unwrap_or(Uuid::nil()))>
                                    <option value="" selected=move || selected_category_id.get() == Uuid::nil()>"Select category"</option>
                                    {cats.into_iter().map(|cat| {
                                        let cid = cat.id;
                                        let cname = cat.name.clone();
                                        view! {
                                            <option value=cid.to_string() selected=move || selected_category_id.get() == cid>
                                                {cname}
                                            </option>
                                        }
                                    }).collect::<Vec<_>>()}
                                </select>
                            }
                        })
                    }}
                </Suspense>
                <select on:change=move |ev| set_visibility.set(parse_visibility(&event_target_value(&ev)))>
                    <option value="public" selected=move || visibility.get() == Visibility::Public>"Public"</option>
                    <option value="private" selected=move || visibility.get() == Visibility::Private>"Private"</option>
                    <option value="restricted" selected=move || visibility.get() == Visibility::Restricted>"Restricted"</option>
                </select>
                <button type="submit">{move || if editing_id.get().is_some() { "Update" } else { "Add" }}</button>
                {move || editing_id.get().map(|_| view! {
                    <button type="button" on:click=move |_| {
                        set_editing_id.set(None);
                        set_name.set(String::new());
                        set_url.set(String::new());
                        set_icon.set(String::new());
                        set_selected_category_id.set(Uuid::nil());
                        set_visibility.set(Visibility::Public);
                        set_error.set(None);
                    }>"Cancel"</button>
                })}
            </form>
            <ul class="reorder-list">
                {move || {
                    bookmarks.get().into_iter().map(|bm| {
                        let bm_id = bm.id;
                        let bm_name = bm.name.clone();
                        let bm_url = bm.url.clone();
                        let bm_visibility = bm.visibility;
                        let edit_name = bm_name.clone();
                        let edit_url = bm_url.clone();
                        let edit_icon = bm.icon.clone();
                        let edit_category_id = bm.category_id.unwrap_or(Uuid::nil());
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
                                        set_name.set(edit_name.clone());
                                        set_url.set(edit_url.clone());
                                        set_icon.set(edit_icon.clone().unwrap_or_default());
                                        set_selected_category_id.set(edit_category_id);
                                        set_visibility.set(bm_visibility);
                                        set_editing_id.set(Some(bm_id));
                                    }
                                >
                                    "Edit"
                                </button>
                                <button
                                    type="button"
                                    on:click=move |_| {
                                        if confirm_delete(&format!("Delete bookmark \"{}\"", bm_name)) {
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

/// Parse a visibility string from a `<select>` element into a Visibility enum.
fn parse_visibility(s: &str) -> Visibility {
    match s {
        "private" => Visibility::Private,
        "restricted" => Visibility::Restricted,
        _ => Visibility::Public,
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
                            <crate::components::Navbar />
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
                            <crate::components::Navbar />
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
                            <crate::components::Navbar />
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

// --- Application Edit Page ---

/// Application edit page — shows only the ApplicationEditor form and list of existing applications.
/// Redirects to /login if not authenticated.
#[component]
pub fn ApplicationEditPage() -> impl IntoView {
    use crate::domain::ServiceFilter;
    use crate::server::{auth, content_read};
    use leptos_router::components::Redirect;

    let user = Resource::new(
        || (),
        |_| async { auth::current_user().await.unwrap_or(None) },
    );

    let applications_resource = Resource::new(
        || (),
        |_| async {
            content_read::list_applications(ServiceFilter::default())
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
                            <crate::components::Navbar />
                            <h2>"Applications"</h2>
                            <Suspense fallback=|| view! { <p>"Loading applications..."</p> }>
                                {move || {
                                    applications_resource.get().map(|apps: Vec<Application>| {
                                        let (app_signal, set_app) = signal(apps);
                                        view! { <ApplicationEditor applications=app_signal set_applications=set_app /> }
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
                            <crate::components::Navbar />
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
