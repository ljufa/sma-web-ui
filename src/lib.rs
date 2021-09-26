#![allow(clippy::wildcard_imports)]
// @TODO: Remove.
#![allow(dead_code, unused_variables)]

use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};

mod page;

const SETTINGS: &str = "settings";

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders
        .subscribe(Msg::UrlChanged)
        .stream(streams::window_event(Ev::Click, |_| Msg::HideMenu))
        .perform_cmd(async { 
            Msg::AuthConfigFetched(
                async { fetch("/auth_config.json").await?.check_status()?.json().await }.await
            )
        });

    Model {
        ctx: Context {
            user: None,
            token: None,
        },
        base_url: url.to_base_url(),
        page: Page::init(url, orders),
        menu_visible: false,
        auth_config: None,
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    ctx: Context,
    base_url: Url,
    page: Page,
    menu_visible: bool,
    auth_config: Option<AuthConfig>,
}
#[derive(Clone)]
struct Context {
    user: Option<User>,
    token: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct User {
    nickname: String,
    name: String,
    picture: String,
    updated_at: String,
    sub: String,
}

// ------ Page ------

enum Page {
    Home,
    Settings(page::settings::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Self {
        match url.remaining_path_parts().as_slice() {
            [] => Self::Home,
            [SETTINGS] => Self::Settings(
                page::settings::init(url, &mut orders.proxy(Msg::SettingsMsg))
            ),
            _ => Self::NotFound,
        }
    }
}

// ------ AuthConfig ------

#[derive(Deserialize)]
struct AuthConfig {
    domain: String,
    client_id: String,
    audience: String,
}

// ------ ------
//     Urls
// ------ ------

struct_urls!();
impl<'a> Urls<'a> {
    fn home(self) -> Url {
        self.base_url()
    }
    fn settings(self) -> Url {
        self.base_url().add_path_part(SETTINGS)
    }
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    UrlChanged(subs::UrlChanged),
    ToggleMenu,
    HideMenu,
    AuthConfigFetched(fetch::Result<AuthConfig>),
    AuthInitialized(Result<JsValue, JsValue>),
    SignUp,
    LogIn,
    LogOut,
    LoggedIn(fetch::Result<String>),
    RedirectingToSignUp(Result<(), JsValue>),
    RedirectingToLogIn(Result<(), JsValue>),

    // ------ pages ------

    SettingsMsg(page::settings::Msg),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(subs::UrlChanged(url)) => model.page = Page::init(url, orders),
        Msg::ToggleMenu => model.menu_visible = not(model.menu_visible),
        Msg::HideMenu => {
            if model.menu_visible {
                model.menu_visible = false;
            } else {
                orders.skip();
            }
        },
        Msg::AuthConfigFetched(Ok(auth_config)) => {
            let domain = auth_config.domain.clone();
            let client_id = auth_config.client_id.clone();
            let audience = auth_config.audience.clone();

            orders.perform_cmd(async { Msg::AuthInitialized(
                init_auth(domain, client_id, audience).await
            )});
            model.auth_config = Some(auth_config);
        },
        Msg::AuthConfigFetched(Err(fetch_error)) => error!("AuthConfig fetch failed!", fetch_error),
        Msg::AuthInitialized(Ok(js_user)) => {
            log!("Auth object: {}", js_user);
            if not(js_user.is_undefined()) {
                match serde_wasm_bindgen::from_value(js_user) {
                    Ok(user) => {
                        model.ctx.user = Some(user);
                        orders.perform_cmd({
                            let message = model.ctx.clone();
                            async { Msg::LoggedIn(register_user(message).await) }
                            
                        }); 
                    },
                    Err(error) => error!("User deserialization failed!", error),
                }
            }

            let search = model.base_url.search_mut();
            if search.remove("code").is_some() && search.remove("state").is_some() {        
                model.base_url.go_and_replace();
            }
        }
        Msg::AuthInitialized(Err(error)) => {
            error!("Auth initialization failed!", error);
        }
        Msg::SignUp => {
            orders.perform_cmd(async { Msg::RedirectingToSignUp(
                redirect_to_sign_up().await
            )});
        },
        Msg::LogIn => {
            orders.perform_cmd(async { Msg::RedirectingToLogIn(
                redirect_to_log_in().await
            )});
        },
        Msg::RedirectingToSignUp(result) => {
            if let Err(error) = result {
                error!("Redirect to sign up failed!", error);
            }
        },
        Msg::RedirectingToLogIn(result) => {
            if let Err(error) = result {
                error!("Redirect to log in failed!", error);
            }
        }
        Msg::LogOut => {
            if let Err(error) = logout() {
                error!("Cannot log out!", error);
            } else {
                model.ctx.user = None;
            }
        },

        // ------ pages ------
        Msg::SettingsMsg(msg) => {
            if let Page::Settings(model) = &mut model.page {
                page::settings::update(msg, model, &mut orders.proxy(Msg::SettingsMsg))
            }
        }
        Msg::LoggedIn(e) => log!("User registration response {}", e),
    }
}


async fn register_user(context: Context) -> fetch::Result<String> {
    let token = getTokenSilently().await.unwrap();
    Request::new("/sma-control/api/register")
        .method(Method::Get)
        .header(Header::bearer(token.as_string().unwrap()))
        .fetch()
        .await?
        .check_status()?
        .text()
        .await
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn init_auth(domain: String, client_id: String, audience: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn redirect_to_sign_up() -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    async fn redirect_to_log_in() -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    fn logout() -> Result<(), JsValue>;

    #[wasm_bindgen(catch)]
    async fn getTokenSilently() -> Result<JsValue, JsValue>;

}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> Vec<Node<Msg>> {
    vec![
        view_navbar(model.menu_visible, &model.base_url, model.ctx.user.as_ref(), &model.page),
        view_content(&model.page, &model.base_url),
    ]
}

// ----- view_content ------

fn view_content(page: &Page, base_url: &Url) -> Node<Msg> {
    div![
        C!["container"],
        match page {
            Page::Home => page::home::view(base_url),
            Page::Settings(model) => page::settings::view(model).map_msg(Msg::SettingsMsg),
            Page::NotFound => page::not_found::view(),
        }
    ]
}

// ----- view_navbar ------

fn view_navbar(menu_visible: bool, base_url: &Url, user: Option<&User>, page: &Page) -> Node<Msg> {
    nav![
        C!["navbar", "is-link"],
        attrs!{
            At::from("role") => "navigation",
            At::AriaLabel => "main navigation",
        },
        view_brand_and_hamburger(menu_visible, base_url),
        view_navbar_menu(menu_visible, base_url, user, page),
    ]
}

fn view_brand_and_hamburger(menu_visible: bool, base_url: &Url) -> Node<Msg> {
    div![
        C!["navbar-brand"],
        // ------ Logo ------
        a![
            C!["navbar-item", "has-text-weight-bold", "is-size-3"],
            attrs!{At::Href => Urls::new(base_url).home()},
            "TT"
        ],
        // ------ Hamburger ------
        a![
            C!["navbar-burger", "burger", IF!(menu_visible => "is-active")],
            style!{
                St::MarginTop => "auto",
                St::MarginBottom => "auto",
            },
            attrs!{
                At::from("role") => "button",
                At::AriaLabel => "menu",
                At::AriaExpanded => menu_visible,
            },
            ev(Ev::Click, |event| {
                event.stop_propagation();
                Msg::ToggleMenu
            }),
            span![attrs!{At::AriaHidden => "true"}],
            span![attrs!{At::AriaHidden => "true"}],
            span![attrs!{At::AriaHidden => "true"}],
        ]
    ]
}

fn view_navbar_menu(menu_visible: bool, base_url: &Url, user: Option<&User>, page: &Page) -> Node<Msg> {
    div![
        C!["navbar-menu", IF!(menu_visible => "is-active")],
        view_navbar_menu_start(base_url, page),
        view_navbar_menu_end(base_url, user),
    ]
}

fn view_navbar_menu_start(base_url: &Url, page: &Page) -> Node<Msg> {
    div![
        C!["navbar-start"],

    ]
}

fn view_navbar_menu_end(base_url: &Url, user: Option<&User>) -> Node<Msg> {
     div![
        C!["navbar-end"],
        div![
            C!["navbar-item"],
            div![
                C!["buttons"],
                if let Some(user) = user {
                    view_buttons_for_logged_in_user(base_url, user)
                } else {
                    view_buttons_for_anonymous_user()
                }
            ]
        ]
    ]
}

fn view_buttons_for_logged_in_user(base_url: &Url, user: &User) -> Vec<Node<Msg>> {
    vec![
        a![
            C!["button", "is-primary"],
            attrs![
                At::Href => Urls::new(base_url).settings(),
            ],
            strong![&user.nickname],
        ],
        a![
            C!["button", "is-light"],
            "Log out",
            ev(Ev::Click, |_| Msg::LogOut),
        ]
    ]
}

fn view_buttons_for_anonymous_user() -> Vec<Node<Msg>> {
    vec![
        a![
            C!["button", "is-primary"],
            strong!["Sign up"],
            ev(Ev::Click, |_| Msg::SignUp),
        ],
        a![
            C!["button", "is-light"],
            "Log in",
            ev(Ev::Click, |_| Msg::LogIn),
        ]
    ]
}

// ------ ------
//     Start
// ------ ------

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
