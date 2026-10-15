package probe.pin;

import probe.Context;
import probe.Suite;

public final class OrderChild extends Suite {
    public void run(Context c) {
        c.out("exec probe.pin.OrderChild");
        c.pass("case");
    }
}
