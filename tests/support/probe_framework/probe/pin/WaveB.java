package probe.pin;

import probe.Context;
import probe.Suite;

public final class WaveB extends Suite {
    public void run(Context c) throws Exception {
        c.await("a");
        Thread.sleep(300);
        c.out("exec probe.pin.WaveB ends");
        c.pass("case");
    }
}
