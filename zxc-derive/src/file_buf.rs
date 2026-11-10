use super::*;

pub fn impl_file_buf(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) =
        ast.generics.split_for_impl();
    let var_file = format_ident!("file");
    let var_buf = format_ident!("buf");
    quote! {
        impl #impl_generics FileBuf for #name #type_generics #where_clause {
            fn file_and_buf_as_mut(&mut self) -> (&mut File, &mut BytesMut) {
                (&mut self.#var_file, &mut self.#var_buf)
            }
        }
    }
    .into()
}
