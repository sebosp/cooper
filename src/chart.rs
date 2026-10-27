//! A line chart of the players over the game, drawn on a canvas with plotters.

use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use web_sys::HtmlCanvasElement;
use yew::prelude::*;

/// A line, i.e. the workers of a player, in the color of the player.
#[derive(Clone, PartialEq)]
pub struct ChartSeries {
    pub name: String,
    pub color: (u8, u8, u8),
    /// The game seconds and the values.
    pub points: Vec<(u32, f64)>,
}

#[derive(Properties, PartialEq)]
pub struct LineChartProps {
    pub title: AttrValue,
    pub series: Vec<ChartSeries>,
}

const WIDTH: u32 = 640;
const HEIGHT: u32 = 300;

fn draw(canvas: HtmlCanvasElement, series: &[ChartSeries]) -> Result<(), String> {
    canvas.set_width(WIDTH);
    canvas.set_height(HEIGHT);
    let backend = CanvasBackend::with_canvas_object(canvas).ok_or("No canvas")?;
    let area = backend.into_drawing_area();
    area.fill(&RGBColor(33, 37, 41))
        .map_err(|err| err.to_string())?;
    let max_minutes = series
        .iter()
        .flat_map(|series| {
            series
                .points
                .iter()
                .map(|(seconds, _)| *seconds as f64 / 60.)
        })
        .fold(1., f64::max);
    let max_value = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(_, value)| *value))
        .fold(1., f64::max);
    let text = RGBColor(173, 181, 189);
    let mut chart = ChartBuilder::on(&area)
        .margin(12)
        .x_label_area_size(32)
        .y_label_area_size(48)
        .build_cartesian_2d(0f64..max_minutes, 0f64..max_value * 1.05)
        .map_err(|err| err.to_string())?;
    chart
        .configure_mesh()
        .x_desc("Game minutes")
        .x_label_formatter(&|minutes| format!("{minutes:.0}"))
        .y_label_formatter(&|value| format!("{value:.0}"))
        .axis_desc_style(("sans-serif", 13).into_font().color(&text))
        .label_style(("sans-serif", 12).into_font().color(&text))
        .axis_style(text.mix(0.5))
        .bold_line_style(text.mix(0.15))
        .light_line_style(TRANSPARENT)
        .draw()
        .map_err(|err| err.to_string())?;
    for line in series {
        let color = RGBColor(line.color.0, line.color.1, line.color.2);
        chart
            .draw_series(LineSeries::new(
                line.points
                    .iter()
                    .map(|(seconds, value)| (*seconds as f64 / 60., *value)),
                color.stroke_width(2),
            ))
            .map_err(|err| err.to_string())?
            .label(line.name.clone())
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 16, y)], color.stroke_width(2))
            });
    }
    if series.len() > 1 {
        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperLeft)
            .background_style(RGBColor(33, 37, 41).mix(0.85))
            .border_style(text.mix(0.3))
            .label_font(("sans-serif", 12).into_font().color(&text))
            .draw()
            .map_err(|err| err.to_string())?;
    }
    area.present().map_err(|err| err.to_string())
}

/// The chart is drawn again when its series change.
#[function_component(LineChart)]
pub fn line_chart(props: &LineChartProps) -> Html {
    let canvas = use_node_ref();
    {
        let canvas = canvas.clone();
        use_effect_with(props.series.clone(), move |series| {
            if let Some(element) = canvas.cast::<HtmlCanvasElement>()
                && let Err(err) = draw(element, series)
            {
                gloo::console::error!(format!("The chart can not be drawn: {err}"));
            }
        });
    }
    html! {
        <div class="col-12 col-xl-6 mb-3">
            <h3 class="h6 text-body-secondary">{ props.title.clone() }</h3>
            <canvas ref={canvas} style="width: 100%; max-width: 640px; aspect-ratio: 640 / 300;" />
        </div>
    }
}
