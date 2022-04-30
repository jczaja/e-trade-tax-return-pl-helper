mod logging;

use chrono;

type ReqwestClient = reqwest::blocking::Client;

pub use logging::ResultExt;

pub struct Transaction {
    pub transaction_date: String,
    pub gross_us: f32,
    pub tax_us: f32,
    pub exchange_rate_date: String,
    pub exchange_rate: f32,
}

pub trait Residency {
    //    fn get_exchange_rate(&self, transaction_date: &str) -> Result<(String, f32), String>;
    fn present_result(&self, gross: f32, tax: f32);
    fn get_exchange_rates(
        &self,
        transactions: Vec<(String, f32, f32)>,
    ) -> Result<Vec<Transaction>, String>;

    // Default parser (not to be used)
    fn parse_exchange_rates(&self, _body: &str) -> Result<(f32, String), String> {
        panic!("This method should not be used. Implement your own if needed!");
    }

    fn get_currency_exchange_rates(
        &self,
        transactions: Vec<(String, f32, f32)>,
        from: &str,
        to: &str,
    ) -> Result<Vec<Transaction>, String> {
        // proxies are taken from env vars: http_proxy and https_proxy
        let http_proxy = std::env::var("http_proxy");
        let https_proxy = std::env::var("https_proxy");

        // If there is proxy then pick first URL
        let base_client = ReqwestClient::builder();
        let client = match &http_proxy {
            Ok(proxy) => base_client
                .proxy(reqwest::Proxy::http(proxy).expect_and_log("Error setting HTTP proxy")),
            Err(_) => base_client,
        };
        let client = match &https_proxy {
            Ok(proxy) => client
                .proxy(reqwest::Proxy::https(proxy).expect_and_log("Error setting HTTP proxy")),
            Err(_) => client,
        };
        let client = client.build().expect_and_log("Could not create client");

        // Example URL: https://www.exchange-rates.org/Rate/USD/EUR/2-27-2021

        let base_exchange_rate_url = "https://www.exchange-rates.org/Rate/";
        let mut detailed_transactions: Vec<Transaction> = Vec::new();

        for (transaction_date, gross_us, tax_us) in transactions {
            let mut converted_date =
                chrono::NaiveDate::parse_from_str(&transaction_date, "%m/%d/%y").unwrap();

            converted_date = converted_date
                .checked_sub_signed(chrono::Duration::days(1))
                .expect_and_log("Error traversing date");

            let exchange_rate_url: String = base_exchange_rate_url.to_string()
                + &format!("{}/{}/{}", from, to, converted_date.format("%m-%d-%Y"))
                + "/?format=json";

            let body = client.get(&(exchange_rate_url)).send();
            let actual_body = body.expect_and_log(&format!(
                "Getting Exchange Rate from Exchange-Rates.org ({}) failed",
                exchange_rate_url
            ));
            if actual_body.status().is_success() {
                log::info!("RESPONSE {:#?}", actual_body);

                let exchange_rates_response = actual_body
                    .text()
                    .expect_and_log("Error converting response to Text");
                log::info!("body of exchange_rate = {:#?}", &exchange_rates_response);
                // parsing text response
                if let Ok((exchange_rate, exchange_rate_date)) =
                    self.parse_exchange_rates(&exchange_rates_response)
                {
                    detailed_transactions.push(Transaction {
                        transaction_date,
                        gross_us,
                        tax_us,
                        exchange_rate_date,
                        exchange_rate,
                    });
                }
            } else {
                return Err("Error getting exchange rate".to_owned());
            }
        }
        Ok(detailed_transactions)
    }
}
