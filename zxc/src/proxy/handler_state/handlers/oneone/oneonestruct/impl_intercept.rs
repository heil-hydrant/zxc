use super::*;
use crate::interceptor::message::to_ui::InterToUI;
use crate::proxy::handler_state::transition::intercept::Intercept;
use crate::proxy::server_info::json::ServerInfoJson;

impl<T, E> Intercept for OneOneStruct<T, E, OneRequestLine> {
    fn get_inter_info(&self) -> InterToUI {
        let info = ServerInfoJson::from(&self.server_info);
        InterToUI::build_http_req(self.log_id, Some(info))
    }
}

impl<T, E> Intercept for OneOneStruct<T, E, OneResponseLine> {
    fn get_inter_info(&self) -> InterToUI {
        InterToUI::build_http_res(self.log_id)
    }
}
