package probe.pin;

import probe.Context;
import probe.Suite;

/** A task that reports a success, then throws. */
public final class Throw extends Suite {
    public void run(Context c) {
        c.pass("case");
        throw new RuntimeException("boom");
    }
}
