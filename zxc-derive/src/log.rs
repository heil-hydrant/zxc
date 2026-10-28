use super::*;

pub fn impl_log(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) =
        ast.generics.split_for_impl();
    quote! {
        impl #impl_generics Log for #name #type_generics #where_clause {
            fn path(&self) -> &PathBuf {
                self.path.as_ref().unwrap()
            }

            fn log_data(&self) -> &[u8] {
                self.msg_bytes.as_ref().unwrap()
            }
        }
    }
    .into()
}
