use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use std::{fs::File, io::{Read, Write}};
use reqwest::{Client, Url};
use console::style;
use dotenv::dotenv;
use clap::Parser;

static CONFIG_PATH: &str = ".env";

#[derive(Parser, Debug)]
struct Args {
    /// Source to download the file
    #[arg(short, long, value_name = "config-file")]
    url: Option<String>,

    /// Select a custom config file
    #[arg(short, long, value_name = "config-file")]
    config_file: Option<String>,
}

struct Loading {}

impl Loading {
    fn new(total_size: u64) -> ProgressBar {
        let progress_bar: ProgressBar = match total_size {
            0 => ProgressBar::new_spinner(),
            _ => ProgressBar::new(total_size),
        };

        progress_bar
    }
}

#[tokio::main]
async fn main() {
    let cli_args = Args::parse();
    dotenv().ok();

    let config_path = match cli_args.url {
        Some(url) =>  {
                let response: Result<Vec<u8>, _> = download(&url).await;
                let base_config = response.unwrap();

                let mut config_file = File::create(CONFIG_PATH).expect("Unable to create config file");
                config_file.write(&base_config).expect("Unable to write config file");
        },
        None => (),
    };

}

async fn download(target: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let url = Url::parse(target)?;
    let client = Client::new();

    let response = client
        .get(url)
        .header("User-Agent", "API Request")
        .send()
        .await;

    println!(
        "{} {} {}",
        style("HTTP:").bold().cyan().bright(),
        style("Request sent...").italic().blink(),
        style("it'll take a few seconds, but we will do the heavy lifting you \n")
            .italic()
            .dim()
    );

    let mut response = response?;

    if response.status().is_success() {
        let headers = response.headers().clone();
        let content_size = headers.get("content-length").map(|size| size.clone());

        let content_size = match content_size {
            Some(value) => String::from(value.to_str().unwrap())
                .parse::<u64>()
                .unwrap_or(0),
            None => 0u64,
        };
        let chunk_size = content_size as usize / 99;

        let loading = Loading::new(content_size);

        loading
            .set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta()
            .as_secs_f64())
            .unwrap())
            .progress_chars("#>-"));

        let mut buffer = vec![0; chunk_size];

        while let Some(chunk) = response.chunk().await? {
            let current_pos = chunk.len() as u64;
            loading.inc(current_pos);

            for byte in chunk.bytes() {
                let byte = byte.unwrap();
                buffer.push(byte);
            }
        }

        loading.finish();

        Ok(buffer)
    } else {
        panic!(
            "{} {}",
            style("ERROR:").red().bold(),
            style("Unable to download data...").italic()
        );
    }
}
