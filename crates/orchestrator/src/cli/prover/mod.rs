use sharp_service::SharpValidatedArgs;

pub mod sharp;

#[derive(Debug, Clone)]
pub enum ProverParams {
    Sharp(SharpValidatedArgs),
}
