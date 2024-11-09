use sharp_service::SharpParams;

pub mod sharp;

#[derive(Debug, Clone)]
pub enum ProverParams {
    Sharp(SharpParams),
}
