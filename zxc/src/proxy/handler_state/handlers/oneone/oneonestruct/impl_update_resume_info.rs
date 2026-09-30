use super::*;
use crate::{
    interceptor::message::from_ui::resume_info::ResumeInfo,
    proxy::handler_state::transition::resume_intercept::update_resume_info::UpdateResumeInfo,
};

/* Steps:
 *      Update whether the response needs to be intercepted updated
 *
 *      OneRequestLine only, used by response in should_intercept state
 */

impl<T, E> UpdateResumeInfo for OneOneStruct<T, E, OneRequestLine> {
    fn update_resume_info(&mut self, resume_info: &ResumeInfo) {
        self.need_response = resume_info.need_response();
    }
}

// Blanket implementation
impl<T, E> UpdateResumeInfo for OneOneStruct<T, E, OneResponseLine> {
    #[inline(always)]
    fn update_resume_info(&mut self, _: &ResumeInfo) {}
}
