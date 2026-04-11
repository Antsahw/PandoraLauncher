use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{
    IndexPath, button::{Button, ButtonVariants}, h_flex, select::{Select, SelectEvent, SelectState}, table::{DataTable, TableDelegate, TableState}
};
use strum::IntoEnumIterator;

use crate::{
    component::{server_list::ServerList, named_dropdown::{NamedDropdown, NamedDropdownItem}, responsive_grid::ResponsiveGrid},
    entity::{DataEntities, metadata::FrontendMetadata},
    icon::PandoraIcon, 
    interface_config::{ServersViewMode, InterfaceConfig},
    pages::page::Page,
    ts
};

pub struct ServersPage {
    server_table: Entity<TableState<ServerList>>,
    view_dropdown: Entity<SelectState<NamedDropdown<ServersViewMode>>>,
    metadata: Entity<FrontendMetadata>,
    backend_handle: BackendHandle,
}

impl ServersPage {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let server_table = ServerList::create_table(data, window, cx);
        let view_dropdown = cx.new(|cx| {
            let items = ServersViewMode::iter().map(|view| {
                NamedDropdownItem { name: view.name(), item: view }
            }).collect::<Vec<_>>();
            let current_view = InterfaceConfig::get(cx).servers_view_mode;
            let row = items.iter().position(|v| v.item == current_view).unwrap_or(0);
            let delegate = NamedDropdown::new(items);
            SelectState::new(delegate, Some(IndexPath::new(row)), window, cx)
        });
        cx.subscribe(&view_dropdown, |_, _, event: &SelectEvent<NamedDropdown<ServersViewMode>>, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };
            let view = value.item;
            InterfaceConfig::get_mut(cx).servers_view_mode = view;
        }).detach();

        Self {
            server_table,
            view_dropdown,
            metadata: data.metadata.clone(),
            backend_handle: data.backend_handle.clone(),
        }
    }
}

impl Page for ServersPage {
    fn controls(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let create_server = Button::new("create_server")
            .success()
            .icon(PandoraIcon::Plus)
            .label("Create Server")
            .on_click(cx.listener(|this, _, window, cx| {
                crate::modals::create_server::open_create_server(this.metadata.clone(), this.backend_handle.clone(), window, cx);
            }));

        let select_view = div()
            .child(Select::new(&self.view_dropdown).title_prefix(format!("{}: ", ts!("instance.view"))));

        h_flex().gap_3().child(create_server).child(select_view)
    }

    fn scrollable(&self, cx: &App) -> bool {
        match InterfaceConfig::get(cx).servers_view_mode {
            ServersViewMode::Cards => true,
            ServersViewMode::List => false,
        }
    }
}

impl Render for ServersPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match InterfaceConfig::get(cx).servers_view_mode {
            ServersViewMode::Cards => {
                let cards = self.server_table.update(cx, |table, cx| {
                    let rows = table.delegate().rows_count(cx);
                    (0..rows).map(|i| table.delegate().render_card(i, cx)).collect::<Vec<_>>()
                });

                let size = Size::new(
                    gpui::AvailableSpace::MinContent,
                    gpui::AvailableSpace::MinContent
                );

                div().p_4().child(ResponsiveGrid::new(size).size_full().gap_4().children(cards)).into_any_element()
            },
            ServersViewMode::List => {
                DataTable::new(&self.server_table).bordered(false).into_any_element()
            },
        }
    }
}
