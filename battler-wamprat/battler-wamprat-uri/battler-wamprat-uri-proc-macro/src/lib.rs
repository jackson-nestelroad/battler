extern crate proc_macro;

use std::collections::BTreeSet;

use battler_wamp::core::uri::Uri;
use itertools::Itertools;
use proc_macro2::{
    Span,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
    quote_spanned,
};
use regex::Regex;
use syn::{
    Error,
    Field,
    Ident,
    Index,
    ItemStruct,
    LitStr,
    Member,
    Result,
    Type,
    ext::IdentExt,
    parse::{
        Parse,
        ParseStream,
        Parser,
    },
    parse_macro_input,
};

struct InputFieldAttrs {
    rest: bool,
}

fn parse_input_field_attrs(field: &Field) -> Result<InputFieldAttrs> {
    let mut rest = false;
    for attr in &field.attrs {
        if attr.path().is_ident("rest") {
            attr.meta.require_path_only()?.require_ident()?;
            rest = true;
        }
    }
    Ok(InputFieldAttrs { rest })
}

struct InputField {
    member: Member,
    ty: Type,
    attrs: InputFieldAttrs,
}

struct UriAttr {
    fmt: LitStr,
    args: TokenStream,
    match_fields: Vec<usize>,
}

impl UriAttr {
    fn new(span: Span, fmt: LitStr, fields: &[InputField]) -> Result<Self> {
        let mut attr = Self {
            fmt,
            args: TokenStream::new(),
            match_fields: Vec::new(),
        };
        attr.extract_fields(fields)?;
        attr.validate_all_fields_matched(span, fields)?;
        Ok(attr)
    }

    fn validate_all_fields_matched(&self, span: Span, fields: &[InputField]) -> Result<()> {
        let matched_fields = self.match_fields.iter().cloned().collect::<BTreeSet<_>>();
        let unmatched = (0..fields.len())
            .collect::<BTreeSet<_>>()
            .difference(&matched_fields)
            .cloned()
            .collect::<Vec<_>>();
        if !unmatched.is_empty() {
            return Err(Error::new(
                span,
                format!(
                    "uri format string is missing matches for {}",
                    unmatched
                        .iter()
                        .map(|i| {
                            // SAFETY: Indices stored in match_fields were generated from positions
                            // of input.fields.
                            match &fields.get(*i).unwrap().member {
                                Member::Unnamed(index) => index.index.to_string(),
                                Member::Named(ident) => ident.to_string(),
                            }
                        })
                        .join(", ")
                ),
            ));
        }
        Ok(())
    }

    fn extract_fields(&mut self, fields: &[InputField]) -> Result<()> {
        let span = self.fmt.span();
        let fmt = self.fmt.value();
        let mut read = fmt.as_str();
        let mut out = String::new();

        while let Some(brace) = read.find('{') {
            out += &read[..brace + 1];
            read = &read[brace + 1..];

            // Escaping.
            if read.starts_with('{') {
                out.push('{');
                read = &read[1..];
                continue;
            }

            // Parse out the identifier in the format string.
            let next = match read.chars().next() {
                Some(next) => next,
                None => return Err(Error::new(span, "unexpected end of format string")),
            };
            let member = match next {
                '0'..='9' => {
                    let index = take_integer_from_string(&mut read);
                    match index.parse::<u32>() {
                        Ok(index) => Member::Unnamed(Index { index, span }),
                        Err(_) => {
                            return Err(Error::new(
                                span,
                                format!(
                                    "format identifier {index} was expected to parse as an integer"
                                ),
                            ));
                        }
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut ident = take_ident_from_string(&mut read);
                    ident.set_span(span);
                    Member::Named(ident)
                }
                _ => {
                    return Err(Error::new(
                        span,
                        format!("unexpected start of a formatting identifier: {next}"),
                    ));
                }
            };

            // Find the field the identifier corresponds to.
            //
            // Each identifier MUST correspond to a field, since we use this format string for
            // parsing.
            let (i, field) = fields
                .iter()
                .find_position(|field| field.member == member)
                .ok_or_else(|| {
                    Error::new(
                        span,
                        format!(
                            "struct does not have any member \"{}\"",
                            match member {
                                Member::Unnamed(index) => index.index.to_string(),
                                Member::Named(ident) => ident.to_string(),
                            }
                        ),
                    )
                })?;

            // Remember the order in which fields should be matched.
            self.match_fields.push(i);

            // Add the local variable to the format arguments.
            let local = match &field.member {
                Member::Unnamed(index) => format_ident!("_{}", index),
                Member::Named(ident) => ident.clone(),
            };
            self.args.extend(quote_spanned!(span => ,));
            if field.attrs.rest {
                self.args.extend(quote_spanned!(span => #local.join(".")));
            } else {
                self.args.extend(quote_spanned!(span => #local));
            }
        }

        out += read;
        self.fmt = LitStr::new(&out, self.fmt.span());
        Ok(())
    }
}

fn take_integer_from_string(read: &mut &str) -> String {
    let mut int = String::new();
    for (i, ch) in read.char_indices() {
        match ch {
            '0'..='9' => int.push(ch),
            _ => {
                *read = &read[i..];
                break;
            }
        }
    }
    int
}

fn take_ident_from_string(read: &mut &str) -> Ident {
    let mut ident = String::new();
    let raw = read.starts_with("r#");
    if raw {
        ident.push_str("r#");
        *read = &read[2..];
    }
    for (i, ch) in read.char_indices() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => ident.push(ch),
            _ => {
                *read = &read[i..];
                break;
            }
        }
    }

    // SAFETY: We only took characters that are valid for an identifier above.
    Ident::parse_any.parse_str(&ident).unwrap()
}

struct InputAttrs {
    uri: UriAttr,
}

struct Input {
    ident: Ident,
    attrs: InputAttrs,
    fields: Vec<InputField>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let call_site = Span::call_site();
        let input = match ItemStruct::parse(input) {
            Ok(item) => item,
            Err(_) => return Err(Error::new(call_site, "input must be a struct")),
        };
        let ident = input.ident;
        let fields = input
            .fields
            .into_iter()
            .enumerate()
            .map(|(i, field)| {
                let attrs = parse_input_field_attrs(&field)?;
                Ok(InputField {
                    member: field.ident.map(Member::Named).unwrap_or_else(|| {
                        Member::Unnamed(Index {
                            index: i as u32,
                            span: call_site,
                        })
                    }),
                    ty: field.ty,
                    attrs,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut rest = false;
        for field in &fields {
            if rest {
                return Err(Error::new(
                    call_site,
                    "no fields allowed after the rest field",
                ));
            }
            rest = field.attrs.rest;
        }

        let mut uri = None;
        for attr in input.attrs {
            if attr.path().is_ident("uri") {
                if uri.is_some() {
                    return Err(Error::new(call_site, "only one \"uri\" attribute allowed"));
                }
                attr.parse_args_with(|input: ParseStream| {
                    let fmt = input.parse::<LitStr>()?;
                    uri = Some(UriAttr::new(call_site, fmt, &fields)?);
                    Ok(())
                })?;
            }
        }

        let uri = uri.ok_or_else(|| Error::new(call_site, "missing required \"uri\" attribute"))?;
        let attrs = InputAttrs { uri };
        Ok(Self {
            ident,
            attrs,
            fields,
        })
    }
}

/// Procedural macro for deriving `battler_wamprat_uri::WampUriMatcher` for a struct.
#[proc_macro_derive(WampUriMatcher, attributes(uri, rest))]
pub fn derive_wamp_uri_matcher(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Input);
    let call_site = Span::call_site();

    let ident = input.ident;
    let uri_pattern = &input.attrs.uri.fmt;
    let uri_pattern_args = &input.attrs.uri.args;

    let format_pattern = Regex::new(r"\{\}").unwrap();

    // Validate that the base pattern gives us a valid URI.
    let uri_sample = uri_pattern.value();
    let uri_sample = format_pattern.replace_all(&uri_sample, "foo");
    if Uri::try_from(uri_sample.into_owned()).is_err() {
        return proc_macro::TokenStream::from(
            Error::new(call_site, "invalid uri").into_compile_error(),
        );
    }

    enum Matcher {
        Static(LitStr),
        Simple(String),
        Dynamic(String),
    }

    let uri_components = uri_pattern
        .value()
        .split('.')
        .map(|str| str.to_owned())
        .collect::<Vec<_>>();

    // Generate the match style and registration URI.
    let (match_style, uri_for_router) = match (|| {
        if input.attrs.uri.match_fields.is_empty() {
            return Ok(("::core::option::Option::None", input.attrs.uri.fmt.value()));
        }
        if input.fields.iter().any(|field| field.attrs.rest) {
            let uri = uri_pattern.value();
            let uri = format_pattern.replace_all(&uri, "");
            let prefix_pattern = Regex::new(r"^((?:[^\.]+\.)*[^\.]+)\.+$").unwrap();
            if let Some(captures) = prefix_pattern.captures(&uri) {
                // SAFETY: This pattern has one capture group.
                return Ok((
                    "::core::option::Option::Some(::battler_wamp::core::match_style::MatchStyle::Prefix)",
                    captures.get(1).unwrap().as_str().to_owned(),
                ));
            } else {
                return Err(Error::new(
                    call_site,
                    "rest field does not make sense non-prefix uri",
                ));
            }
        }

        Ok((
            "::core::option::Option::Some(::battler_wamp::core::match_style::MatchStyle::Wildcard)",
            uri_components
                .iter()
                .map(|uri_component| {
                    if format_pattern.is_match(uri_component) {
                        ""
                    } else {
                        uri_component
                    }
                })
                .join("."),
        ))
    })() {
        Ok(result) => result,
        Err(err) => return proc_macro::TokenStream::from(err.into_compile_error()),
    };

    let match_style = syn::parse_str::<TokenStream>(&match_style).unwrap();
    let uri_for_router = LitStr::new(&uri_for_router, call_site);

    // Constructing the type from all fields.
    let mut members = input.fields.iter().map(|field| &field.member).peekable();
    let constructor_fields = match members.peek() {
        Some(Member::Named(_)) => quote!( { #(#members),* }),
        Some(Member::Unnamed(_)) => {
            let vars = members.map(|member| match member {
                Member::Unnamed(index) => format_ident!("_{}", index),
                _ => unreachable!(),
            });
            quote!((#(#vars),*))
        }
        None => quote!({}),
    };

    // Generate matchers for each field.
    let mut matchers = uri_components
        .iter()
        .map(|uri_component| {
            let matches = format_pattern.find_iter(uri_component).collect::<Vec<_>>();

            // No matches, so we just need to match the static string.
            if matches.is_empty() {
                return Matcher::Static(LitStr::new(&uri_component, call_site));
            }
            let pattern = format_pattern.replace_all(&uri_component, "([^\\.]+)");
            if pattern == "([^\\.]+)" && matches.len() == 1 {
                // If we are only matching exactly one member, we can optimize this to just assign
                // the string value directly.
                return Matcher::Simple(pattern.into_owned());
            }

            // Otherwise, we must match a regular expression and assign to multiple members.
            Matcher::Dynamic(pattern.into_owned())
        })
        .collect::<Vec<_>>();

    let requires_regex = matchers.iter().any(|matcher| match matcher {
        Matcher::Dynamic { .. } => true,
        _ => false,
    });

    // If the last field is marked "rest," its pattern should be adjusted.
    if let Some(field) = input.fields.last() {
        if field.attrs.rest {
            let pattern = "(.+)".to_owned();
            *matchers.last_mut().unwrap() = if requires_regex {
                Matcher::Dynamic(pattern)
            } else {
                Matcher::Simple(pattern)
            };
        }
    }

    let generator = if input.attrs.uri.match_fields.is_empty() {
        // No fields to match, so we assume we can construct the type directly.
        quote! {
            if uri != #uri_pattern {
                return ::core::result::Result::Err(
                    ::battler_wamprat_uri::WampUriMatchError::new("uri does not match the static pattern")
                );
            }
        }
    } else if !requires_regex {
        let mut parsed = BTreeSet::new();
        let mut match_index = 0;
        let matchers = matchers.into_iter().enumerate().map(|(i, matcher)| match matcher {
            Matcher::Static(component) => {
                let error = LitStr::new(&format!("expected {} for component {i}", component.value()), call_site);
                quote! {
                    uri_components.get(#i).and_then(|uri_component| if uri_component == &#component { Some(uri_component) } else { None }).ok_or_else(|| ::battler_wamprat_uri::WampUriMatchError::new(#error))?;
                }
            }
            Matcher::Simple(_) => {
                let next_match_index = match_index;
                match_index += 1;

                // SAFETY: If every field needs one match, match_index will still be a valid index into match_fields.
                let field_index = *input.attrs.uri.match_fields.get(next_match_index).unwrap();
                // SAFETY: Indices stored in match_fields were generated from positions of input.fields.
                let field = input.fields.get(field_index).unwrap();

                let ty = &field.ty;
                let field_name = match &field.member {
                    Member::Unnamed(index) => index.index.to_string(),
                    Member::Named(ident) => ident.to_string(),
                };
                let error = LitStr::new(&format!("missing component for {field_name}"), call_site);
                let parse_error = LitStr::new(&format!("invalid component for {field_name}"), call_site);

                let local = match &field.member {
                    Member::Unnamed(index) => format_ident!("_{}", index),
                    Member::Named(ident) => ident.clone(),
                };

                if field.attrs.rest {
                    quote! {
                        let #local = uri_components[#i..].iter().map(|uri_component| uri_component.to_string()).collect();
                    }
                } else if parsed.insert(field_index) {
                    quote! {
                        let #local = uri_components.get(#i).ok_or_else(|| ::battler_wamprat_uri::WampUriMatchError::new(#error))?;
                        let #local: #ty = ::core::str::FromStr::from_str(*#local).map_err(|err| ::battler_wamprat_uri::WampUriMatchError::new(#parse_error))?;
                    }
                } else {
                    // Not the first time we are seeing this value. We need to compare it against the original.
                    let inconsistent_error = LitStr::new(&format!("inconsistent value for {field_name} in component {i}"), call_site);
                    let local_copy = format_ident!("{local}_copy");
                    quote! {
                        let #local_copy = uri_components.get(#i).ok_or_else(|| ::battler_wamprat_uri::WampUriMatchError::new(#error))?;
                        let #local_copy: #ty = ::core::str::FromStr::from_str(*#local_copy).map_err(|err| ::battler_wamprat_uri::WampUriMatchError::new(#parse_error))?;
                        if #local != #local_copy {
                            return ::core::result::Result::Err(::battler_wamprat_uri::WampUriMatchError::new(#inconsistent_error));
                        }
                    }
                }
            }
            Matcher::Dynamic(_) => unreachable!(),
        }).collect::<Vec<_>>();
        quote! {
            let uri_components = uri.split('.').collect::<Vec<_>>();
            #(#matchers)*
        }
    } else {
        // Compile the URI into a regular expression and match all fields in order.
        let pattern = matchers
            .into_iter()
            .map(|matcher| match matcher {
                Matcher::Static(component) => component.value(),
                Matcher::Simple(pattern) | Matcher::Dynamic(pattern) => pattern,
            })
            .join("\\.");
        let pattern = format!("^{pattern}$");
        let pattern = match Regex::new(&pattern).map_err(|err| {
            Error::new(
                call_site,
                format!("failed to compile regular expression for matching uri: {err}"),
            )
        }) {
            Ok(pattern) => pattern,
            Err(err) => return proc_macro::TokenStream::from(err.into_compile_error()),
        };
        let pattern_literal = LitStr::new(pattern.as_str(), call_site);

        let mut parsed = BTreeSet::new();

        let matchers = input.attrs.uri.match_fields.iter().enumerate().map(|(i, field_index)| (i + 1, field_index)).map(|(i, field_index)| {
            // SAFETY: Indices stored in match_fields were generated from positions of input.fields.
            let field = input.fields.get(*field_index).unwrap();
            let ty = &field.ty;

            let field_name = match &field.member {
                Member::Unnamed(index) => index.index.to_string(),
                Member::Named(ident) => ident.to_string(),
            };

            let error = LitStr::new(&format!("missing component for {field_name}"), call_site);
            let parse_error = LitStr::new(&format!("invalid component for {field_name}"), call_site);

            let local = match &field.member {
                Member::Unnamed(index) => format_ident!("_{}", index),
                Member::Named(ident) => ident.clone(),
            };

            if parsed.insert(field_index) {
                quote! {
                    let #local = captures.get(#i).ok_or_else(|| ::battler_wamprat_uri::WampUriMatchError::new(#error))?.as_str();
                    let #local: #ty = ::core::str::FromStr::from_str(#local).map_err(|err| ::battler_wamprat_uri::WampUriMatchError::new(#parse_error))?;
                }
            } else {
                  // Not the first time we are seeing this value. We need to compare it against the original.
                  let inconsistent_error = LitStr::new(&format!("inconsistent value for {field_name} in component {i}"), call_site);
                  let local_copy = format_ident!("{local}_copy");
                  quote! {
                    let #local_copy = captures.get(#i).ok_or_else(|| ::battler_wamprat_uri::WampUriMatchError::new(#error))?.as_str();
                    let #local_copy: #ty = ::core::str::FromStr::from_str(#local_copy).map_err(|err| ::battler_wamprat_uri::WampUriMatchError::new(#parse_error))?;
                      if #local != #local_copy {
                          return ::core::result::Result::Err(::battler_wamprat_uri::WampUriMatchError::new(#inconsistent_error));
                      }
                  }
            }
        }).collect::<Vec<_>>();

        quote! {
            // SAFETY: Pattern was validated at build time.
            static RE: ::std::sync::LazyLock<::regex::Regex> = ::std::sync::LazyLock::new(|| ::regex::Regex::new(#pattern_literal).unwrap());
            let captures = RE.captures(uri).ok_or_else(|| ::battler_wamprat_uri::WampUriMatchError::new("uri does not match the configured pattern"))?;
            #(#matchers)*
        }
    };

    quote! {
        impl ::battler_wamprat_uri::WampUriMatcher for #ident {
            fn uri_for_router() -> ::battler_wamp::core::uri::WildcardUri {
                ::battler_wamp::core::uri::WildcardUri::try_from(#uri_for_router).unwrap()
            }

            fn match_style() -> ::core::option::Option<::battler_wamp::core::match_style::MatchStyle> {
                #match_style
            }

            fn wamp_match_uri(uri: &str) -> ::core::result::Result<Self, ::battler_wamprat_uri::WampUriMatchError> {
                #generator
                ::core::result::Result::Ok(Self #constructor_fields)
            }

            fn wamp_generate_uri(&self) -> ::core::result::Result<::battler_wamp::core::uri::Uri, ::battler_wamp::core::uri::InvalidUri> {
                ::battler_wamp::core::uri::Uri::try_from(self.to_string().as_str())
            }
        }

        impl ::core::fmt::Display for #ident {
            fn fmt(&self, __formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #[allow(unused_variables, deprecated)]
                let Self #constructor_fields = self;
                ::core::write!(__formatter, #uri_pattern #uri_pattern_args)
            }
        }
    }.into()
}
