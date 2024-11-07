//! A flexible HTTP client builder with support for default parameters and request customization.
//!
//! This module provides a builder pattern implementation for creating and customizing HTTP clients
//! with default configurations. It supports:
//! - Default headers, query parameters, and form data
//! - Client-level TLS certificates and identities
//! - Request-level customization
//! - Body handling (currently optimized for string payloads)
//!
//! # Body Handling
//! The current implementation assumes bodies are UTF-8 string data. When merging default body
//! parameters with request bodies, they are concatenated with '&' as a separator. For binary
//! data or different formats, the implementation would need to be modified.
//!
//! # Examples
//! ```
//! use utils::http_client::HttpClient;
//!
//! let client = HttpClient::builder("https://api.example.com")
//!     .default_header("Authorization", "Bearer token")
//!     .default_query_param("version", "v1")
//!     .default_body_param("tenant=main")
//!     .build()?;
//!
//! let response = client
//!     .request()
//!     .method(Method::POST)
//!     .path("/api/data")
//!     .query_param("id", "123")
//!     .body("name=test")
//!     .send()
//!     .await?;
//! ```

use std::collections::HashMap;
use std::path::Path;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::{Certificate, Client, ClientBuilder, Identity, Method, Response, Result};
use url::Url;

/// Main HTTP client with default configurations.
///
/// Holds the base configuration and default parameters that will be applied
/// to all requests made through this client.
#[derive(Debug)]
pub struct HttpClient {
    /// Base URL for all requests
    base_url: Url,
    /// Configured reqwest client
    client: Client,
    /// Headers to be included in every request
    default_headers: HeaderMap,
    /// Query parameters to be included in every request
    default_query_params: HashMap<String, String>,
    /// Form data to be included in every multipart request
    default_form_data: HashMap<String, String>,
    /// Body parameters to be included in every request body
    /// Note: Currently assumes string data with & as separator
    default_body_params: String,
}

/// Builder for constructing an HttpClient with custom configurations.
#[derive(Debug)]
pub struct HttpClientBuilder {
    base_url: Url,
    client_builder: ClientBuilder,
    default_headers: HeaderMap,
    default_query_params: HashMap<String, String>,
    default_form_data: HashMap<String, String>,
    default_body_params: String,
}

impl HttpClient {
    /// Creates a new builder for constructing an HttpClient.
    ///
    /// # Arguments
    /// * `base_url` - The base URL for all requests made through this client
    ///
    /// # Panics
    /// Panics if the provided base URL is invalid
    pub fn builder(base_url: &str) -> HttpClientBuilder {
        println!("HttpClient builder created with base url: {:?}", base_url);
        HttpClientBuilder::new(Url::parse(base_url).expect("Invalid base URL"))
    }

    /// Creates a new request builder for making HTTP requests.
    ///
    /// # Returns
    /// A RequestBuilder instance that can be used to construct and send an HTTP request
    pub fn request(&self) -> RequestBuilder {
        RequestBuilder::new(self)
    }

    /// Internal method to send a request with all default parameters applied.
    ///
    /// # Arguments
    /// * `builder` - The RequestBuilder containing request-specific configurations
    ///
    /// # Returns
    /// A Result containing either the Response or an error
    async fn send_request(&self, builder: RequestBuilder<'_>) -> Result<Response> {
        println!("sending request with base url: {:?}", self.base_url);
        // Create a new URL by cloning the base URL and appending the path
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .expect("Base URL cannot be a base")
            .extend(builder.path.trim_start_matches('/').split('/'));
        println!("url: {:?}", url);
        // Merge query parameters
        {
            let mut pairs = url.query_pairs_mut();
            let default_params = self.default_query_params.clone();
            for (key, value) in default_params {
                pairs.append_pair(&key, &value);
            }
            for (key, value) in &builder.query_params {
                pairs.append_pair(key, value);
            }
        }

        let mut request = self.client.request(builder.method, url);

        // Merge headers
        let mut final_headers = self.default_headers.clone();
        final_headers.extend(builder.headers);
        request = request.headers(final_headers);

        // Handle body - merge builder body with defaults
        let mut body = String::new();
        if !self.default_body_params.is_empty() {
            body.push_str(&self.default_body_params);
        }
        if let Some(builder_body) = builder.body {
            if !body.is_empty() {
                body.push('&');
            }
            body.push_str(&builder_body);
        }

        if !body.is_empty() {
            request = request.body(body);
        }

        // Handle form data
        if let Some(mut form) = builder.form {
            let default_form: HashMap<String, String> = self.default_form_data.clone();
            for (key, value) in default_form {
                form = form.text(key, value);
            }
            request = request.multipart(form);
        }
        println!("sending request");
        let response = request.send().await;
        println!("response: {:?}", response);
        response
    }
}

impl HttpClientBuilder {
    /// Creates a new HttpClientBuilder with default configurations.
    fn new(base_url: Url) -> Self {
        Self {
            base_url,
            client_builder: Client::builder(),
            default_headers: HeaderMap::new(),
            default_query_params: HashMap::new(),
            default_form_data: HashMap::new(),
            default_body_params: String::new(),
        }
    }

    /// Adds client identity for TLS authentication.
    pub fn identity(mut self, identity: Identity) -> Self {
        self.client_builder = self.client_builder.identity(identity);
        self
    }

    /// Adds a root certificate for TLS verification.
    pub fn add_root_certificate(mut self, cert: Certificate) -> Self {
        self.client_builder = self.client_builder.add_root_certificate(cert);
        self
    }

    /// Adds a default header to be included in all requests.
    pub fn default_header(mut self, key: HeaderName, value: HeaderValue) -> Self {
        self.default_headers.insert(key, value);
        self
    }

    /// Adds a default query parameter to be included in all requests.
    pub fn default_query_param(mut self, key: &str, value: &str) -> Self {
        self.default_query_params.insert(key.to_string(), value.to_string());
        self
    }

    /// Adds default form data to be included in all multipart requests.
    pub fn default_form_data(mut self, key: &str, value: &str) -> Self {
        self.default_form_data.insert(key.to_string(), value.to_string());
        self
    }

    /// Adds a default body parameter to be included in all request bodies.
    /// Parameters are joined with '&' separator.
    pub fn default_body_param(mut self, param: &str) -> Self {
        if !self.default_body_params.is_empty() {
            self.default_body_params.push('&');
        }
        self.default_body_params.push_str(param);
        self
    }

    /// Builds the HttpClient with all configured defaults.
    pub fn build(self) -> Result<HttpClient> {
        Ok(HttpClient {
            base_url: self.base_url,
            client: self.client_builder.build()?,
            default_headers: self.default_headers,
            default_query_params: self.default_query_params,
            default_form_data: self.default_form_data,
            default_body_params: self.default_body_params,
        })
    }
}

/// Builder for constructing individual HTTP requests.
#[derive(Debug)]
pub struct RequestBuilder<'a> {
    client: &'a HttpClient,
    method: Method,
    path: String,
    headers: HeaderMap,
    query_params: HashMap<String, String>,
    /// Request body as a string. For binary data, this would need to be modified.
    body: Option<String>,
    form: Option<Form>,
}

impl<'a> RequestBuilder<'a> {
    fn new(client: &'a HttpClient) -> Self {
        Self {
            client,
            method: Method::GET,
            path: String::new(),
            headers: HeaderMap::new(),
            query_params: HashMap::new(),
            body: None,
            form: None,
        }
    }

    /// Sets the HTTP method for the request.
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Appends a path segment to the existing path.
    /// If the path starts with '/', it will replace the existing path instead of appending.
    pub fn path(mut self, path: &str) -> Self {
        if path.starts_with('/') {
            self.path = path.to_string();
        } else {
            if !self.path.is_empty() && !self.path.ends_with('/') {
                self.path.push('/');
            }
            self.path.push_str(path);
        }
        self
    }

    /// Adds a header to the request.
    pub fn header(mut self, key: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Adds a query parameter to the request.
    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.query_params.insert(key.to_string(), value.to_string());
        self
    }

    /// Sets the request body.
    /// Note: Currently assumes string data. For binary data, this would need to be modified.
    pub fn body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }

    /// Adds a text part to the multipart form.
    pub fn form_text(mut self, key: &str, value: &str) -> Self {
        let form = match self.form.take() {
            Some(existing_form) => existing_form.text(key.to_string(), value.to_string()),
            None => Form::new().text(key.to_string(), value.to_string()),
        };
        self.form = Some(form);
        self
    }

    pub fn form_file(mut self, key: &str, file_path: &Path, file_name: &str) -> Self {
        let file_bytes = std::fs::read(file_path).expect("Failed to read file");
        // Convert file_name to owned String
        let file_name = file_name.to_string();

        let part = Part::bytes(file_bytes).file_name(file_name);

        let form = match self.form.take() {
            Some(existing_form) => existing_form.part(key.to_string(), part),
            None => Form::new().part(key.to_string(), part),
        };
        self.form = Some(form);
        self
    }

    /// Adds a file part to the multipart form.
    // pub fn form_file(mut self, key: &str, file: std::fs::File) -> Self {
    //     let form = match self.form.take() {
    //         Some(existing_form) => existing_form.file(key.to_string(), file),
    //         None => Form::new().file(key.to_string(), file),
    //     };
    //     self.form = Some(form);
    //     self
    // }

    /// Sends the request with all configured parameters.
    pub async fn send(self) -> Result<Response> {
        self.client.send_request(self).await
    }
}
