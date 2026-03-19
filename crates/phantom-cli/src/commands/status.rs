//! `phantom status` — Show system status overview.

use phantom_core::pipeline::BuildPhase;

pub async fn run(live: bool) -> anyhow::Result<()> {
    if live {
        println!("Live dashboard not yet available. Showing static status.\n");
    }

    println!("\x1b[1mPhantom Status\x1b[0m\n");

    // System info
    println!("\x1b[1;34mSystem\x1b[0m");
    println!("  Status:        \x1b[33midle\x1b[0m");
    println!("  License:       not activated");
    println!("  Master key:    not initialized");
    println!();

    // Pipeline phases
    println!("\x1b[1;34mBuild Pipeline\x1b[0m");
    let phases = [
        BuildPhase::Ingest,
        BuildPhase::Infrastructure,
        BuildPhase::Architecture,
        BuildPhase::Code,
        BuildPhase::Test,
        BuildPhase::Security,
        BuildPhase::Deploy,
        BuildPhase::Deliver,
    ];
    for phase in &phases {
        println!("  {:<18} \x1b[90m--\x1b[0m", format!("{}", phase));
    }
    println!();

    // Agent status
    println!("\x1b[1;34mAgents\x1b[0m");
    for role in phantom_ai::ALL_ROLES {
        println!("  {:<20} \x1b[90midle\x1b[0m", role.display_name());
    }
    println!();

    // Infrastructure
    println!("\x1b[1;34mInfrastructure\x1b[0m");
    println!("  Providers:     0/14 authenticated");
    println!("  Resources:     0 provisioned");
    println!("  Mesh peers:    0 connected");
    println!();

    // Storage
    println!("\x1b[1;34mStorage\x1b[0m");
    println!("  Vault entries: 0");
    println!("  R2 blobs:      0");
    println!("  State keys:    0");

    Ok(())
}
