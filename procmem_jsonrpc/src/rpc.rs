//! https://www.jsonrpc.org/specification

use std::borrow::Cow;
use serde::{Serialize, Deserialize};

pub const RPC_VERSION: &'static str = "2.0";

/// Like the never type `!` or `std::conver::Infallible` but implements `Serialize`.
#[derive(Serialize, Copy, Clone)]
pub enum Null {}

/// Client id that is used to match requests and responses.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum ClientId<'a> {
	String(#[serde(borrow)] Cow<'a, str>),
	Number(isize)
}

/// Convenience trait for rpc errors.
pub trait RpcError<'a> {
	fn code(&self) -> isize;
	fn message(&self) -> Cow<'a, str>;

	type Data: Serialize + 'a;
	fn data(&self) -> Option<Self::Data>;
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[repr(isize)]
pub enum PredefinedError {
	ParseError = -32700,
	InvalidRequest = -32600,
	MethodNotFound = -32601,
	InvalidParams = -32602,
	InternalError = -32603,
	ServerError = -32000 // to -32099
}
impl RpcError<'static> for PredefinedError {
	type Data = ();

	fn code(&self) -> isize {
		*self as isize
	}

	fn message(&self) -> Cow<'static, str> {
		match self {
			PredefinedError::ParseError => "Parse error",
			PredefinedError::InvalidRequest => "Invalid Request",
			PredefinedError::MethodNotFound => "Method not found",
			PredefinedError::InvalidParams => "Invalid params",
			PredefinedError::InternalError => "Internal error",
			PredefinedError::ServerError => "Server error"
		}.into()
	}

	fn data(&self) -> Option<Self::Data> {
		None
	}
}

/// Convenience trait for simple `.into_json()` function.
pub trait IntoJson: Serialize {
	/// Serializes self into json.
	fn into_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string(self)
	}
}
impl<T: Serialize> IntoJson for T {}

/// Convenience trait for simple `.from_json_str()` function.
pub trait FromJson<'de>: Deserialize<'de> + Sized {
	/// Deserializes self from json string.
	fn from_json_str(value: &'de str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(value)
	}

	/// Deserializes self from json slice.
	fn from_json_slice(value: &'de [u8]) -> Result<Self, serde_json::Error> {
		serde_json::from_slice(value)
	}
}
impl<'de, T: Deserialize<'de>> FromJson<'de> for T {}

pub mod server {
	//! RPC interfaces as seen by the server.
	//!
	//! For requests, this is how the server parses a request.
	//! For responses, this is how the server serializes a response.

	use std::borrow::Cow;

	use serde::{Serialize, Deserialize};
	use serde_json::value::RawValue;

	use super::{ClientId, RPC_VERSION, RpcError};

	#[derive(Deserialize, Debug)]
	pub struct Request<'a> {
		/// Must be exactly "2.0".
		pub jsonrpc: &'a str,
		/// Method to invoke.
		pub method: &'a str,
		/// Optional params to the method. To be parsed once the method is known.
		#[serde(borrow)]
		pub params: Option<&'a RawValue>,
		/// Client identifier that will be included in the response. May be omitted if no response is to be sent.
		#[serde(default)]
		pub id: Option<ClientId<'a>>
	}
	#[cfg(test)]
	impl<'a> PartialEq for Request<'a> {
		fn eq(&self, other: &Self) -> bool {
			self.jsonrpc == other.jsonrpc
			&& self.method == other.method
			&& self.params.map(|rw| rw.get()) == other.params.map(|rw| rw.get())
			&& self.id == other.id
		}
	}

	#[derive(Serialize, Debug)]
	#[cfg_attr(test, derive(PartialEq))]
	pub enum ResponseResult<'a, T: Serialize = (), E: Serialize = ()> {
		#[serde(rename = "result")]
		Ok(T),
		#[serde(rename = "error")]
		Error {
			/// Type of the error.
			code: isize,
			/// Short description of the error.
			message: Cow<'a, str>,
			/// Optional additional information about the error.
			#[serde(skip_serializing_if = "Option::is_none")]
			data: Option<E>
		}
	}

	#[derive(Serialize, Debug)]
	#[cfg_attr(test, derive(PartialEq))]
	pub struct Response<'a, T: Serialize = (), E: Serialize = ()> {
		/// Must be exactly "2.0".
		pub jsonrpc: Cow<'a, str>,
		/// Result of the call.
		#[serde(flatten)]
		pub result: ResponseResult<'a, T, E>,
		/// Client identifier included in request, or `None` of it could not be determined.
		pub id: Option<ClientId<'a>>
	}
	impl<'a, T: Serialize> Response<'a, T, ()> {
		pub fn success(id: ClientId<'a>, value: T) -> Self {
			Response {
				jsonrpc: RPC_VERSION.into(),
				result: ResponseResult::Ok(value),
				id: Some(id)
			}
		}
	}
	impl<'a, E: Serialize> Response<'a, (), E> {
		pub fn error(
			id: Option<ClientId<'a>>,
			code: isize,
			message: Cow<'a, str>,
			data: Option<E>
		) -> Self {
			Response {
				jsonrpc: RPC_VERSION.into(),
				result: ResponseResult::Error {
					code,
					message,
					data
				},
				id
			}
		}

		pub fn from_rpc_error<Err: RpcError<'a>>(
			id: Option<ClientId<'a>>,
			error: Err
		) -> Response<'a, (), Err::Data> {
			Response {
				jsonrpc: RPC_VERSION.into(),
				result: ResponseResult::Error {
					code: error.code(),
					message: error.message(),
					data: error.data()
				},
				id
			}
		}
	}
}

pub mod client {
	//! RPC interfaces as seen by the client.
	//!
	//! For requests, this is how the client serializes a request.
	//! For responses, this is how a client parses a response.

	use std::borrow::Cow;

	use serde::{Serialize, Deserialize};
	use serde_json::value::RawValue;

	use super::{RPC_VERSION, ClientId};

	#[derive(Serialize, Debug)]
	pub struct Request<'a, P: Serialize = ()> {
		/// Must be exactly "2.0".
		pub jsonrpc: Cow<'a, str>,
		/// Method to invoke.
		pub method: Cow<'a, str>,
		/// Optional params to the method. May be omitted.
		#[serde(skip_serializing_if = "Option::is_none")]
		pub params: Option<P>,
		/// Client identifier that will be included in the response. May be omitted if no response is to be sent.
		#[serde(skip_serializing_if = "Option::is_none")]
		pub id: Option<ClientId<'a>>
	}
	impl<'a, P: Serialize> Request<'a, P> {
		pub fn new(
			method: Cow<'a, str>,
			params: Option<P>,
			id: ClientId<'a>
		) -> Self {
			Request {
				jsonrpc: RPC_VERSION.into(),
				method,
				params,
				id: Some(id)
			}
		}

		pub fn new_notification(
			method: Cow<'a, str>,
			params: Option<P>,
		) -> Self {
			Request {
				jsonrpc: RPC_VERSION.into(),
				method,
				params,
				id: None
			}
		}
	}

	#[derive(Deserialize, Debug)]
	pub enum ResponseResult<'a> {
		#[serde(rename = "result")]
		Result(#[serde(borrow)] &'a RawValue),
		#[serde(rename = "error")]
		Error {
			/// Type of the error.
			code: isize,
			/// Short description of the error.
			message: &'a str,
			/// Optional additional information about the error.
			#[serde(borrow)]
			data: Option<&'a RawValue>
		}
	}
	#[cfg(test)]
	impl<'a> PartialEq for ResponseResult<'a> {
		fn eq(&self, other: &Self) -> bool {
			match (self, other) {
				(ResponseResult::Result(a), ResponseResult::Result(b)) => a.get() == b.get(),
				(ResponseResult::Error { code: code_a, message: message_a, data: data_a }, ResponseResult::Error { code: code_b, message: message_b, data: data_b }) => {
					code_a == code_b
					&& message_a == message_b
					&& data_a.map(|rw| rw.get()) == data_b.map(|rw| rw.get())
				},
				(_, _) => false
			}
		}
	}

	#[derive(Deserialize, Debug)]
	pub struct ResponseError<'a> {
		/// Type of the error.
		pub code: isize,
		/// Short description of the error.
		pub message: &'a str,
		/// Optional additional information about the error.
		#[serde(borrow)]
		pub data: Option<&'a RawValue>
	}
	#[cfg(test)]
	impl<'a> PartialEq for ResponseError<'a> {
		fn eq(&self, other: &Self) -> bool {
			self.code == other.code
			&& self.message == other.message
			&& self.data.map(|rw| rw.get()) == other.data.map(|rw| rw.get())
		}
	}

	#[derive(Deserialize, Debug)]
	pub struct Response<'a> {
		/// Must be exactly "2.0".
		pub jsonrpc: &'a str,
		/// Result of the call.
		#[serde(default, borrow)]
		pub result: Option<&'a RawValue>,
		#[serde(default, borrow)]
		pub error: Option<ResponseError<'a>>,

		/// Client identifier included in request, or `None` of it could not be determined.
		#[serde(borrow)]
		pub id: Option<ClientId<'a>>
	}
	#[cfg(test)]
	impl<'a> PartialEq for Response<'a> {
		fn eq(&self, other: &Self) -> bool {
			self.jsonrpc == other.jsonrpc
			&& self.result.map(|rw| rw.get()) == other.result.map(|rw| rw.get())
			&& self.error == other.error
			&& self.id == other.id
		}
	}
}

#[cfg(test)]
mod test {
	use super::{ClientId, IntoJson, FromJson, client, server};

	#[test]
	fn test_rpc_request() {
		let client_request = client::Request::new(
			"foo".into(),
			Some((1, "hello")),
			ClientId::Number(1)
		);

		let json = client_request.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","method":"foo","params":[1,"hello"],"id":1}"#
		);
		
		let server_request = server::Request::from_json_str(&json).unwrap();

		assert_eq!(
			server_request,
			server::Request {
				jsonrpc: "2.0",
				method: "foo",
				params: Some(unsafe { std::mem::transmute(r#"[1,"hello"]"#) }),
				id: Some(ClientId::Number(1))
			}
		);
	}

	#[test]
	fn test_rpc_request_noparams() {
		let client_request = client::Request::new(
			"bar".into(),
			None::<()>,
			ClientId::Number(2)
		);

		let json = client_request.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","method":"bar","id":2}"#
		);
		
		let server_request = server::Request::from_json_str(&json).unwrap();

		assert_eq!(
			server_request,
			server::Request {
				jsonrpc: "2.0",
				method: "bar",
				params: None,
				id: Some(ClientId::Number(2))
			}
		);
	}

	#[test]
	fn test_rpc_request_notification() {
		let client_request = client::Request::new_notification(
			"baz".into(),
			Some(true),
		);

		let json = client_request.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","method":"baz","params":true}"#
		);
		
		let server_request = server::Request::from_json_str(&json).unwrap();

		assert_eq!(
			server_request,
			server::Request {
				jsonrpc: "2.0",
				method: "baz",
				params: Some(unsafe { std::mem::transmute(r#"true"#) }),
				id: None
			}
		);
	}

	#[test]
	fn test_rpc_request_notification_noparams() {
		let client_request = client::Request::new_notification(
			"baz".into(),
			None::<()>
		);

		let json = client_request.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","method":"baz"}"#
		);
		
		let server_request = server::Request::from_json_str(&json).unwrap();

		assert_eq!(
			server_request,
			server::Request {
				jsonrpc: "2.0",
				method: "baz",
				params: None,
				id: None
			}
		);
	}

	#[test]
	fn test_rpc_response_success() {
		let server_response = server::Response::success(
			ClientId::String("salmon".into()),
			(2, "hi")
		);

		let json = server_response.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","result":[2,"hi"],"id":"salmon"}"#
		);
		
		let client_response = client::Response::from_json_str(&json).unwrap();

		assert_eq!(
			client_response,
			client::Response {
				jsonrpc: "2.0",
				result: Some(unsafe { std::mem::transmute(r#"[2,"hi"]"#) }),
				error: None,
				id: Some(ClientId::String("salmon".into()))
			}
		);
	}

	#[test]
	fn test_rpc_response_error() {
		let server_response = server::Response::error(
			Some(ClientId::String("baba".into())),
			-3600,
			"my error".into(),
			Some((1, 2, true))
		);

		let json = server_response.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","error":{"code":-3600,"message":"my error","data":[1,2,true]},"id":"baba"}"#
		);
		
		let client_response = client::Response::from_json_str(&json).unwrap();

		assert_eq!(
			client_response,
			client::Response {
				jsonrpc: "2.0",
				result: None,
				error: Some(client::ResponseError {
					code: -3600,
					message: "my error",
					data: Some(unsafe { std::mem::transmute(r#"[1,2,true]"#) })
				}),
				id: Some(ClientId::String("baba".into()))
			}
		);
	}

	#[test]
	fn test_rpc_response_error_nodata() {
		let server_response = server::Response::error(
			Some(ClientId::String("gaga".into())),
			123,
			"axf".into(),
			None::<()>
		);

		let json = server_response.into_json().unwrap();
		assert_eq!(
			json,
			r#"{"jsonrpc":"2.0","error":{"code":123,"message":"axf"},"id":"gaga"}"#
		);
		
		let client_response = client::Response::from_json_str(&json).unwrap();

		assert_eq!(
			client_response,
			client::Response {
				jsonrpc: "2.0",
				result: None,
				error: Some(client::ResponseError {
					code: 123,
					message: "axf",
					data: None
				}),
				id: Some(ClientId::String("gaga".into()))
			}
		);
	}
}