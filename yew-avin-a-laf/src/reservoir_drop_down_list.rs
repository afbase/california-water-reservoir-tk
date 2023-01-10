use yew::{function_component, html, Html};

pub enum ReservervoirSelectionEvent {
    // The user selected a reservoir from the dropdown list
    SelectReservoir(String),
}

#[derive(Properties, PartialEq)]
pub struct ReservoirsDropDownProps {
    // see generic_callback
    pub on_change: Fn(Event, &str) -> ReservervoirSelectionEvent),
    pub div_id: String,
    pub select_id: String,
    pub model: CalendarYearModel,
}


#[function_component]
pub fn reservoir_drop_down_list(props: &ReservoirsDropDownProps) -> Html {
    let reservoir_vector props.model.reservoir_vector;
    let mut reservoir_ids_sorted = props.model
                .reservoir_data
                .keys()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>();
    reservoir_ids_sorted.sort();
    html! {
        <div id={props.div_id}>
            // Dropdown list for selecting a reservoir
            <select id={props.select_id} onchange={reservoir_selection_callback}>
            { for
                reservoir_ids_sorted.iter().map(|station_id| {
                    let station_id_value = station_id.clone();
                    let station_id_option = station_id.clone();
                    let reservoir = reservoir_vector.iter().find_map(|resy|
                        {
                            let mut result = None;
                            let reservoir_station_id = resy.station_id.clone();
                            let station_id_cloned = station_id.clone();
                            if reservoir_station_id == station_id_cloned {
                                result = Some(resy.clone());
                            }
                            result
                        }).unwrap();
                    let option_text = format!("{} - {}", reservoir.dam, station_id_option);
                    if *station_id == props.model.selected_reservoir {
                            html!{
                                <option value={station_id_value} selected=true>{option_text}</option>
                            }
                        } else {
                            html!{
                                <option value={station_id_value}>{option_text}</option>
                            }
                        }
                })
            }
            </select>
        </div>
    }
}