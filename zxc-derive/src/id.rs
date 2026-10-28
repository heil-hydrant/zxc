use super::*;

pub fn impl_id(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) =
        ast.generics.split_for_impl();
    quote! {
        impl #impl_generics Id for #name #type_generics #where_clause {
            fn id(&self) -> crate::id::SoldierId {
                self.id.into()
            }
        }
    }
    .into()
}
