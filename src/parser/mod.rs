use pest::Parser;
#[derive(Parser)]
#[grammar = "parser/expressions.pest"]
pub struct ExprParser;

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum Operator {
    Plus,
    Minus,
    Times,
    Div,
    Pow,
}

impl Operator {
    pub fn to_string(&self) -> String {
        match self {
            Operator::Plus  => "+".into(),
            Operator::Minus => "-".into(),
            Operator::Times => "*".into(),
            Operator::Div   => "/".into(),
            Operator::Pow   => "^".into(),
        }
    }

    pub fn from_str(name: &str) -> Self {
        match name {
            "+" => Operator::Plus,
            "-" => Operator::Minus,
            "*" => Operator::Times,
            "/" => Operator::Div,
            "^" => Operator::Pow,
            _ => unreachable!("matched an unknown operator symbol"),
        }
    }
}


#[derive(Debug, Clone)]
enum MathFunc {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sqrt,
    Exp,
    Log,
    Abs,
}

impl MathFunc {
    pub fn from_str(name: &str) -> Self {
        match name {
            "sin" => MathFunc::Sin,
            "cos" => MathFunc::Cos,
            "tan" => MathFunc::Tan,
            "asin" => MathFunc::Asin,
            "acos" => MathFunc::Acos,
            "atan" => MathFunc::Atan,
            "sqrt" => MathFunc::Sqrt,
            "exp" => MathFunc::Exp,
            "log" => MathFunc::Log,
            "abs" => MathFunc::Abs,
            _ => unreachable!("matched an unknown function keyword"),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            MathFunc::Sin  => "sin".into(),
            MathFunc::Cos  => "cos".into(),
            MathFunc::Tan  => "tan".into(),
            MathFunc::Asin => "asin".into(),
            MathFunc::Acos => "acos".into(),
            MathFunc::Atan => "atan".into(),
            MathFunc::Sqrt => "sqrt".into(),
            MathFunc::Exp  => "exp".into(),
            MathFunc::Log  => "log".into(),
            MathFunc::Abs  => "abs".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Number(f32),
    Ident(String),
    UnaryOp {
        operator: Operator,
        arg: Box<AstNode>,
    },
    PowOp {
        base: Box<AstNode>,
        exp: Box<AstNode>,
    },
    BinOp {
        lhs: Box<AstNode>,
        repeated_rhs: Vec<(Operator, Box<AstNode>)>,
    },
    Func {
        func: MathFunc,
        arg: Box<AstNode>,
    },
}

impl AstNode {
    pub fn to_string(&self) -> String {
        match self {
            // we might want to add parenthesis because the number might be negative. We need to check if this
            // is necessary to correctly translate stuff like "5 + -1" to the shading language
             // BEWARE: use the debug format, or it will forego the fractional part for integers
            AstNode::Number(val) => { format!("{:?}", val) },
            AstNode::Ident(ident) => ident.clone(),
            // we might want to add even more parenthesis because expression could be right after a binary operator
            // We need to check if this is necessary to correctly translate stuff like "3 * -sin(pi)" to the shading language
            AstNode::UnaryOp{ operator, arg } => { format!("{}({})", operator.to_string(), arg.to_string()) },
            AstNode::PowOp{ base, exp } => { format!("pow({},{})", base.to_string(), exp.to_string()) },
            AstNode::BinOp{ lhs, repeated_rhs } =>  {
                let mut to_return = String::new();
                to_return.push('(');
                to_return.push_str(&lhs.to_string());
                for rhs in repeated_rhs.iter() {
                    to_return.push(' ');
                    to_return.push_str(&rhs.0.to_string());
                    to_return.push(' ');
                    to_return.push_str(&rhs.1.to_string());
                }
                to_return.push(')');
                to_return
            },
            AstNode::Func{ func, arg } => { format!("{}({})", func.to_string(), arg.to_string()) },
        }
    }

    pub fn find_all_idents(&self) -> Vec<String> {
        match self {
            AstNode::Number(val) => vec![],
            AstNode::Ident(ident) => vec![ident.clone()],
            // we might want to add even more parenthesis because expression could be right after a binary operator
            // We need to check if this is necessary to correctly translate stuff like "3 * -sin(pi)" to the shading language
            AstNode::UnaryOp{ arg, .. } => { arg.find_all_idents() },
            AstNode::PowOp{ base, exp } => {
                let mut base_idents = base.find_all_idents();
                let mut exp_idents = exp.find_all_idents();
                base_idents.append(&mut exp_idents);
                base_idents
            },
            AstNode::BinOp{ lhs, repeated_rhs } =>  {
                let mut lhs_idents = lhs.find_all_idents();
                for rhs in repeated_rhs.iter() {
                    let mut rhs_idents = rhs.1.find_all_idents();
                    lhs_idents.append(&mut rhs_idents);
                }
                lhs_idents
            },
            AstNode::Func{ arg, .. } => { arg.find_all_idents() },
        }
    }
}

#[derive(Debug)]
pub enum AstError {
    MissingParenthesis(String),
    MultipleExpressions(String),
    MultipleOps(String),
    MultipleSigns(String),
    ImplicitProduct(String),
    InternalError(String),
    UnreachableMatch(String),
    PowAmbiguity(String),
    FailedParse(String),
    InvalidCharacter(String),
}

fn ast_node_from_pair(pair: pest::iterators::Pair<Rule>) -> Result<AstNode, AstError> {
    match pair.as_rule() {
        // sum and product can be handled exactly in the same way. They were defined as two different rules
        // just because this allowed for automatic resolution of precedence within the grammar.
        Rule::sum | Rule::product => {
            let mut inner_matches: VecDeque<_> = pair.into_inner().collect();
            // do something different depending on the length of inner_mathes:
            match inner_matches.len() {
                // if it just has length one, it means that this is not an actual sum/product.
                // recursively try to figure out what value this is
                1 => ast_node_from_pair(inner_matches.pop_front().unwrap()),
                // if the length is odd, then it means there are pairs of (operator and rhs) to be
                // matched
                n if (n & 1) == 1 => {
                    let mut all_rhs = Vec::<(Operator, Box<AstNode>)>::new();
                    let lhs_ast = ast_node_from_pair(inner_matches.pop_front().unwrap())?;
                    // all the remaining parts are processed in pairs
                    while !inner_matches.is_empty() {
                        let op_pair = inner_matches.pop_front().unwrap();
                        let addend_pair = inner_matches.pop_front().unwrap();

                        let single_rhs = ast_node_from_pair(addend_pair)?;
                        all_rhs.push((Operator::from_str(op_pair.as_str()), Box::new(single_rhs)));
                    }
                    Ok(AstNode::BinOp {
                        lhs: Box::new(lhs_ast),
                        repeated_rhs: all_rhs,
                    })
                },
                // in all other cases, error out
                _ => Err(AstError::InternalError("unexpected size of matches for Rule::sum | Rule::product".into())),
            }
        },
        // power is a bit peculiar, because we need to translate it into the use of the
        // `pow(base, exp)` function. We also match on its special operator "^", so it is
        // in a different match arm alltogether
        Rule::power => {
            let pair_str = pair.as_str();
            let mut inner_matches: VecDeque<_> = pair.into_inner().collect();
            // do something different depending on the length of inner_mathes:
            match inner_matches.len() {
                // if it just has length one, it means that this is not an actual sum/product.
                // recursively try to figure out what value this is
                1 => ast_node_from_pair(inner_matches.pop_front().unwrap()),
                // if the length is greater than one, then it means there one or more
                // terms, each one of them is separated by a "^". We only accept powers
                // of two numbers.
                2 => {
                    let base_ast = ast_node_from_pair(inner_matches.pop_front().unwrap())?;
                    let exp_ast = ast_node_from_pair(inner_matches.pop_front().unwrap())?;
                    Ok(AstNode::PowOp {
                        base: Box::new(base_ast),
                        exp: Box::new(exp_ast),
                    })
                },
                // If there are more than two terms, ask the user to add parenthesis to resolve any ambiguity
                _ => {
                    let err_str = format!("Three exponentiations create ambiguity, please add parenthesis: `{}`", pair_str);
                    Err(AstError::PowAmbiguity(err_str))
                }
            }
        },
        Rule::func => {
            let mut inner_matches: VecDeque<_> = pair.into_inner().collect();
            // A func always matches two rules: a keyword, and an argument
            assert_eq!(inner_matches.len(), 2);
            let keyword = inner_matches.pop_front().unwrap();
            let func = MathFunc::from_str(keyword.as_str());
            let arg_ast = ast_node_from_pair(inner_matches.pop_front().unwrap())?;
            // all the remaining parts are processed in pairs
            Ok(AstNode::Func {
                func,
                arg: Box::new(arg_ast),
            })
        },
        Rule::abs_func => {
            let mut inner_matches: VecDeque<_> = pair.into_inner().collect();
            // The abs_func rule matches when someone used vertical pipes: `y = |x|`
            assert_eq!(inner_matches.len(), 1);
            let arg_ast = ast_node_from_pair(inner_matches.pop_front().unwrap())?;
            // all the remaining parts are processed in pairs
            Ok(AstNode::Func {
                func: MathFunc::Abs,
                arg: Box::new(arg_ast),
            })
        },
        Rule::number => {
            // we found a number, just parse it and store as a f32!
            let parsed_number = pair.as_str().parse::<f32>();
            let number: f32 = parsed_number.or_else(|err| {
                let err_str = format!("unable to parse string `{}` as a number", err);
                Err(AstError::InternalError(err_str))
            })?;
            Ok(AstNode::Number(number))
        },
        Rule::unary_sign => {
            // we found an expression that migh be preceded by a sign!
            let mut inner_matches: VecDeque<_> = pair.into_inner().collect();
            // The value might be signed or not signed.
            match inner_matches.len() {
                // if it just has length one, it means that there is no sign.
                1 => ast_node_from_pair(inner_matches.pop_front().unwrap()),
                // if it has length two, then build the unary op
                2 => {
                    let op = inner_matches.pop_front().unwrap();
                    let arg_ast = ast_node_from_pair(inner_matches.pop_front().unwrap())?;
                    // all the remaining parts are processed in pairs
                    Ok(AstNode::UnaryOp {
                        operator: Operator::from_str(op.as_str()),
                        arg: Box::new(arg_ast),
                    })
                },
                // in all other cases, error out
                _ => Err(AstError::InternalError("unexpected size of matches for Rule::unary_sign".into())),
            }
        },
        Rule::ident => {
            // we found an identifier, we can make a copy and store it as a string
            Ok(AstNode::Ident(pair.as_str().into()))
        },
        // USER ERROR HANDLING STARTS HERE //
        Rule::func_no_parenthesis => {
            let err_str = format!("Please use parenthesis for mathematical functions: `{}`", pair.as_str());
            Err(AstError::MissingParenthesis(err_str))
        },
        Rule::implicit_product => {
            // we can decide to do different things. The wise one is to just return an error,
            // because otherwise things like 2t^3 become uber-difficult to handle correctly
            let err_str = format!("Please avoid using implicit products: `{}`", pair.as_str());
            Err(AstError::ImplicitProduct(err_str))
        },
        Rule::multiple_expressions => {
            let err_str = format!("Found two or more expressions without any operator within them: `{}`", pair.as_str());
            Err(AstError::MultipleExpressions(err_str))
        },
        Rule::repeating_ops => {
            let err_str = format!("Too many operation symbols chained together: `{}`", pair.as_str());
            Err(AstError::MultipleOps(err_str))
        },
        Rule::multiple_signs => {
            let err_str = format!("Remove the extra sign from this expression: `{}`", pair.as_str());
            Err(AstError::MultipleSigns(err_str))
        },
        Rule::invalid_character => {
            let err_str = format!("The input contains an invalid symbol: `{}`", pair.as_str());
            Err(AstError::InvalidCharacter(err_str))
        },
        Rule::non_signed_number | Rule::non_number_value | Rule::maybe_value | Rule::expr => {
            Err(AstError::UnreachableMatch("matched a silent rule".into()))
        },
        Rule::sign | Rule::only_fractional | Rule::integer | Rule::full_float | Rule::fractional | Rule::exponent => {
            Err(AstError::UnreachableMatch("matched a sub-signed-number rule".into()))
        }
        Rule::keyword | Rule::plus_minus | Rule::mul_div => {
            Err(AstError::UnreachableMatch("matched a keyword/operator rule".into()))
        },
        Rule::valid_character | Rule::invalid_line | Rule::eoi | Rule::line | Rule::WHITESPACE => {
            Err(AstError::UnreachableMatch("matched a control sequence rule".into()))
        },
    }
}

pub fn parse_expression(expr: &str) -> Result<AstNode, AstError> {
    let maybe_pairs = ExprParser::parse(Rule::line, expr);
    match maybe_pairs {
        Ok(mut pairs) => {
            ast_node_from_pair(pairs.next().unwrap())
        },
        Err(_) => {
            Err(AstError::FailedParse("Failed to parse. Please check the expression for mismatched parenthesis or other errors".into()))
        }
    }
}


