//! The derive behind the settings schema: parsing, validation, D-Bus values and
//! introspection from one Rust definition (`platform.md` Settings 8).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprLit, Fields, Lit, LitStr, Meta, RangeLimits,
    parse_macro_input, spanned::Spanned,
};

enum Kind {
    /// A node whose arguments are the value.
    Leaf,
    /// A node with a block: `keyboard { }`.
    Section,
    /// Several nodes keyed by their argument: `output "id" { }`, into a `BTreeMap`.
    Keyed,
    /// One `app "id" { }` node: an app's preferences (B27).
    App(LitStr),
}

struct FieldSpec {
    ident: syn::Ident,
    ty: syn::Type,
    name: String,
    kind: Kind,
    default: Option<Expr>,
    range: Option<(Expr, Expr)>,
    doc: String,
}

/// A block of settings. Struct attributes: `#[setting(retired = "name")]` ignores a node
/// that is no longer a setting, `#[setting(check = path)]` names a cross-field check run
/// after parsing and before a write. Field attributes: `#[setting(default = expr)]`,
/// `#[setting(range = a..=b)]`, `#[setting(section)]`, `#[setting(keyed)]`,
/// `#[setting(app = "id")]`, `#[setting(name = "kdl-name")]`. Doc comments document the key.
#[proc_macro_derive(Section, attributes(setting))]
pub fn derive_section(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match section(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// A setting with a fixed set of values, written as strings. Variants are kebab-cased;
/// `#[setting(name = "90")]` overrides one.
#[proc_macro_derive(Choice, attributes(setting))]
pub fn derive_choice(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match choice(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn kebab(ident: &str) -> String {
    let mut out = String::new();
    for (i, c) in ident.chars().enumerate() {
        if c == '_' {
            out.push('-');
        } else if c.is_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn doc(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => Some(s.value().trim().to_owned()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn field_spec(field: &syn::Field) -> syn::Result<FieldSpec> {
    let ident = field.ident.clone().expect("named fields");
    let mut spec = FieldSpec {
        name: kebab(&ident.to_string()),
        ident,
        ty: field.ty.clone(),
        kind: Kind::Leaf,
        default: None,
        range: None,
        doc: doc(&field.attrs),
    };
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("setting")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                spec.kind = Kind::Section;
            } else if meta.path.is_ident("keyed") {
                spec.kind = Kind::Keyed;
            } else if meta.path.is_ident("app") {
                spec.kind = Kind::App(meta.value()?.parse()?);
            } else if meta.path.is_ident("name") {
                spec.name = meta.value()?.parse::<LitStr>()?.value();
            } else if meta.path.is_ident("default") {
                spec.default = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("range") {
                match meta.value()?.parse::<Expr>()? {
                    Expr::Range(r) if matches!(r.limits, RangeLimits::Closed(_)) => {
                        let (Some(a), Some(b)) = (r.start, r.end) else {
                            return Err(meta.error("a range needs both ends"));
                        };
                        spec.range = Some((*a, *b));
                    }
                    other => return Err(syn::Error::new(other.span(), "use `a..=b`")),
                }
            } else {
                return Err(meta.error("unknown setting attribute"));
            }
            Ok(())
        })?;
    }
    Ok(spec)
}

fn section(input: &DeriveInput) -> syn::Result<Tokens> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "Section is for structs"));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(input.span(), "Section needs named fields"));
    };
    let specs = fields
        .named
        .iter()
        .map(field_spec)
        .collect::<syn::Result<Vec<_>>>()?;
    let krate = quote!(::isle_settings);

    let mut retired: Vec<String> = Vec::new();
    let mut check: Option<syn::Path> = None;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("setting")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("retired") {
                let lit: syn::LitStr = meta.value()?.parse()?;
                retired.push(lit.value());
            } else if meta.path.is_ident("check") {
                check = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error("unknown section attribute"));
            }
            Ok(())
        })?;
    }
    let check_fn = check.map(|path| {
        quote! {
            fn check(&self) -> ::core::result::Result<(), #krate::CheckError> {
                #path(self)
            }
        }
    });

    let defaults = specs.iter().map(|f| {
        let ident = &f.ident;
        match &f.default {
            Some(expr) => quote!(#ident: #expr),
            None => quote!(#ident: ::core::default::Default::default()),
        }
    });

    let range_check = |f: &FieldSpec, value: Tokens| -> Tokens {
        match &f.range {
            Some((a, b)) => quote! {
                match #krate::Field::number(&#value) {
                    Some(n) if !((#a) as f64 <= n && n <= (#b) as f64) => {
                        Err(format!("must be from {} to {}", #a, #b))
                    }
                    _ => Ok::<(), String>(()),
                }
            },
            None => quote!(Ok::<(), String>(())),
        }
    };

    let keys = specs.iter().map(|f| {
        let ty = &f.ty;
        let node = &f.name;
        let doc = &f.doc;
        match &f.kind {
            Kind::Leaf => {
                let default = match &f.default {
                    Some(expr) => quote!(#krate::Field::to_kdl(&{ let v: #ty = #expr; v })),
                    None => quote!(#krate::Field::to_kdl(&<#ty as ::core::default::Default>::default())),
                };
                let constraint = match &f.range {
                    Some((a, b)) => quote!(format!("{} to {}", #a, #b)),
                    None => quote!(<#ty as #krate::Field>::choices().join(", ")),
                };
                quote! {
                    out.push(#krate::KeyInfo {
                        key: #krate::join(prefix, #node),
                        kind: <#ty as #krate::Field>::KIND,
                        default: #default,
                        doc: #doc.into(),
                        constraint: #constraint,
                    });
                }
            }
            Kind::Section => quote!(<#ty as #krate::Section>::keys(&#krate::join(prefix, #node), out);),
            Kind::Keyed => quote! {
                <<#ty as #krate::KeyedMap>::Value as #krate::Section>::keys(
                    &#krate::join(prefix, &format!("{}[*]", #node)), out);
            },
            Kind::App(id) => quote! {
                <#ty as #krate::Section>::keys(&#krate::join(prefix, &format!("app[{}]", #id)), out);
            },
        }
    });

    let mut parse_arms = Vec::new();
    let mut app_arms = Vec::new();
    for f in &specs {
        let ident = &f.ident;
        let ty = &f.ty;
        let node = &f.name;
        match &f.kind {
            Kind::Leaf => {
                let check = range_check(f, quote!(value));
                parse_arms.push(quote! {
                    #node => {
                        if node.children().is_some() {
                            ctx.error(node.span(), format!("`{}` takes a value, not a block", #node));
                        } else if let Some(value) = <#ty as #krate::Field>::from_node(node, ctx) {
                            match #check {
                                Ok(()) => out.#ident = value,
                                Err(e) => ctx.error(node.span(), format!("`{}` {}", #node, e)),
                            }
                        }
                    }
                });
            }
            Kind::Section => parse_arms.push(quote! {
                #node => {
                    if !node.entries().is_empty() {
                        ctx.error(node.span(), format!("`{}` takes a block, not values", #node));
                    }
                    let empty = #krate::kdl::KdlDocument::new();
                    out.#ident = <#ty as #krate::Section>::parse(node.children().unwrap_or(&empty), ctx);
                }
            }),
            Kind::Keyed => parse_arms.push(quote! {
                #node => {
                    match #krate::node_key(node, ctx) {
                        Some(key) if out.#ident.contains_key(&key) => {
                            ctx.error(node.span(), format!("`{} \"{}\"` is set twice", #node, key));
                        }
                        Some(key) => {
                            let empty = #krate::kdl::KdlDocument::new();
                            let value = #krate::Section::parse(node.children().unwrap_or(&empty), ctx);
                            out.#ident.insert(key, value);
                        }
                        None => {}
                    }
                }
            }),
            Kind::App(id) => app_arms.push(quote! {
                #id => {
                    let empty = #krate::kdl::KdlDocument::new();
                    out.#ident = <#ty as #krate::Section>::parse(node.children().unwrap_or(&empty), ctx);
                }
            }),
        }
    }
    let app_arm = if app_arms.is_empty() {
        quote!()
    } else {
        quote! {
            "app" => {
                if let Some(key) = #krate::node_key(node, ctx) {
                    if !seen_apps.insert(key.clone()) {
                        ctx.error(node.span(), format!("`app \"{}\"` is set twice", key));
                    } else {
                        match key.as_str() {
                            #(#app_arms)*
                            other => ctx.error(node.span(), format!("no app has the id `{}`", other)),
                        }
                    }
                }
            }
        }
    };
    let once_names: Vec<_> = specs
        .iter()
        .filter(|f| matches!(f.kind, Kind::Leaf | Kind::Section))
        .map(|f| f.name.clone())
        .collect();

    let flattens = specs.iter().map(|f| {
        let ident = &f.ident;
        let node = &f.name;
        match &f.kind {
            Kind::Leaf => quote! {
                if !#krate::Field::to_kdl(&self.#ident).is_empty() {
                    out.insert(#krate::join(prefix, #node), #krate::Field::to_value(&self.#ident));
                }
            },
            Kind::Section => quote!(#krate::Section::flatten(&self.#ident, &#krate::join(prefix, #node), out);),
            Kind::Keyed => quote! {
                for (key, value) in self.#ident.iter() {
                    #krate::Section::flatten(value, &#krate::join(prefix, &format!("{}[{}]", #node, key)), out);
                }
            },
            Kind::App(id) => quote! {
                #krate::Section::flatten(&self.#ident, &#krate::join(prefix, &format!("app[{}]", #id)), out);
            },
        }
    });

    let mut set_arms = Vec::new();
    let mut args_arms = Vec::new();
    for f in &specs {
        let ident = &f.ident;
        let ty = &f.ty;
        let node = &f.name;
        match &f.kind {
            Kind::Leaf => {
                let check = range_check(f, quote!(parsed));
                set_arms.push(quote! {
                    [seg] if seg.name == #node && seg.key.is_none() => {
                        let parsed = <#ty as #krate::Field>::from_value(value)?;
                        if !<#ty as #krate::Field>::choices().is_empty()
                            && !<#ty as #krate::Field>::choices().contains(&#krate::Field::to_kdl(&parsed).trim_matches('"'))
                        {
                            return Err(format!("is one of {}", <#ty as #krate::Field>::choices().join(", ")));
                        }
                        #check?;
                        self.#ident = parsed;
                        Ok(())
                    }
                });
                args_arms.push(quote! {
                    [seg] if seg.name == #node && seg.key.is_none() => {
                        Some(#krate::Field::to_kdl(&self.#ident)).filter(|t| !t.is_empty())
                    }
                });
            }
            Kind::Section => {
                set_arms.push(quote! {
                    [seg, rest @ ..] if seg.name == #node && seg.key.is_none() && !rest.is_empty() => {
                        #krate::Section::set(&mut self.#ident, rest, value)
                    }
                });
                args_arms.push(quote! {
                    [seg, rest @ ..] if seg.name == #node && seg.key.is_none() && !rest.is_empty() => {
                        #krate::Section::kdl_args(&self.#ident, rest)
                    }
                });
            }
            Kind::Keyed => {
                set_arms.push(quote! {
                    [seg, rest @ ..] if seg.name == #node && seg.key.is_some() && !rest.is_empty() => {
                        let key = seg.key.clone().unwrap();
                        let mut entry = self.#ident.get(&key).cloned().unwrap_or_default();
                        #krate::Section::set(&mut entry, rest, value)?;
                        self.#ident.insert(key, entry);
                        Ok(())
                    }
                });
                args_arms.push(quote! {
                    [seg, rest @ ..] if seg.name == #node && seg.key.is_some() && !rest.is_empty() => {
                        self.#ident.get(seg.key.as_ref().unwrap()).and_then(|e| #krate::Section::kdl_args(e, rest))
                    }
                });
            }
            Kind::App(id) => {
                set_arms.push(quote! {
                    [seg, rest @ ..] if seg.name == "app" && seg.key.as_deref() == Some(#id) && !rest.is_empty() => {
                        #krate::Section::set(&mut self.#ident, rest, value)
                    }
                });
                args_arms.push(quote! {
                    [seg, rest @ ..] if seg.name == "app" && seg.key.as_deref() == Some(#id) && !rest.is_empty() => {
                        #krate::Section::kdl_args(&self.#ident, rest)
                    }
                });
            }
        }
        let _ = ty;
    }

    Ok(quote! {
        impl ::core::default::Default for #name {
            fn default() -> Self {
                Self { #(#defaults,)* }
            }
        }

        impl #krate::Section for #name {
            fn keys(prefix: &str, out: &mut ::std::vec::Vec<#krate::KeyInfo>) {
                #(#keys)*
            }

            #[allow(unused_mut, unused_variables)]
            fn parse(doc: &#krate::kdl::KdlDocument, ctx: &mut #krate::Ctx) -> Self {
                let mut out = Self::default();
                let mut seen = ::std::collections::HashSet::<&str>::new();
                let mut seen_apps = ::std::collections::HashSet::<String>::new();
                let once: &[&str] = &[#(#once_names),*];
                for node in doc.nodes() {
                    let name = node.name().value();
                    if once.contains(&name) && !seen.insert(name) {
                        ctx.error(node.span(), format!("`{}` is set twice", name));
                        continue;
                    }
                    match name {
                        #(#parse_arms)*
                        #app_arm
                        #(#retired => {})*
                        other => ctx.error(node.name().span(), format!("`{}` is not a setting here", other)),
                    }
                }
                out
            }

            #check_fn

            fn flatten(&self, prefix: &str, out: &mut ::std::collections::BTreeMap<String, #krate::Value>) {
                #(#flattens)*
            }

            #[allow(unreachable_patterns)]
            fn set(&mut self, path: &[#krate::Seg], value: #krate::Value) -> Result<(), String> {
                match path {
                    #(#set_arms)*
                    _ => Err(format!("`{}` is not a setting", #krate::format_path(path))),
                }
            }

            #[allow(unreachable_patterns)]
            fn kdl_args(&self, path: &[#krate::Seg]) -> Option<String> {
                match path {
                    #(#args_arms)*
                    _ => None,
                }
            }
        }
    })
}

fn choice(input: &DeriveInput) -> syn::Result<Tokens> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "Choice is for enums"));
    };
    let mut idents = Vec::new();
    let mut names = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(
                variant.span(),
                "Choice variants carry no data",
            ));
        }
        let mut text = kebab(&variant.ident.to_string());
        for attr in variant
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("setting"))
        {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    text = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else {
                    Err(meta.error("unknown choice attribute"))
                }
            })?;
        }
        idents.push(variant.ident.clone());
        names.push(text);
    }
    let krate = quote!(::isle_settings);
    Ok(quote! {
        impl #name {
            pub const CHOICES: &'static [&'static str] = &[#(#names),*];

            pub fn as_str(&self) -> &'static str {
                match self { #(Self::#idents => #names,)* }
            }

            fn from_name(name: &str) -> Option<Self> {
                match name { #(#names => Some(Self::#idents),)* _ => None }
            }
        }

        impl #krate::Field for #name {
            const KIND: &'static str = "choice";

            fn from_node(node: &#krate::kdl::KdlNode, ctx: &mut #krate::Ctx) -> Option<Self> {
                #krate::choice_from_node(node, ctx, Self::CHOICES).and_then(|s| Self::from_name(&s))
            }

            fn from_value(value: #krate::Value) -> Result<Self, String> {
                match value {
                    #krate::Value::Str(s) => Self::from_name(&s)
                        .ok_or_else(|| format!("is one of {}", Self::CHOICES.join(", "))),
                    #krate::Value::Int(i) => Self::from_name(&i.to_string())
                        .ok_or_else(|| format!("is one of {}", Self::CHOICES.join(", "))),
                    other => Err(format!("is one of {}, not {}", Self::CHOICES.join(", "), other)),
                }
            }

            fn to_value(&self) -> #krate::Value {
                #krate::Value::Str(self.as_str().to_owned())
            }

            fn to_kdl(&self) -> String {
                #krate::quote(self.as_str())
            }

            fn choices() -> &'static [&'static str] {
                Self::CHOICES
            }
        }
    })
}
