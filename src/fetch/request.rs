use std::borrow::Cow;
use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::config::{HttpMethod, RequestConfig};
use crate::error::{AppError, Context};

use super::FetchResult;

#[derive(Debug, Clone)]
pub struct PreparedRequest {
    pub url: String,
    pub headers: HeaderMap,
}

pub struct RequestContext<'a> {
    pub stock_code: &'a str,
    pub region_code: &'a str,
    pub extras: &'a [(&'a str, Cow<'a, str>)],
}

pub fn prepare_request(
    request: &RequestConfig,
    context: RequestContext<'_>,
) -> FetchResult<PreparedRequest> {
    if !matches!(request.method, HttpMethod::Get) {
        return Err(AppError::message("Unsupported HTTP method"));
    }

    let transformed_code = request.code_transform.apply(context.stock_code);

    let mut replacements: HashMap<String, String> = HashMap::new();
    replacements.insert("code".to_string(), transformed_code.clone());
    replacements.insert("symbol".to_string(), transformed_code.clone());
    replacements.insert("raw_code".to_string(), context.stock_code.to_string());
    replacements.insert("region".to_string(), context.region_code.to_string());
    replacements.insert(
        "region_lower".to_string(),
        context.region_code.to_lowercase(),
    );

    for (key, value) in context.extras {
        replacements.insert((*key).to_string(), value.clone().into_owned());
    }

    let url = render_template(&request.url_template, &replacements)?;
    let headers = build_headers(&request.headers)?;

    Ok(PreparedRequest { url, headers })
}

pub fn expand_env_vars(value: &str) -> FetchResult<String> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && matches!(chars.peek(), Some('{')) {
            chars.next();
            let mut name = String::new();
            let mut closed = false;
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '}' {
                    closed = true;
                    break;
                }
                name.push(next);
            }

            if name.is_empty() {
                return Err(AppError::message(
                    "Encountered empty environment placeholder in header",
                ));
            }

            if !closed {
                return Err(AppError::message(
                    "Unterminated environment placeholder in header",
                ));
            }

            let value = std::env::var(&name).with_context(|| {
                format!(
                    "Environment variable {} required by request header is not set",
                    name
                )
            })?;
            result.push_str(&value);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn render_template(template: &str, replacements: &HashMap<String, String>) -> FetchResult<String> {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut key = String::new();
            let mut closed = false;
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '}' {
                    closed = true;
                    break;
                }
                key.push(next);
            }

            if !closed {
                return Err(AppError::message(format!(
                    "Unterminated placeholder in template: {{{key}"
                )));
            }

            if key.is_empty() {
                return Err(AppError::message(
                    "Encountered empty placeholder `{}` in template",
                ));
            }

            let value = replacements.get(&key).ok_or_else(|| {
                AppError::message(format!(
                    "No replacement provided for placeholder `{}` in template",
                    key
                ))
            })?;
            result.push_str(value);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn build_headers(headers: &HashMap<String, String>) -> FetchResult<HeaderMap> {
    let mut map = HeaderMap::new();
    for (key, value) in headers {
        let name = HeaderName::from_bytes(key.as_bytes())
            .with_context(|| format!("Invalid header name: {}", key))?;
        let expanded = expand_env_vars(value)?;
        let header_value = HeaderValue::from_str(&expanded)
            .with_context(|| format!("Invalid header value for {}", key))?;
        map.insert(name, header_value);
    }
    Ok(map)
}
