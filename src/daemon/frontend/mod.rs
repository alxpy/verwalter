use std::io;
use std::io::Read;
use std::str::from_utf8;
use std::sync::Arc;
use std::path::Path;
use std::path::Component::ParentDir;
use std::fs::File;
use std::time::{Duration};
use std::collections::HashMap;

use rotor::{Scope, Time};
use rotor_http::server::{Server, Response, RecvMode, Head};
use rustc_serialize::Encodable;
use rustc_serialize::json::{as_json, as_pretty_json, Json};

use net::Context;
use elect::Epoch;
use shared::{PushActionError, Id};

#[derive(Clone, Debug)]
pub enum ApiRoute {
    Status,
    Peers,
    Schedule,
    Scheduler,
    SchedulerDebugInfo,
    Election,
    PushAction,
    ActionIsPending(u64),
    PendingActions,
}

#[derive(Clone, Debug, Copy)]
pub enum Format {
    Json,
    Plain,
}

#[derive(Clone, Debug)]
pub enum Route {
    Index,
    Static(String),
    Api(ApiRoute, Format),
}

pub struct Public(Route);

fn path_component(path: &str) -> (&str, &str) {
    let path = if path.starts_with('/') {
        &path[1..]
    } else {
        path
    };
    match path.bytes().position(|x| x == b'/') {
        Some(end) => (&path[..end], &path[end+1..]),
        None => {
            let end = path.bytes().position(|x| x == b'.')
                .unwrap_or(path.as_bytes().len());
            (&path[..end], "")
        }
    }
}

fn suffix(path: &str) -> &str {
    match path.bytes().rposition(|x| x == b'.' || x == b'/') {
        Some(i) if path.as_bytes()[i] == b'.' => &path[i+1..],
        Some(_) => "",
        None => "",
    }
}

fn read_file<P:AsRef<Path>>(path: P, res: &mut Response)
    -> Result<(), io::Error>
{
    let path = path.as_ref();
    for cmp in path.components() {
        if matches!(cmp, ParentDir) {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied,
                "Parent dir `..` path components are not allowed"));
        }
    }
    let mut file = try!(File::open(path));
    let mut buf = Vec::with_capacity(1024);
    try!(file.read_to_end(&mut buf));
    res.status(200, "OK");
    res.add_length(buf.len() as u64).unwrap();
    // TODO(tailhook) guess mime type
    res.done_headers().unwrap();
    res.write_body(&buf);
    res.done();
    Ok(())
}

fn parse_api(path: &str) -> Option<Route> {
    use self::Route::*;
    use self::ApiRoute::*;
    use self::Format::*;
    match path_component(path) {
        ("status", "") => Some(Api(Status,
            if suffix(path) == "pretty" { Plain } else { Json })),
        ("peers", "") => Some(Api(Peers,
            if suffix(path) == "pretty" { Plain } else { Json })),
        ("schedule", "") => Some(Api(Schedule,
            if suffix(path) == "pretty" { Plain } else { Json })),
        ("scheduler", "") => Some(Api(Scheduler,
            if suffix(path) == "pretty" { Plain } else { Json })),
        ("scheduler_debug_info", "") => Some(Api(SchedulerDebugInfo, Plain)),
        ("election", "") => Some(Api(Election,
            if suffix(path) == "pretty" { Plain } else { Json })),
        ("action", "") => Some(Api(PushAction,
            if suffix(path) == "pretty" { Plain } else { Json })),
        ("action_is_pending", tail) => {
            tail.parse().map(|x| {
                Api(ActionIsPending(x),
                    if suffix(path) == "pretty" { Plain } else { Json })
            }).ok()
        }
        ("pending_actions", "") => Some(Api(PendingActions,
            if suffix(path) == "pretty" { Plain } else { Json })),
        _ => None,
    }
}

fn respond<T: Encodable>(res: &mut Response, format: Format, data: T)
    -> Result<(), io::Error>
{
    res.status(200, "OK");
    res.add_header("Content-Type", b"application/json").unwrap();
    let data = match format {
        Format::Json => format!("{}", as_json(&data)),
        Format::Plain => format!("{}", as_pretty_json(&data)),
    };
    res.add_length(data.as_bytes().len() as u64).unwrap();
    res.done_headers().unwrap();
    res.write_body(data.as_bytes());
    res.done();
    Ok(())
}

fn respond_text<T: AsRef<[u8]>>(res: &mut Response, data: T)
    -> Result<(), io::Error>
{
    let data = data.as_ref();
    res.status(200, "OK");
    res.add_header("Content-Type", b"text/plain").unwrap();
    res.add_length(data.len() as u64).unwrap();
    res.done_headers().unwrap();
    res.write_body(data);
    res.done();
    Ok(())
}

fn serve_api(scope: &mut Scope<Context>, route: &ApiRoute,
    data: &[u8], format: Format, res: &mut Response)
    -> Result<(), io::Error>
{
    use self::ApiRoute::*;
    match *route {
        Status => {
            #[derive(RustcEncodable)]
            struct LeaderInfo<'a> {
                id: &'a Id,
                name: &'a str,
                hostname: &'a str,
                addr: Option<String>,
            }
            #[derive(RustcEncodable)]
            struct Status<'a> {
                version: &'static str,
                id: &'a Id,
                peers: usize,
                leader: Option<LeaderInfo<'a>>,
                scheduler_state: &'static str,
                election_epoch: Epoch,
                errors: HashMap<&'static str, Arc<String>>,
            }
            let peers = scope.state.peers();
            let election = scope.state.election();
            let leader_id = if election.is_leader {
                Some(scope.state.id())
            } else {
                election.leader.as_ref()
            };
            let leader = leader_id.and_then(
                |id| peers.as_ref().and_then(|x| x.1.get(id)));
            respond(res, format, &Status {
                version: concat!("v", env!("CARGO_PKG_VERSION")),
                id: scope.state.id(),
                peers: peers.as_ref().map(|x| x.1.len()).unwrap_or(0),
                leader: leader.map(|peer| LeaderInfo {
                    id: leader_id.unwrap(),
                    name: &peer.name,
                    hostname: &peer.hostname,
                    addr: peer.addr.map(|x| x.to_string()),
                }),
                scheduler_state: scope.state.scheduler_state().describe(),
                election_epoch: election.epoch,
                errors: scope.state.errors(),
            })
        }
        Peers => {
            respond(res, format, &scope.cantal.get_peers().as_ref()
                .map(|x| &x.peers))
        }
        Schedule => {
            if let Some(schedule) = scope.state.schedule() {
                respond(res, format, &schedule)
            } else {
                // TODO(tailhook) Should we return error code instead ?
                respond(res, format, Json::Null)
            }
        }
        Scheduler => {
            respond(res, format, &scope.state.scheduler_state())
        }
        SchedulerDebugInfo => {
            respond_text(res, &*scope.state.scheduler_debug_info())
        }
        Election => {
            respond(res, format, &scope.state.election())
        }
        PendingActions => {
            respond(res, format, &scope.state.pending_actions())
        }
        PushAction => {
            let jdata = from_utf8(data).ok()
                .and_then(|x| Json::from_str(x).ok());
            match jdata {
                Some(x) => {
                    match scope.state.push_action(x) {
                        Ok(id) => {
                            respond(res, format, {
                                let mut h = HashMap::new();
                                h.insert("registered", id);
                                h
                            })
                        }
                        Err(PushActionError::TooManyRequests) => {
                            serve_error_page(429, res);
                            Ok(())
                        }
                        Err(PushActionError::NotALeader) => {
                            serve_error_page(421, res);
                            Ok(())
                        }
                    }
                }
                None => {
                    serve_error_page(400, res);
                    Ok(())
                }
            }
        }
        ActionIsPending(id) => {
            respond(res, format, {
                let mut h = HashMap::new();
                h.insert("pending", scope.state.check_action(id));
                h
            })
        }
    }
}


fn serve_error_page(code: u32, response: &mut Response) {
    let (status, reason) = match code {
        400 => (400, "Bad Request"),
        404 => (404, "Not Found"),
        408 => (408, "Request Timeout"),
        413 => (413, "Payload Too Large"),
        421 => (421, "Misdirected Request"),
        429 => (429, "Too Many Requests"),
        431 => (431, "Request Header Fields Too Large"),
        500 => (500, "Internal Server Error"),
        _ => unreachable!(),
    };
    response.status(status, reason);
    let data = format!("<h1>{} {}</h1>\n\
        <p><small>Served for you by rotor-http</small></p>\n",
        status, reason);
    let bytes = data.as_bytes();
    response.add_length(bytes.len() as u64).unwrap();
    response.add_header("Content-Type", b"text/html").unwrap();
    response.done_headers().unwrap();
    response.write_body(bytes);
    response.done();
}

impl Server for Public {
    type Context = Context;
    type Seed = ();
    fn headers_received(_seed: (), head: Head, res: &mut Response,
        scope: &mut Scope<Context>)
        -> Option<(Self, RecvMode, Time)>
    {
        use self::Route::*;
        if !head.path.starts_with('/') {
            // Don't know what to do with that ugly urls
            return None;
        }
        let path = match head.path.find('?') {
            Some(x) => &head.path[..x],
            None => head.path,
        };
        let route = match path_component(&path[..]) {
            ("", _) => Some(Index),
            ("v1", suffix) => parse_api(suffix),
            (_, _) => Some(Static(path.to_string())),
        };
        trace!("Routed {:?} to {:?}", head, route);
        match route {
            Some(route) => {
                Some((Public(route), RecvMode::Buffered(65536),
                    scope.now() + Duration::new(120, 0)))
            }
            None => {
                serve_error_page(500, res);
                None
            }
        }
    }
    fn request_received(self, data: &[u8], res: &mut Response,
        scope: &mut Scope<Context>)
        -> Option<Self>
    {
        use self::Route::*;
        let iores = match *&self.0 {
            Index => read_file(scope.frontend_dir
                               .join("common/index.html"), res),
            Static(ref x) => {
                match read_file(scope.frontend_dir.join(&x[1..]), res) {
                    Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                        read_file(scope.frontend_dir
                            .join("common/index.html"), res)
                    }
                    res => res,
                }
            }
            Api(ref route, fmt) => serve_api(scope, route, data, fmt, res),
        };
        match iores {
            Ok(()) => {}
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                serve_error_page(404, res);
            }
            Err(ref e) if e.kind() == io::ErrorKind::PermissionDenied => {
                serve_error_page(403, res);
            }
            Err(e) => {
                info!("Error serving {:?}: {}", self.0, e);
                serve_error_page(500, res);
            }
        }
        None
    }
    fn request_chunk(self, _chunk: &[u8], _response: &mut Response,
        _scope: &mut Scope<Context>)
        -> Option<Self>
    {
        unreachable!();
    }

    /// End of request body, only for Progressive requests
    fn request_end(self, _response: &mut Response, _scope: &mut Scope<Context>)
        -> Option<Self>
    {
        unreachable!();
    }

    fn timeout(self, _response: &mut Response, _scope: &mut Scope<Context>)
        -> Option<(Self, Time)>
    {
        unimplemented!();
    }
    fn wakeup(self, _response: &mut Response, _scope: &mut Scope<Context>)
        -> Option<Self>
    {
        unimplemented!();
    }
}