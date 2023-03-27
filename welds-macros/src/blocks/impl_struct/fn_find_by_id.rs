use crate::column::Column;
use crate::info::Info;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;

pub(crate) fn write(infos: &Info) -> TokenStream {
    let pks = infos.pks.as_slice();
    if pks.len() == 0 {
        return quote!();
    }

    let id_params: Vec<_> = pks.iter().map(|col| id_param(col)).collect();
    let id_params = quote! { #(#id_params),* };

    let typelist = uniq_type_list(&pks);
    let encode_types: Vec<_> = typelist.iter().map(|col| encode_type(col)).collect();
    let encode_types = quote! {#(#encode_types),* };

    let converts: Vec<_> = pks.iter().map(|col| convert(col)).collect();
    let converts = quote! {#(#converts)* };

    let filters: Vec<_> = pks.iter().map(|col| filter(col)).collect();
    let filters = quote! {#(#filters)* };

    quote! {

    pub async fn find_by_id<'a, 'e, 'args, E, DB>(
        exec: E,
        #id_params
    ) -> welds_core::errors::Result<Option<welds_core::state::DbState<Self>>>
    where
        'a: 'args,
        E: sqlx::Executor<'e, Database = DB>,
        DB: sqlx::Database,
        <Self as welds_core::table::HasSchema>::Schema: welds_core::table::TableColumns<DB>,
        <DB as sqlx::database::HasArguments<'a>>::Arguments: sqlx::IntoArguments<'args, DB>,
        Self: Send + Unpin + for<'r> sqlx::FromRow<'r, DB::Row>,
        DB: welds_core::writers::DbLimitSkipWriter,
        DB: welds_core::writers::DbColumnWriter,
        DB: welds_core::query::clause::DbParam,
        #encode_types
    {
        #converts
        let mut q = Self::all();
        #filters
        let mut results = q.limit(1).run(exec).await?;
        Ok(results.pop())
    }

    }
}

fn id_param(col: &Column) -> TokenStream {
    let name = &col.field;
    let ty = &col.field_type;
    quote! { #name: impl Into<#ty> }
}

fn encode_type(id_type: &syn::Type) -> TokenStream {
    quote! { #id_type: sqlx::Encode<'a, DB> + sqlx::Type<DB> }
}

fn filter(col: &Column) -> TokenStream {
    let name = &col.field;
    quote! { q = q.where_col(|x| x.#name.equal(#name)); }
}

fn convert(col: &Column) -> TokenStream {
    let name = &col.field;
    let ty = &col.field_type;
    quote! { let #name: #ty = #name.into(); }
}

fn uniq_type_list(cols: &[Column]) -> Vec<syn::Type> {
    let mut set: HashSet<syn::Type> = Default::default();
    for col in cols {
        set.insert(col.field_type.clone());
    }
    set.into_iter().collect()
}
