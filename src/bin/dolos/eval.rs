use miette::{Context, IntoDiagnostic};
use pallas::{
    applying::{validate, Environment, UTxOs},
    ledger::traverse::{Era, MultiEraInput, MultiEraOutput},
};
use std::{borrow::Cow, collections::HashMap, path::PathBuf};

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long, short)]
    file: PathBuf,

    #[arg(long, short)]
    era: u16,

    #[arg(long, short)]
    slot: u64,
}

pub fn run(config: &super::Config, args: &Args) -> miette::Result<()> {
    crate::common::setup_tracing(&config.logging)?;

    let (_, _, ledger) = crate::common::open_data_stores(config)?;

    let cbor = std::fs::read_to_string(&args.file)
        .into_diagnostic()
        .context("reading tx from file")?;

    let cbor = hex::decode(&cbor)
        .into_diagnostic()
        .context("decoding hex content from file")?;

    let era = Era::try_from(args.era).unwrap();

    let tx = pallas::ledger::traverse::MultiEraTx::decode_for_era(era, &cbor)
        .into_diagnostic()
        .context("decoding tx cbor")?;

    let mut utxos = HashMap::new();
    ledger
        .resolve_inputs_for_tx(&tx, &mut utxos)
        .into_diagnostic()
        .context("resolving tx inputs")?;

    let mut utxos2 = UTxOs::new();

    for (ref_, output) in utxos.iter() {
        let txin = pallas::ledger::primitives::byron::TxIn::Variant0(
            pallas::codec::utils::CborWrap((ref_.0.clone(), ref_.1 as u32)),
        );

        let key = MultiEraInput::Byron(
            <Box<Cow<'_, pallas::ledger::primitives::byron::TxIn>>>::from(Cow::Owned(txin)),
        );

        let era = Era::try_from(output.0)
            .into_diagnostic()
            .context("parsing era")?;

        let value = MultiEraOutput::decode(era, &output.1)
            .into_diagnostic()
            .context("decoding utxo")?;

        utxos2.insert(key, value);
    }

    let env: Environment = ledger
        .get_active_pparams(args.slot)
        .into_diagnostic()
        .context("resolving pparams")?;

    validate(&tx, &utxos2, &env).unwrap();

    Ok(())
}
