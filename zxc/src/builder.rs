use std::fs::{File, remove_file};
use std::io::{BufRead, BufReader, Error, Read};

use tokio::net::UnixListener;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::{error, trace};

use crate::addons::handler::AddonHandler;
use crate::commander::captain_crypto::CaptainCrypto;
use crate::commander::communicate::comm_history::HistoryComm;
use crate::commander::communicate::comm_interceptor::InterceptorComm;
use crate::commander::communicate::comm_repeater::RepeaterComm;
use crate::commander::{Commander, CommanderRequest};
use crate::config::global::parser::parse_global_config;
use crate::config::local::proxy::ProxyArgs;
use crate::config::{Config, GlobalConfig};
use crate::forward_info::ForwardInfo;
use crate::history::handler::HistoryHandler;
use crate::history::message::from_commander::CommanderToHistory;
use crate::history::message::from_ui::HistoryUIOps;
use crate::interceptor::InterceptorHandler;
use crate::interceptor::message::from_ui::InterUIOps;
use crate::interceptor::message::to_ui::InterToUI;
use crate::proxy::handler_state::transition::write_history::HistoryEnum;
use crate::repeater::RepeaterHandler;

pub const HISTORY_STATE_FILE: &str = ".history.state";

pub const PROXY: &str = "proxy";
pub const COMMANDER: &str = "commander";

pub const INTERCEPTOR: &str = "interceptor";
pub const HISTORY: &str = "history";
pub const REPEATER: &str = "repeater";
pub const ADDONS: &str = "addons";

pub struct Builder {
    attach: bool,
    captain_crypto: CaptainCrypto,
    comm_addons: Option<Sender<ForwardInfo>>,
    comm_history: Option<HistoryComm>,
    comm_interceptor: Option<InterceptorComm>,
    comm_repeater: Option<RepeaterComm>,
    comm_soldiers: Option<Receiver<CommanderRequest>>,
    global_config: Option<GlobalConfig>,
    index: usize,
    sname: String,
}

impl Builder {
    pub fn new(index: usize, sname: String) -> Builder {
        let attach = index != 1;
        Builder {
            attach,
            captain_crypto: CaptainCrypto::new().unwrap(),
            comm_addons: None,
            comm_history: None,
            comm_interceptor: None,
            comm_repeater: None,
            comm_soldiers: None,
            global_config: None,
            index,
            sname,
        }
    }

    pub fn parse_global_config(&mut self) {
        self.global_config = parse_global_config()
            .map_err(|e| error!("parse global config| {}", e))
            .ok()
            .flatten();
    }

    pub fn build_listener(
        &self,
        mod_name: &str,
    ) -> Result<UnixListener, Error> {
        let sock = format!("/tmp/{}/{}.sock", self.sname, mod_name);
        UnixListener::bind(sock)
    }

    pub fn build_history(&mut self) -> HistoryHandler {
        let (send_cth, recv_cth) = channel::<CommanderToHistory>(100); // Commander to History
        let (send_htc, recv_htc) = channel::<HistoryUIOps>(1); // History to Commander

        let storage = if self.attach {
            read_history_state(self.index).unwrap_or_else(|e| {
                if !matches!(e.kind(), std::io::ErrorKind::NotFound) {
                    error!("failed to read history state| {}", e);
                }
                Vec::new()
            })
        } else {
            Vec::new()
        };

        self.comm_history =
            Some(HistoryComm::new(self.index, recv_htc, send_cth));
        HistoryHandler::new(recv_cth, send_htc, storage)
    }

    pub fn build_soldier_comm(&mut self) -> Sender<CommanderRequest> {
        let (tx, rx) = channel::<CommanderRequest>(100);
        self.comm_soldiers = Some(rx);
        tx
    }

    pub fn build_interceptor(&mut self) -> InterceptorHandler {
        let (send_itc, recv_itc) = channel::<InterUIOps>(1); // Interceptor to Commander
        let (send_cti, recv_cti) = channel::<InterToUI>(1); //  Commander to Interceptor
        self.comm_interceptor = Some(InterceptorComm::new(recv_itc, send_cti));
        InterceptorHandler::new(send_itc, recv_cti)
    }

    pub fn build_repeater(&mut self) -> RepeaterHandler {
        let (send_ctr, recv_ctr) = channel::<ForwardInfo>(1); // Commander to Repeater
        let (send_rtc, recv_rtc) = channel::<ForwardInfo>(1); // Repeater to Commander
        self.comm_repeater = Some(RepeaterComm::new(recv_rtc, send_ctr));
        let connector = self.captain_crypto.get_connector();
        RepeaterHandler::new(connector, recv_ctr, send_rtc)
    }

    pub fn build_addons(&mut self) -> AddonHandler {
        let (send_cta, recv_cta) = channel::<ForwardInfo>(1); // Commander to Addons
        self.comm_addons = Some(send_cta);
        let addons = self
            .global_config
            .as_mut()
            .and_then(|config| config.parse_addons())
            .unwrap_or_default();
        AddonHandler::new(recv_cta, addons)
    }

    pub fn build_commander(
        self,
        local_config: Option<ProxyArgs>,
    ) -> Commander {
        let config = Config::build(local_config, self.global_config);
        if let Some(c) = config.as_ref() {
            trace!("config| {:?}", c);
        } else {
            trace!("config| None");
        }

        Commander::new(
            self.captain_crypto,
            self.comm_addons.unwrap(),
            self.comm_history.unwrap(),
            self.comm_interceptor.unwrap(),
            self.comm_repeater.unwrap(),
            self.comm_soldiers.unwrap(),
            config,
        )
    }
}

#[inline(always)]
fn read_history_state(id: usize) -> Result<Vec<String>, Error> {
    let file = File::open(HISTORY_STATE_FILE)?;
    let result = remove_unlogged(file, id);
    remove_file(HISTORY_STATE_FILE)?;
    Ok(result)
}

/* Description:
 *      Remove unlogged entries from .history.state
 *
 * Steps:
 *      1. For each line in file, serialize line to HistoryEnum
 *      2. If HistoryEnum.id < max, add line to result
 */

#[inline(always)]
fn remove_unlogged<T>(file: T, max: usize) -> Vec<String>
where
    T: Read,
{
    let mut result = Vec::new();
    let f = BufReader::new(file);
    let lines = f.lines();
    for raw_line in lines {
        match raw_line {
            Ok(line) => match serde_json::from_str::<HistoryEnum>(&line) {
                Ok(val) => {
                    if val.id().is_none_or(|id| id < max) {
                        result.push(line);
                    }
                }
                Err(e) => {
                    error!("remove_unlogged| deserialize| {}| {}", e, line)
                }
            },
            Err(e) => error!("remove_unlogged| read| {}", e),
        }
    }
    result
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_history_state() {
        let datav = vec![
            r#"{"Request":{"id":112,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Request":{"id":113,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":112,"status":"200","length":538,"mime":"ukn"}}"#,
            r#"{"Response":{"id":113,"status":"200","length":538,"mime":"ukn"}}"#,
            r#"{"Request":{"id":114,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":114,"status":"200","length":538,"mime":"ukn"}}"#,
            r#"{"Request":{"id":115,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":115,"status":"200","length":538,"mime":"ukn"}}"#,
            r#"{"Request":{"id":116,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":116,"status":"200","length":538,"mime":"ukn"}}"#,
        ];
        let data = datav.join("\n");
        // 113
        let result = remove_unlogged(data.as_bytes(), 113);
        let verify = &[datav[0], datav[2]];
        assert_eq!(result, verify);

        // 114
        let result = remove_unlogged(data.as_bytes(), 114);
        let verify = &datav[0..4];
        assert_eq!(result, verify);

        // 115
        let result = remove_unlogged(data.as_bytes(), 115);
        let verify = &datav[0..6];
        assert_eq!(result, verify);

        // 116
        let result = remove_unlogged(data.as_bytes(), 116);
        let verify = &datav[0..8];
        assert_eq!(result, verify);

        // 117
        let result = remove_unlogged(data.as_bytes(), 117);
        assert_eq!(result, datav);
    }
}
