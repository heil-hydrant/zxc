use super::*;
use crate::file_types::{EXT_REQ, EXT_RES};
use crate::proxy::handler_state::transition::write_log::update_log_extension::UpdateLogExt;

/* Steps:
 *      1. Push the log_id to the path (history/index/index).
 *      2. Set the extension to .req (history/index/index.req).
 *
 * Note:
 *      history/index - directory created by commander
 */

impl<T, E> UpdateLogExt for OneOneStruct<T, E, OneRequestLine> {
    fn update_extension(&mut self) {
        let path = self.path.as_mut().unwrap();
        path.push(self.log_id.to_string());
        path.set_extension(EXT_REQ);
    }
}

/* Steps:
 *      Set the extension to .res
 */

impl<T, E> UpdateLogExt for OneOneStruct<T, E, OneResponseLine> {
    fn update_extension(&mut self) {
        self.path
            .as_mut()
            .unwrap()
            .set_extension(EXT_RES);
    }
}
