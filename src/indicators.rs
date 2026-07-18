//! Simple Wilder RSI helpers for the teaching bot example.

use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsiSignal {
    pub latest_rsi: Option<Decimal>,
    pub previous_rsi: Option<Decimal>,
    pub action: String,
    pub reason: String,
}

/// Return Wilder RSI values aligned to the input close series.
pub fn calculate_rsi(closes: &[Decimal], period: usize) -> anyhow::Result<Vec<Option<Decimal>>> {
    if period == 0 {
        anyhow::bail!("period must be positive");
    }
    let mut values = vec![None; closes.len()];
    if closes.len() < period + 1 {
        return Ok(values);
    }

    let mut gains = Vec::with_capacity(period);
    let mut losses = Vec::with_capacity(period);
    for index in 1..=period {
        let change = closes[index] - closes[index - 1];
        gains.push(change.max(Decimal::ZERO));
        losses.push((-change).max(Decimal::ZERO));
    }

    let period_dec = Decimal::from(period);
    let mut avg_gain: Decimal = gains.iter().copied().sum::<Decimal>() / period_dec;
    let mut avg_loss: Decimal = losses.iter().copied().sum::<Decimal>() / period_dec;
    values[period] = Some(rsi_from_averages(avg_gain, avg_loss));

    for index in (period + 1)..closes.len() {
        let change = closes[index] - closes[index - 1];
        let gain = change.max(Decimal::ZERO);
        let loss = (-change).max(Decimal::ZERO);
        avg_gain = ((avg_gain * Decimal::from(period - 1)) + gain) / period_dec;
        avg_loss = ((avg_loss * Decimal::from(period - 1)) + loss) / period_dec;
        values[index] = Some(rsi_from_averages(avg_gain, avg_loss));
    }
    Ok(values)
}

/// Evaluate threshold crossings on the latest RSI values.
pub fn rsi_signal(
    closes: &[Decimal],
    period: usize,
    oversold: Decimal,
    overbought: Decimal,
) -> anyhow::Result<RsiSignal> {
    let values = calculate_rsi(closes, period)?;
    let latest = latest_non_none(&values);
    let previous = previous_non_none(&values);
    if latest.is_none() || previous.is_none() {
        return Ok(RsiSignal {
            latest_rsi: latest,
            previous_rsi: previous,
            action: "hold".into(),
            reason: "not enough candle history".into(),
        });
    }
    let latest_v = latest.unwrap();
    let previous_v = previous.unwrap();
    if previous_v <= oversold && latest_v > oversold {
        return Ok(RsiSignal {
            latest_rsi: latest,
            previous_rsi: previous,
            action: "buy".into(),
            reason: "RSI crossed up out of oversold".into(),
        });
    }
    if previous_v >= overbought && latest_v < overbought {
        return Ok(RsiSignal {
            latest_rsi: latest,
            previous_rsi: previous,
            action: "sell".into(),
            reason: "RSI crossed down out of overbought".into(),
        });
    }
    Ok(RsiSignal {
        latest_rsi: latest,
        previous_rsi: previous,
        action: "hold".into(),
        reason: "RSI has not crossed a threshold".into(),
    })
}

fn rsi_from_averages(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
    if avg_loss.is_zero() {
        if avg_gain.is_zero() {
            return Decimal::from(50);
        }
        return Decimal::from(100);
    }
    let relative_strength = avg_gain / avg_loss;
    Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + relative_strength))
}

fn latest_non_none(values: &[Option<Decimal>]) -> Option<Decimal> {
    values.iter().rev().flatten().copied().next()
}

fn previous_non_none(values: &[Option<Decimal>]) -> Option<Decimal> {
    let mut seen_latest = false;
    for value in values.iter().rev().flatten().copied() {
        if seen_latest {
            return Some(value);
        }
        seen_latest = true;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_needs_history() {
        let closes = vec![Decimal::from(100), Decimal::from(101), Decimal::from(102)];
        let signal = rsi_signal(&closes, 14, Decimal::from(30), Decimal::from(70)).unwrap();
        assert_eq!(signal.action, "hold");
    }

    #[test]
    fn rsi_computes_values() {
        let mut closes = Vec::new();
        for i in 0..30 {
            closes.push(Decimal::from(100 + i));
        }
        let values = calculate_rsi(&closes, 14).unwrap();
        assert!(values[14].is_some());
        assert!(values.last().unwrap().is_some());
    }
}
