use header_plz::OneRequestLine;
use header_plz::OneResponseLine;

use super::*;
use crate::proxy::states::Connection;
mod from_connection;
mod from_connection_add_info;
mod request_to_response;
mod response_to_request;
mod to_connection_add_info;
mod to_state_connection;
mod try_from_proxy_state;
