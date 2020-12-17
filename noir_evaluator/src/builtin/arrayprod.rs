use super::{BuiltInCaller, RuntimeErrorKind};
use crate::object::{Array, Object};
use crate::binary_op;
use crate::{CallExpression, Environment, Evaluator};

/// Takes the direct product of the elements in an array
pub struct ArrayProd;

impl BuiltInCaller for ArrayProd {
    fn call(
        evaluator: &mut Evaluator,
        env: &mut Environment,
        mut call_expr: CallExpression,
    ) -> Result<Object, RuntimeErrorKind> {

        let arr_expr = {
            assert_eq!(call_expr.arguments.len(),1);
            call_expr.arguments.pop().unwrap()
        };

        // ArrayProd should only take a single parameter, which is an array. This should have been caught by the compiler in the analysis phase
        let arr = Array::from_expression(evaluator, env, arr_expr)?;

        let mut result = arr.get(0)?;
        for i in 1..arr.contents.len(){
            result = binary_op::handle_mul_op(result, arr.get(i as u128)?,env, evaluator)?;
        }

        Ok(result)
    }
}