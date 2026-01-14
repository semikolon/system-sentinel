# Report: Designing "Non-Stupid" System Monitoring

This report explores advanced algorithms and architectural patterns for resource monitoring that avoid self-contradiction, reduce alert fatigue, and distinguish between "normal work" and "system crisis."

## 1. State Stability: Hysteresis & Damping
The most common "stupid" behavior in monitoring is **flapping**—when an alert toggles ON/OFF repeatedly because a value is hovering exactly on the threshold (e.g., 89.9% vs 90.1%).

### Hysteresis (Sticky States)
Instead of one threshold, use two.
- **Trigger**: Alert enters `CRITICAL` at **95%**.
- **Clear**: Alert only returns to `OK` when the value drops below **85%**.
- **Result**: Once the system is "sick," it must prove it's "healthy" before we stop worrying.

### Damping (Temporal Persistence)
Ignore "blips" like opening a complex app.
- **Algorithm**: $N$-of-$M$ (e.g., must be in breach for 4 out of the last 5 checks).
- **Result**: Short spikes are ignored; only sustained pressure triggers a notification.

---

## 2. Robust Statistics: Beyond Simple Averages
System metrics are rarely "normal" (Gaussian). They are spiky and skewed.

### Median Absolute Deviation (MAD)
Standard Deviation ($\sigma$) is heavily distorted by single outliers (like a massive 10GB growth spike).
- **Algorithm**: Calculate the median and then the median of the differences from that median.
- **Result**: MAD is much more "robust"—it defines what is *truly* unusual for YOUR specific system better than a hard-coded Z-score.

### Percentile-based (Quantile) Baselines
Instead of saying "alert at 90%", alert when usage exceeds the **98th percentile** of your own history for the last 24 hours.
- **Result**: The system automatically learns that 13GB swap is "normal" for you and won't bug you unless it hits 15GB.

---

## 3. High-Fidelity Correlation (Context-Awareness)
A "stupid" monitor looks at metrics in silos. An "intelligent" monitor correlates them.

| Metric A | Metric B | Intelligent Conclusion |
| :--- | :--- | :--- |
| **Memory Growth ↑** | **Swap Usage Low** | **Normal Work**: Likely caching or a new JVM starting. Ignore. |
| **Memory Growth ↑** | **Swap Usage ↑** | **Leak Detected**: System is forced to move data to disk. Alert! |
| **High CPU ↑** | **High I/O Wait ↑** | **Disk Bottleneck**: Don't tell the user to "kill processes"; tell them the "SSD is saturated". |
| **High Swap ↑** | **Green Pressure** | **Safe State**: macOS is managing memory efficiently. No alert. |

---

## 4. Alert Inhibition & Hierarchy
Prevent the "notification bomb" when one failure causes ten others.

- **Inhibition**: If a `CRITICAL` alert for Memory is already active, automatically **mute** any incoming `WARNING` alerts for Swap or Growth.
- **Hierarchy**: Group alerts by "Subsystem". Only the most severe alert in a group should be shown.

---

## 5. Proposed Roadmap for System Sentinel "IQ"
To implement these without adding heavy ML libraries:

1. **Phase 1: Hysteresis Implementation** (Low effort, high impact).
2. **Phase 2: Persistence Counters** (Damping) for all alerts.
3. **Phase 3: Condition-based Inhibition** (e.g., Growth needs high memory usage to fire).
4. **Phase 4: Percentile Baselines** (Storing 24h of history to auto-tune thresholds).
