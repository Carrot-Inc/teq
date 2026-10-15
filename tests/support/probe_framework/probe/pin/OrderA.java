package probe.pin;

import probe.Context;
import probe.Suite;

/** One worker: its task, then OrderB's, then the task it returns, OrderChild's. */
public final class OrderA extends Suite {
    public void run(Context c) {
        c.out("exec probe.pin.OrderA");
        c.pass("case");
        c.nest("probe.pin.OrderChild");
    }
}
