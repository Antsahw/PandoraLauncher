use std::sync::Arc;

use bridge::instance::InstanceID;
use bridge::meta::MetadataRequest;
use gpui::{prelude::*, *, Subscription};
use gpui_component::{input::{Input, InputEvent, InputState}, v_flex};
use schema::technic::{TechnicSearchRequest, TechnicSearchResult};

use crate::{
    component::error_alert::ErrorAlert, entity::{
        DataEntities, metadata::{AsMetadataResult, FrontendMetadata, FrontendMetadataResult, FrontendMetadataState}
    }, pages::page::Page, ts
};

pub struct TechnicSearchPage {
    data: DataEntities,
    hits: Vec<schema::technic::TechnicSearchHit>,
    install_for: Option<InstanceID>,
    search_state: Entity<InputState>,
    loading: Option<Entity<FrontendMetadataState>>,
    search_error: Option<SharedString>,
    pending_reload: bool,
    last_search: Arc<str>,
    _search_subscription: Subscription,
}

impl TechnicSearchPage {
    pub fn new(install_for: Option<InstanceID>, data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(ts!("instance.content.search.modpack"))
                .clean_on_escape()
        });

        let _search_subscription = cx.subscribe_in(&search_state, window, Self::on_search_input_event);

        Self {
            data: data.clone(),
            hits: Vec::new(),
            install_for,
            search_state,
            loading: None,
            search_error: None,
            pending_reload: false,
            last_search: Arc::from(""),
            _search_subscription,
        }
    }

    fn on_search_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let InputEvent::Change = event else { return };
        let search = state.read(cx).text().to_string();
        let search: Arc<str> = Arc::from(search.trim());

        if search.is_empty() || &*self.last_search == &*search {
            return;
        }

        self.last_search = search;
        self.pending_reload = true;
        self.search_error = None;
        cx.notify();
    }

    fn load_search(&mut self, cx: &mut Context<Self>) {
        if self.loading.is_some() {
            return;
        }
        self.pending_reload = false;

        let request = TechnicSearchRequest {
            query: if self.last_search.is_empty() {
                None
            } else {
                Some(self.last_search.clone())
            },
            offset: 0,
            limit: 50,
        };

        let data = FrontendMetadata::request(
            &self.data.metadata,
            MetadataRequest::TechnicSearch(request),
            cx,
        );

        let result: FrontendMetadataResult<TechnicSearchResult> = data.read(cx).result();
        match result {
            FrontendMetadataResult::Loading => {
                let _subscription = cx.observe(&data, |page, entity, cx| {
                    let result: FrontendMetadataResult<TechnicSearchResult> = entity.read(cx).result();
                    match result {
                        FrontendMetadataResult::Loading => {},
                        FrontendMetadataResult::Loaded(search_result) => {
                            page.hits = search_result.results.to_vec();
                            page.loading = None;
                            cx.notify();
                        },
                        FrontendMetadataResult::Error(error) => {
                            page.search_error = Some(error);
                            page.loading = None;
                            cx.notify();
                        },
                    }
                    if page.pending_reload {
                        page.load_search(cx);
                    }
                });
                self.loading = Some(data);
            },
            FrontendMetadataResult::Loaded(result) => {
                self.hits = result.results.to_vec();
            },
            FrontendMetadataResult::Error(error) => {
                self.search_error = Some(error);
            },
        }
    }
}

impl Page for TechnicSearchPage {
    fn controls(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Empty
    }

    fn scrollable(&self, _cx: &App) -> bool {
        true
    }
}

impl Render for TechnicSearchPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pending_reload {
            self.load_search(cx);
        }

        let search_input = Input::new(&self.search_state);

        let content = if let Some(error) = &self.search_error {
            div()
                .p_3()
                .child(ErrorAlert::new(ts!("instance.content.searching_error"), error.clone()))
                .into_any_element()
        } else if self.loading.is_some() {
            div()
                .p_3()
                .child("Loading...")
                .into_any_element()
        } else if self.hits.is_empty() {
            div()
                .p_3()
                .child("Search for modpacks to get started")
                .into_any_element()
        } else {
            v_flex()
                .gap_2()
                .children(self.hits.iter().map(|hit| {
                    div()
                        .p_2()
                        .border_b_1()
                        .child(
                            div()
                                .text_base()
                                .child(format!("{}", hit.display_name.as_ref().unwrap_or(&hit.name)))
                        )
                        .when_some(hit.description.as_ref(), |this, desc| {
                            this.child(div().text_sm().child(format!("{}", desc)))
                        })
                        .child(
                            gpui_component::button::Button::new(format!("install-{}", hit.name))
                                .label(ts!("instance.content.install"))
                                .on_click({
                                    let modpack_name = hit.name.clone();
                                    let data = self.data.clone();
                                    let install_for = self.install_for;
                                    move |_, window, cx| {
                                        crate::modals::technic_install::open(
                                            modpack_name.clone(),
                                            install_for,
                                            &data,
                                            window,
                                            cx
                                        );
                                    }
                                })
                        )
                }))
                .into_any_element()
        };

        v_flex()
            .size_full()
            .gap_3()
            .p_3()
            .child(search_input)
            .child(content)
    }
}
