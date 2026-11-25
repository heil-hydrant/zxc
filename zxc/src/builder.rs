use std::io::{BufRead, Error, Read};
use std::path::PathBuf;

use tokio::net::UnixListener;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::{error, trace};

use crate::addons::AddonHandler;
use crate::commander::captain_crypto::CryptoBuildError;
use crate::commander::{
    CaptainCrypto, Commander, CommanderRequest, HistoryComm, InterceptorComm,
    RepeaterComm,
};
use crate::config::{Config, GlobalConfig, ProxyArgs, parse_global_config};
use crate::core::{
    ADDONS, HISTORY, HISTORY_STATE_FILE, INTERCEPTOR, REPEATER,
};
use crate::forward_info::ForwardInfo;
use crate::history::{CommanderToHistory, HistoryHandler, HistoryUIOps};
use crate::interceptor::{InterToUI, InterUIOps, InterceptorHandler};
use crate::proxy::HistoryEnum;
use crate::repeater::RepeaterHandler;

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("listener {path}: {error}")]
    Listener {
        path: PathBuf,
        #[source]
        error: Error,
    },
    #[error("module not built: {0}")]
    MissingModule(&'static str),
}

pub struct Builder {
    attach: bool,
    captain_crypto: CaptainCrypto,
    comm_addons: Option<Sender<ForwardInfo>>,
    comm_history: Option<HistoryComm>,
    comm_interceptor: Option<InterceptorComm>,
    comm_repeater: Option<RepeaterComm>,
    global_config: Option<GlobalConfig>,
    index: usize,
    sname: String,
}

impl Builder {
    pub fn new(
        index: usize,
        sname: String,
    ) -> Result<Builder, CryptoBuildError> {
        let attach = index != 1;
        Ok(Builder {
            attach,
            captain_crypto: CaptainCrypto::new()?,
            comm_addons: None,
            comm_history: None,
            comm_interceptor: None,
            comm_repeater: None,
            global_config: None,
            index,
            sname,
        })
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
    ) -> Result<UnixListener, BuilderError> {
        let path =
            PathBuf::from(format!("/tmp/{}/{}.sock", self.sname, mod_name));
        UnixListener::bind(&path).map_err(|e| BuilderError::Listener {
            path,
            error: e,
        })
    }

    pub fn build_history(&mut self) -> HistoryHandler {
        let (send_cth, recv_cth) = channel::<CommanderToHistory>(100); // Commander to History
        let (send_htc, recv_htc) = channel::<HistoryUIOps>(1); // History to Commander

        let list = match self
            .attach
            .then(|| read_history_state(self.index))
        {
            Some(Ok(list)) => list,
            Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => {
                error!("failed to read history state: {e}");
                Vec::new()
            }
            _ => Vec::new(),
        };

        self.comm_history =
            Some(HistoryComm::new(self.index, recv_htc, send_cth));
        HistoryHandler::new(recv_cth, send_htc, list)
    }

    pub fn build_soldier_comm(
        &self,
    ) -> (Sender<CommanderRequest>, Receiver<CommanderRequest>) {
        channel::<CommanderRequest>(100)
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
        comm_soldiers: Receiver<CommanderRequest>,
    ) -> Result<Commander, BuilderError> {
        let config = Config::build(local_config, self.global_config);
        if let Some(c) = config.as_ref() {
            trace!("config| {:?}", c);
        } else {
            trace!("config| None");
        }

        let comm_addons = self
            .comm_addons
            .ok_or(BuilderError::MissingModule(ADDONS))?;
        let comm_history = self
            .comm_history
            .ok_or(BuilderError::MissingModule(HISTORY))?;
        let comm_interceptor = self
            .comm_interceptor
            .ok_or(BuilderError::MissingModule(INTERCEPTOR))?;
        let comm_repeater = self
            .comm_repeater
            .ok_or(BuilderError::MissingModule(REPEATER))?;

        Ok(Commander::new(
            self.captain_crypto,
            comm_addons,
            comm_history,
            comm_interceptor,
            comm_repeater,
            comm_soldiers,
            config,
        ))
    }
}

#[inline(always)]
fn read_history_state(id: usize) -> Result<Vec<String>, Error> {
    let file = std::fs::File::open(HISTORY_STATE_FILE)?;
    let result = remove_unlogged(file, id);
    std::fs::remove_file(HISTORY_STATE_FILE)?;
    Ok(result)
}

/// Removes unlogged entries from `.history.state`.
///
/// # Steps
///
/// 1. Serialize each line in the file to `HistoryEnum`.
/// 2. Keep the line when the ID is absent or less than `max`.
#[inline(always)]
fn remove_unlogged<T>(file: T, max: usize) -> Vec<String>
where
    T: Read,
{
    let mut result = Vec::new();
    let f = std::io::BufReader::new(file);
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
            r#"{"Response":{"id":112,"status":200,"length":538,"mime":"ukn"}}"#,
            r#"{"Response":{"id":113,"status":200,"length":538,"mime":"ukn"}}"#,
            r#"{"Request":{"id":114,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":114,"status":200,"length":538,"mime":"ukn"}}"#,
            r#"{"Request":{"id":115,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":115,"status":200,"length":538,"mime":"ukn"}}"#,
            r#"{"Request":{"id":116,"method":"GET","host":"www.reddit.com","uri":"/robots.txt"}}"#,
            r#"{"Response":{"id":116,"status":200,"length":538,"mime":"ukn"}}"#,
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
