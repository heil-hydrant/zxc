use super::*;
use crate::interceptor::message::from_ui::resume_info::ResumeInfo;
use crate::proxy::handler_state::transition::update_frame::should_rewrite::ShouldRewrite;

/* Steps:
 *      ShouldRewrite trait implementation for OneOneHandler.
 *
 *      If user sets b:update to true , i.e. frame should be updated according
 *      to rfc , then request/response should be rewritten.
 *
 *      If b:update is false, since the file is already written by UI, no need
 *      to rewrite.
 *
 */

impl<T, E, U> ShouldRewrite for OneOneStruct<T, E, U>
where
    U: OneInfoLine,
{
    #[inline(always)]
    fn should_rewrite(&self, resume_info: &ResumeInfo) -> bool {
        resume_info.update()
    }
}
