use hyper::{Body, Response};

pub(crate) async fn read_body(response: Response<Body>) -> Result<Vec<u8>, hyper::Error> {
    let bytes = hyper::body::to_bytes(response.into_body()).await?;
    Ok(bytes.to_vec())
}

pub(crate) async fn read_utf8_body(response: Response<Body>) -> anyhow::Result<String> {
    let bytes = read_body(response).await?;
    Ok(String::from_utf8(bytes)?)
}
