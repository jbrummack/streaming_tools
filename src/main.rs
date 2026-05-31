use streaming_tools::Downloader;

#[tokio::main]
async fn main() {
    let jsonl_url = "";
    let download = Downloader::new();
    let mut lines = download.download_lines(jsonl_url).await.unwrap();
    let counter = lines.get_progress();
    let mut amount = 0;
    while let Some(line) = lines.read_line().await.unwrap() {
        amount += 1;
        if amount % 50_000 == 0 {
            println!("{:?}", counter.get_progress())
        }
        let _ = line.len() + 1;
    }
    let protos_url = "";
    let mut protos = download.download_protos(protos_url).await.unwrap();
    let counter = protos.get_progress();
    let mut amount = 0;
    while let Some(line) = protos.test_proto32().await.unwrap() {
        amount += 1;
        if amount % 50_000 == 0 {
            println!("{}", counter.get_progress())
        }
        let _ = line.len() + 1;
    }
}
