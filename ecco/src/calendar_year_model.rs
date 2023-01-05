use cdec::{
    reservoir::Reservoir,
    water_year::WaterYear,
};
use plotters::prelude::*;
use std::collections::HashMap;

pub struct CalendarYearModel {
    // The selected reservoir
    pub selected_reservoir: String,
    // The data for the selected reservoir
    pub reservoir_data: HashMap<String, Vec<WaterYear>>,
    pub reservoir_vector: Vec<Reservoir>,
}

pub fn get_colors(number_of_colors: usize) -> Result<Vec<RGBColor>, String> {
    let vec_of_colors = vec![
        // Oranges - 9
        RGBColor(102, 37, 6),
        RGBColor(153, 52, 4),
        RGBColor(204, 76, 2),
        RGBColor(236, 112, 20),
        RGBColor(254, 153, 41),
        RGBColor(254, 196, 79),
        RGBColor(254, 227, 145),
        RGBColor(255, 247, 188),
        RGBColor(255, 255, 229),
        //PuBuGn - 9
        RGBColor(1, 70, 54),
        RGBColor(1, 108, 89),
        RGBColor(2, 129, 138),
        RGBColor(54, 144, 192),
        RGBColor(103, 169, 207),
        RGBColor(166, 189, 219),
        RGBColor(208, 209, 230),
        RGBColor(236, 226, 240),
        RGBColor(255, 247, 251),
        //Accent - 2
        RGBColor(127, 201, 127),
        RGBColor(190, 174, 212),
    ]; // total of 20
    let vec_len = vec_of_colors.len();
    if number_of_colors <= vec_len {
        let slice = vec_of_colors.as_slice();
        let result_slice = &slice[0..number_of_colors];
        return Ok(result_slice.to_vec());
    }
    return Err(String::from("too many colors requested"));
}
