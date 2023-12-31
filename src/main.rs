use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::env;
use std::error::Error;
use std::option::Option;
use std::str;

use icalendar::{Calendar, Component, Event, EventLike};

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

#[derive(Debug)]
struct HelloassoEvent {
    title: String,
    description: String,
    url: String,
    base_price: String,
    adress_name: Option<String>,
    adress: Option<String>,
    adress_city: Option<String>,
    start_naive_date: Option<chrono::NaiveDate>,
    end_naive_date: Option<chrono::NaiveDate>,
}

impl HelloassoEvent {
    fn new(li_action: ElementRef) -> HelloassoEvent {
        let selector_title = Selector::parse("a.ActionLink>div.ActionContent>div>h3").unwrap();
        let title = li_action
            .select(&selector_title)
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        let selector_description = Selector::parse("a.ActionLink>div.ActionContent>div>p").unwrap();
        let description = li_action
            .select(&selector_description)
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        let a_with_href = Selector::parse("a.ActionLink").unwrap();
        let href = li_action
            .select(&a_with_href)
            .next()
            .unwrap()
            .attr("href")
            .unwrap()
            .to_string();

        let selector_date =
            Selector::parse("a.ActionLink>div.ActionContent>div>p.Number-Date").unwrap();
        let option_date = li_action.select(&selector_date).next();

        let dates: Vec<String> = if let Some(date_element_ref) = option_date {
            let regex_date = Regex::new(r"\b(\d{2}/\d{2}/\d{4})\b").unwrap();
            let date_string = date_element_ref
                .text()
                .collect::<String>()
                .trim()
                .to_string();
            regex_date
                .find_iter(&date_string)
                .map(|m| m.as_str().to_string())
                .collect()
        } else {
            vec![]
        };

        fn parser_date_helper(date_str: &str) -> chrono::NaiveDate {
            chrono::NaiveDate::parse_from_str(date_str, "%d/%m/%Y").unwrap()
        }

        let (start_date_day, end_date_day) = if dates.is_empty() {
            (None, None)
        } else {
            (
                Some(parser_date_helper(&dates[0])),
                Some(parser_date_helper(&dates[dates.len() - 1])),
            )
        };

        let selector_adress_name = Selector::parse(
            "a.ActionLink>div.ActionDetails>div.ActionDetails--Data>p.Data-AddressName",
        )
        .unwrap();
        let selector_adress = Selector::parse(
            "a.ActionLink>div.ActionDetails>div.ActionDetails--Data>p.Data-Address",
        )
        .unwrap();
        let selector_adress_city =
            Selector::parse("a.ActionLink>div.ActionDetails>div.ActionDetails--Data>p.Data-City")
                .unwrap();

        fn helper_adress(option_element_ref: Option<ElementRef>) -> Option<String> {
            if let Some(element_ref) = option_element_ref {
                Some(element_ref.text().collect::<String>().trim().to_string())
            } else {
                None
            }
        }

        let adress_name = helper_adress(li_action.select(&selector_adress_name).next());
        let adress = helper_adress(li_action.select(&selector_adress).next());
        let adress_city = helper_adress(li_action.select(&selector_adress_city).next());

        let selector_base_price =
            Selector::parse("a.ActionLink>div.ActionContent>div>p.Number-BasePrice").unwrap();
        let base_price = li_action
            .select(&selector_base_price)
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        let helloasso_event = HelloassoEvent {
            title: title,
            description: description,
            url: href,
            base_price: base_price,
            adress_name: adress_name,
            adress: adress,
            adress_city: adress_city,
            start_naive_date: start_date_day,
            end_naive_date: end_date_day,
        };

        helloasso_event
    }

    fn get_calendar_event(&self) -> Option<Event> {
        if self.start_naive_date.is_none() || self.end_naive_date.is_none() {
            return None;
        }

        let vec_location: Vec<String> = vec![&self.adress_name, &self.adress, &self.adress_city]
            .into_iter()
            .map(|a| a.clone().unwrap_or("".to_string()))
            .collect();
        let location = vec_location
            .join(", ")
            .replace("   ", " ")
            .replace("  ", " ");

        let full_description = format!(
            "{}\nPrix : {}\n{}",
            self.description, self.base_price, self.url
        );

        let event = Event::new()
            .summary(&self.title)
            .description(&full_description)
            //.all_day(self.start_naive_date.unwrap())
            .starts(self.start_naive_date.unwrap())
            .ends(self.end_naive_date.unwrap())
            .url(&self.url)
            .location(&location)
            .done();
        Some(event)
    }
}

struct Association {
    slug: String,
}

impl Association {
    async fn get_events(&self) -> Result<Vec<HelloassoEvent>, Box<dyn Error>> {
        let url = format!("https://www.helloasso.com/associations/{}", self.slug);
        let html = reqwest::get(url).await?.text().await?;
        let document = Html::parse_document(&html);
        let selector = Selector::parse("#event>ul.ActionList>li.Action").unwrap();
        let result: Vec<HelloassoEvent> = document
            .select(&selector)
            .map(|li_action| HelloassoEvent::new(li_action))
            .collect();
        return Ok(result);
    }
}

#[get("/helloasso/{associations}")]
async fn get_helloasso_calendar(associations: web::Path<String>) -> impl Responder {
    let slugs: Vec<&str> = associations.split("+").collect();
    let associations: Vec<Association> = slugs
        .iter()
        .map(|slug| Association {
            slug: slug.to_string(),
        })
        .collect();

    let mut calendar_elements_vector: Vec<Event> = Vec::new();

    for association in associations {
        let result_vec_events = association.get_events().await;
        if let Ok(vec_events) = result_vec_events {
            let tmp2: Vec<Event> = vec_events
                .into_iter()
                .map(|helloasso_event| helloasso_event.get_calendar_event())
                .flatten()
                .collect();
            calendar_elements_vector.extend(tmp2);
        } else {
            println!("error with slug {}", association.slug);
        }
    }

    let calendar = calendar_elements_vector
        .into_iter()
        .collect::<Calendar>()
        .name("helloasso")
        .done();

    HttpResponse::Ok()
        .content_type("text/calendar; charset=utf-8")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", "helloasso.ics"),
        ))
        .body(calendar.to_string())
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let port_str = env::var("HELLOASSO_CALENDAR_PORT").unwrap_or("8080".to_string());
    let port = port_str.parse::<u16>().expect(&format!(
        "HELLOASSO_CALENDAR_PORT={} is not a valid port",
        port_str
    ));
    HttpServer::new(|| App::new().service(get_helloasso_calendar))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}
