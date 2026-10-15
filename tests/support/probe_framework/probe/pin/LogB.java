package probe.pin;

import probe.Context;
import probe.Suite;

public final class LogB extends Suite {
    public void run(Context c) throws Exception {
        c.await("first");
        c.info("B between");
        c.signal("second");
        c.pass("case");
    }
}
