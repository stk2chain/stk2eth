use crate::eth::tx::{eth_tx, EthTx, TxStatus};
use crate::ussd::intent::{parse_intent, UserIntent, ConfirmDecision};
use crate::ussd::session::USSDSession;
use spacetimedb::Table;
use spacetimedb::ReducerContext;

pub fn cancel_tx(ctx: &ReducerContext, session: USSDSession) -> Result<USSDSession, String> {
    let intent = parse_intent("CancelTXScreen", &session.data)?;
    let confirm = match intent {
        UserIntent::SendEthConfirm { confirm, .. } => confirm,
        _ => return Err("expected SendEthConfirm intent".to_string()),
    };

    let pending = ctx.db.eth_tx().iter()
        .filter(|t| t.session_id == session.session_id && matches!(t.status, TxStatus::Pending))
        .next()
        .ok_or_else(|| format!("No pending eth_tx for session {}", session.session_id))?;

    let new_status = match confirm {
        ConfirmDecision::Confirm => TxStatus::Submitted,
        ConfirmDecision::Cancel  => TxStatus::Cancelled,
    };

    ctx.db.eth_tx().id().update(EthTx {
        status: new_status.clone(),
        updated_at: ctx.timestamp,
        ..pending
    });

    log::info!(
        "cancel_tx: session {} -> eth_tx.status = {:?}",
        session.session_id, new_status
    );

    Ok(session)
}
