# Automatically add helloassso association events to your calendar

Launches a web service exporting events from associations on [HelloAsso](https://www.helloasso.com) (no token/login needed).
Start the service on port 8080 :
`cargo run`.
You can change the service port by specifying the `HELLOASSO_CALENDAR_PORT` environment variable before launching the service.


## Usage

1. find the associations for which you would like to see their events directly in your calendar (proton calendar, google calendar etc.)
2. Extract their URL slug. For example, for Anticor (https://www.helloasso.com/associations/anticor), the slug is `anticor`.
3. get the calendar at `http://localhost:8080/helloasso/anticor`. It's possible to have several association slugs in the url, just separate them with `+` : `http://localhost:8080/helloasso/anticor+fertiles`: 


