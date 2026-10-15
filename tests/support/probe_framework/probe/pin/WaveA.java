package probe.pin;

import probe.Context;
import probe.Suite;

/** Two workers: WaveA returns WaveChild, which runs once the whole wave is done, WaveB too, though a worker is free. */
public final class WaveA extends Suite {
    public void run(Context c) {
        c.out("exec probe.pin.WaveA");
        c.pass("case");
        c.nest("probe.pin.WaveChild");
        c.signal("a");
    }
}
