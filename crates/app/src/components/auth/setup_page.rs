//! First-run setup page — shown when setup_status returns Open.

use crate::domain::AdminSetupInput;
use leptos::prelude::*;

#[component]
pub fn SetupPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let done = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let input = AdminSetupInput {
            username: username.get(),
            password: password.get(),
            email: if email.get().is_empty() {
                None
            } else {
                Some(email.get())
            },
        };
        leptos::task::spawn_local(async move {
            match crate::server::auth::complete_setup(input).await {
                Ok(_) => {
                    done.set(true);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }
        });
    };

    view! {
        <div class="setup-page">
            <h1>"First-Run Setup"</h1>
            {move || if done.get() {
                view! {
                    <p>"Admin account created! You can now "
                        <a href="/login">"log in"</a>"."</p>
                }.into_any()
            } else {
                view! {
                    <form on:submit=submit>
                        <label>"Username"</label>
                        <input type="text" bind:value=username required />
                        <label>"Password"</label>
                        <input type="password" bind:value=password required minlength="8" />
                        <label>"Email (optional)"</label>
                        <input type="email" bind:value=email />
                        {move || error.get().map(|e| view! { <p class="error">{e}</p> })}
                        <button type="submit">"Create Admin Account"</button>
                    </form>
                }.into_any()
            }}
        </div>
    }
}
