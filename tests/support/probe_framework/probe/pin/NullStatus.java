package probe.pin;

import probe.Context;
import probe.Suite;

/** An event of no status (`status()` null), then a success. */
public final class NullStatus extends Suite {
    public void run(Context c) {
        c.event("none", null, null);
        c.pass("case");
    }
}
