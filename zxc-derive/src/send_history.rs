use super::*;

pub fn impl_send_history(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) =
        ast.generics.split_for_impl();
    quote! {
        impl #impl_generics SendHistory for #name #type_generics #where_clause {
            fn get_sender(&self) -> &Sender<CommanderToHistory> {
                self.history_sendr.as_ref().unwrap()
            }
        }
    }
    .into()
}
