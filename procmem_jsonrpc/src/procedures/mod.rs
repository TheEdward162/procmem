//! Procedure definitions.

use serde::{Serialize, Deserialize};

use crate::rpc::RpcError;

pub trait Procedure<'a> {
	const NAME: &'static str;
	type Result: Serialize;
	type Error: RpcError<'a>;
}

macro_rules! define_procedure {
	(
		procedure $procedure_name: ident;
		type params = $params_tt: tt;
		type result = $result_tt: tt;
		type error = $error_tt: tt;
	) => {
		define_procedure!(
			__INNER expand_ty
			#[derive(Deserialize)]
			$procedure_name
			$params_tt
		);
		define_procedure!(
			__INNER expand_ty
			#[derive(Serialize)]
			ProcedureResult
			$result_tt
		);
		define_procedure!(
			__INNER expand_ty
			#[derive(Deserialize)]
			ProcedureError
			$error_tt
		);
		impl $crate::procedures::Procedure<'static> for $procedure_name {
			const NAME: &'static str = stringify!($procedure_name);
			type Result = ProcedureResult;
			type Error = ProcedureError;
		}
	};

	(
		__INNER expand_ty
		$(#[derive( $($derive_name: ident),+ )])?
		$name: ident
		{ struct
			$(
				$(#[$field_attr: meta])*
				$field_name: ident: $field_type: ty
			),+ $(,)?
		}
	) => {
		$(
			#[derive($($derive_name),+)]
		)?
		pub struct $name {
			$(
				$(#[$field_attr])*
				pub $field_name: $field_type
			),+
		}
		impl $name {
			pub fn new(
				$($field_name: $field_type),+
			) -> Self {
				$name {
					$($field_name),+
				}
			}
		}
	};

	(
		__INNER expand_ty
		$(#[derive( $($derive_name: ident),+ )])?
		$name: ident
		{ RpcError
			$(
				$variant_name: ident ($code: expr, $message: expr $(, $data_ty: ty)?)
			),+ $(,)?
		}
	) => {

	};

	(
		__INNER expand_ty
		$(#[derive( $($derive_name: ident),+ )])?
		$name: ident
		$result: ty
	) => {
		#[allow(non_camel_case_types)]
		pub type $name = $result;
	};
}



pub mod lock;
