//! Prometheus text exposition (AC-E4).

use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Metrics {
    pub reconcile_total: u64,
    pub verify_pass: u64,
    pub verify_fail: u64,
}

impl Metrics {
    pub fn record_reconcile(&mut self) {
        self.reconcile_total += 1;
    }

    pub fn record_verify(&mut self, pass: bool) {
        if pass {
            self.verify_pass += 1;
        } else {
            self.verify_fail += 1;
        }
    }

    pub fn render_prometheus(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# HELP veridata_reconcile_total Reconcile operations completed"
        );
        let _ = writeln!(out, "# TYPE veridata_reconcile_total counter");
        let _ = writeln!(out, "veridata_reconcile_total {}", self.reconcile_total);
        let _ = writeln!(
            out,
            "# HELP veridata_verify_outcome_total Verify outcomes by result"
        );
        let _ = writeln!(out, "# TYPE veridata_verify_outcome_total counter");
        let _ = writeln!(
            out,
            "veridata_verify_outcome_total{{result=\"pass\"}} {}",
            self.verify_pass
        );
        let _ = writeln!(
            out,
            "veridata_verify_outcome_total{{result=\"fail\"}} {}",
            self.verify_fail
        );
        out
    }

    pub fn write_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.render_prometheus())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_e4_1_prometheus_format() {
        let mut m = Metrics::default();
        m.record_reconcile();
        m.record_verify(true);
        let text = m.render_prometheus();
        assert!(text.contains("veridata_reconcile_total 1"));
        assert!(text.contains("result=\"pass\"} 1"));
    }
}
