use crate::compare::ComparisonItem;
use crate::metrics::{aggregate_metrics, ValidationMetrics};
use chrono::Local;

/// Gerador de relatório de validação
pub struct ReportGenerator;

impl ReportGenerator {
    /// Gera relatório em HTML
    pub fn generate_html(&self, items: &[ComparisonItem], title: &str) -> String {
        let metrics = aggregate_metrics(items);
        let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let summary_color = if metrics.pass_rate >= 0.95 { "green" }
            else if metrics.pass_rate >= 0.8 { "orange" }
            else { "red" };

        let mut html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>B.A.S.E. — {}</title>
<style>
body {{ font-family: 'JetBrains Mono', monospace; max-width: 960px; margin: 2em auto; padding: 0 1em; background: #1a1a2e; color: #e0e0e0; }}
h1 {{ color: #4a9eff; border-bottom: 2px solid #4a9eff; }}
h2 {{ color: #69db7c; }}
.summary {{ background: #16213e; padding: 1em; border-radius: 8px; margin: 1em 0; }}
.pass {{ color: #69db7c; }}
.fail {{ color: #ff6b6b; }}
.warn {{ color: #ffd43b; }}
table {{ width: 100%; border-collapse: collapse; margin: 1em 0; }}
th, td {{ text-align: left; padding: 8px; border-bottom: 1px solid #333; }}
th {{ background: #16213e; }}
tr:hover {{ background: #0f3460; }}
.metric {{ display: inline-block; margin: 0.5em; padding: 0.5em 1em; background: #16213e; border-radius: 4px; }}
</style></head><body>
<h1>🔬 B.A.S.E. Validation Report</h1>
<p><strong>{}</strong> | {}</p>
<div class="summary">
<h2 style="color: {}">Summary — {:.1}% pass rate</h2>
<div class="metric">✅ {} passed</div>
<div class="metric">❌ {} failed</div>
<div class="metric">📊 {} total</div>
<div class="metric">⏱ {:.2}x avg latency</div>
<div class="metric">🎯 {:.1}% value accuracy</div>
</div>
"#, title, title, date, summary_color, metrics.pass_rate * 100.0,
              metrics.passed, metrics.failed, metrics.total_operations,
              metrics.avg_latency_ratio, metrics.value_accuracy * 100.0);

        // Warnings
        if !metrics.warnings.is_empty() {
            html.push_str("<h2>⚠️ Warnings</h2><ul>");
            for w in &metrics.warnings {
                html.push_str(&format!("<li class=\"warn\">{}</li>", w));
            }
            html.push_str("</ul>");
        }

        // SVG Chart: pass/fail pie chart
        let passed_deg = (metrics.pass_rate * 360.0) as f64;
        let failed_deg = 360.0 - passed_deg;
        let red_hex = "#ff6b6b";
        let green_hex = "#69db7c";
        let text_hex = "#e0e0e0";
        let sub_hex = "#888888";
        let chart_svg = format!(r#"<svg width="200" height="200" viewBox="0 0 200 200">
  <circle cx="100" cy="100" r="90" fill="none" stroke="{}" stroke-width="30"/>
  <circle cx="100" cy="100" r="90" fill="none" stroke="{}" stroke-width="30"
    stroke-dasharray="{:.0} {:.0}" stroke-dashoffset="0"
    transform="rotate(-90, 100, 100)"/>
  <text x="100" y="95" text-anchor="middle" fill="{}" font-size="24">{:.1}%</text>
  <text x="100" y="115" text-anchor="middle" fill="{}" font-size="12">pass rate</text>
</svg>"#, red_hex, green_hex, passed_deg, failed_deg, text_hex, metrics.pass_rate * 100.0, sub_hex);
        html.push_str("<h2>📊 Charts</h2><div style=\"display:flex;gap:2em;\">");
        html.push_str("<div style=\"text-align:center;\">");
        html.push_str(&chart_svg);
        html.push_str("</div>");

        // Latency histogram as SVG bars
        let latencies: Vec<f64> = items.iter().map(|i| i.latency_ratio).collect();
        let max_lat = latencies.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        let bar_width = 400.0 / items.len().max(1) as f64;
        html.push_str("<div><h3>Latency Ratio</h3><svg width=\"420\" height=\"120\">");
        for (i, &lat) in latencies.iter().enumerate() {
            let bar_h = (lat / max_lat * 80.0).min(80.0);
            let x = 10.0 + i as f64 * bar_width;
            let color = if lat <= 1.5 { "green" } else if lat <= 2.0 { "gold" } else { "red" };
            html.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
                x, 100.0 - bar_h, bar_width.max(1.0), bar_h, color
            ));
        }
        html.push_str("</svg></div></div>");

        // Failures table
        let failed_items: Vec<&ComparisonItem> = items.iter().filter(|i| !i.passed).collect();
        if !failed_items.is_empty() {
            html.push_str("<h2>❌ Failures</h2><table><tr><th>#</th><th>Type</th><th>Address</th><th>Failures</th></tr>");
            for item in &failed_items {
                html.push_str(&format!(
                    "<tr><td>{}</td><td>{:?}</td><td>0x{:08x}</td><td class=\"fail\">{}</td></tr>",
                    item.operation_id,
                    item.original_event.event_type,
                    item.original_event.address,
                    item.failures.join(", "),
                ));
            }
            html.push_str("</table>");
        }

        html.push_str("</body></html>");
        html
    }

    /// Gera relatório em JSON
    pub fn generate_json(&self, items: &[ComparisonItem], title: &str) -> String {
        let metrics = aggregate_metrics(items);
        let report = serde_json::json!({
            "report": {
                "title": title,
                "timestamp": Local::now().to_rfc3339(),
                "metrics": {
                    "total_operations": metrics.total_operations,
                    "passed": metrics.passed,
                    "failed": metrics.failed,
                    "pass_rate": metrics.pass_rate,
                    "avg_latency_ratio": metrics.avg_latency_ratio,
                    "value_accuracy": metrics.value_accuracy,
                    "address_accuracy": metrics.address_accuracy,
                },
                "warnings": metrics.warnings,
                "failures": items.iter().filter(|i: &&ComparisonItem| !i.passed).map(|i| {
                    serde_json::json!({
                        "operation_id": i.operation_id,
                        "event_type": format!("{:?}", i.original_event.event_type),
                        "address": format!("0x{:08x}", i.original_event.address),
                        "failures": i.failures,
                    })
                }).collect::<Vec<_>>(),
            }
        });
        serde_json::to_string_pretty(&report).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::{EventType, TraceEvent};

    fn mock_items() -> Vec<ComparisonItem> {
        vec![
            ComparisonItem {
                operation_id: 0,
                original_event: TraceEvent { timestamp_ns: 1000, channel: "CH0".into(), event_type: EventType::MmioWrite, address: 0x10000000, value: Some(1) },
                actual_event: None,
                latency_ratio: 1.0,
                value_match: true,
                address_match: true,
                passed: true,
                failures: vec![],
            },
            ComparisonItem {
                operation_id: 1,
                original_event: TraceEvent { timestamp_ns: 2000, channel: "CH0".into(), event_type: EventType::MmioRead, address: 0x10000004, value: None },
                actual_event: None,
                latency_ratio: 3.5,
                value_match: false,
                address_match: false,
                passed: false,
                failures: vec!["ADDRESS_MISMATCH".into(), "TIMING_VIOLATION".into()],
            },
        ]
    }

    #[test]
    fn test_html_report() {
        let gen = ReportGenerator;
        let items = mock_items();
        let html = gen.generate_html(&items, "GPU Validation");
        assert!(html.contains("B.A.S.E."), "Should have title");
        assert!(html.contains("50.0%"), "Should show pass rate");
        assert!(html.contains("ADDRESS_MISMATCH"), "Should show failure");
    }

    #[test]
    fn test_json_report() {
        let gen = ReportGenerator;
        let items = mock_items();
        let json = gen.generate_json(&items, "GPU Validation");
        assert!(json.contains("pass_rate"), "Should have metrics");
        assert!(json.contains("failures"), "Should have failures list");
    }
}
