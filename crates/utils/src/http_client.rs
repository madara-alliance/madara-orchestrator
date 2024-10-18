use std::collections::HashMap;

use reqwest::multipart::Form;
use reqwest::{Body, Certificate, Client, ClientBuilder, Identity, Response, Result};
use url::Url;

pub struct HttpClient {
    base_url: Url,
    client: Client,
    default_query_params: HashMap<String, String>,
}

impl HttpClient {
    pub fn builder(base_url: &str) -> HttpClientBuilder {
        HttpClientBuilder {
            base_url: Url::parse(base_url).expect("Invalid base URL"),
            client_builder: Client::builder(),
            default_query_params: HashMap::new(),
        }
    }

    pub fn request(&self) -> RequestBuilder {
        RequestBuilder::new(self)
    }

    async fn send_request<'a>(&self, builder: RequestBuilder<'a>) -> Result<Response> {
        let mut url = self.base_url.join(&builder.path)?;

        for (key, value) in &self.default_query_params {
            url.query_pairs_mut().append_pair(key, value);
        }

        let mut request = self.client.request(builder.method, url);

        if let Some(body) = builder.body {
            request = request.body(body);
        }

        if let Some(form) = builder.form {
            request = request.multipart(form);
        }

        request.send().await
    }
}

pub struct HttpClientBuilder {
    base_url: Url,
    client_builder: ClientBuilder,
    default_query_params: HashMap<String, String>,
}

impl HttpClientBuilder {
    pub fn identity(mut self, identity: Identity) -> Self {
        self.client_builder = self.client_builder.identity(identity);
        self
    }

    pub fn add_root_certificate(mut self, cert: Certificate) -> Self {
        self.client_builder = self.client_builder.add_root_certificate(cert);
        self
    }

    pub fn add_default_query_param(mut self, key: &str, value: &str) -> Self {
        self.default_query_params.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Result<HttpClient> {
        Ok(HttpClient {
            base_url: self.base_url,
            client: self.client_builder.build()?,
            default_query_params: self.default_query_params,
        })
    }
}

pub struct RequestBuilder<'a> {
    client: &'a HttpClient,
    method: reqwest::Method,
    path: String,
    body: Option<Body>,
    form: Option<Form>,
}

impl<'a> RequestBuilder<'a> {
    fn new(client: &'a HttpClient) -> Self {
        Self { client, method: reqwest::Method::GET, path: String::new(), body: None, form: None }
    }

    pub fn method(mut self, method: reqwest::Method) -> Self {
        self.method = method;
        self
    }

    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    pub fn body<T: Into<Body>>(mut self, body: T) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn form(mut self, form: Form) -> Self {
        self.form = Some(form);
        self
    }

    pub async fn send(self) -> Result<Response> {
        self.client.send_request(self).await
    }
}
