package probe.pin;

import probe.Context;
import probe.Suite;

/** Two workers: LogA's first line, LogB's between, LogA's last, as the loggers are called. */
public final class LogA extends Suite {
    public void run(Context c) throws Exception {
        c.info("A begin");
        c.signal("first");
        c.await("second");
        c.info("A end");
        c.pass("case");
    }
}
