mod buffer;
mod can_communicate;
mod close_action;
mod file_ops;
mod flush_storage;
mod get_path;
mod id;
mod log;
mod notify;
mod repeater_read_from_file;
mod role;
mod send_history;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(RepeaterReadFile)]
pub fn repeaterreadfile(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    repeater_read_from_file::impl_repeater_read_from_file(&ast)
}

#[proc_macro_derive(GetPath)]
pub fn getpath(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    get_path::impl_get_path(&ast)
}

#[proc_macro_derive(FileOps)]
pub fn fileops(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    file_ops::impl_file_ops(&ast)
}

#[proc_macro_derive(GetRole)]
pub fn get_role(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    role::impl_get_role(&ast)
}

#[proc_macro_derive(CanCommunicate)]
pub fn can_communicate(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    can_communicate::impl_can_communicate(&ast)
}

#[proc_macro_derive(Id)]
pub fn id(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    id::impl_id(&ast)
}

#[proc_macro_derive(Buffer)]
pub fn buffer(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    buffer::impl_buffer(&ast)
}

#[proc_macro_derive(FlushStorage)]
pub fn flush_storage(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    flush_storage::impl_flush_storage(&ast)
}

#[proc_macro_derive(CloseAction)]
pub fn close_action(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    close_action::impl_close_action(&ast)
}

#[proc_macro_derive(NotifyCommander)]
pub fn notify_commander(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    notify::impl_notify_commander(&ast)
}

#[proc_macro_derive(SendHistory)]
pub fn send_history(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    send_history::impl_send_history(&ast)
}

#[proc_macro_derive(Log)]
pub fn log(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    log::impl_log(&ast)
}
