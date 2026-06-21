//! Login page — username + password form.

use crate::domain::LoginInput;
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let input = LoginInput {
            username: username.get(),
            password: password.get(),
        };
        leptos::task::spawn_local(async move {
            match crate::server::auth::login(input).await {
                Ok(_) => {
                    leptos_router::hooks::use_navigate()("/", Default::default());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    view! {
        <div class="login-page">
            <h1>"Sign In"</h1>
            <form on:submit=submit>
                <label>"Username"</label>
                <input type="text" bind:value=username required />
                <label>"Password"</label>
                <input type="password" bind:value=password required />
                {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
                <button type="submit">"Log In"</button>
            </form>
        </div>
    }
}
