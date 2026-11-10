use super::*;

pub fn impl_file_ops(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) =
        ast.generics.split_for_impl();
    quote! {
        impl #impl_generics FileBuf for #name #type_generics #where_clause {
            fn file_and_buf_as_mut(&mut self) -> (&mut File,&mut BytesMut) {
                (self.file.as_mut().unwrap(), &mut self.buf)
            }
        }

        impl #impl_generics FileOps for #name #type_generics #where_clause {
            fn attach_file(&mut self,file: File) {
                self.file = Some(file);
            }
        }
    }
    .into()
}
