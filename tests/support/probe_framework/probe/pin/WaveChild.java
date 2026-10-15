package probe.pin;

import probe.Context;
import probe.Suite;

public final class WaveChild extends Suite {
    public void run(Context c) {
        c.out("exec probe.pin.WaveChild");
        c.pass("case");
    }
}
