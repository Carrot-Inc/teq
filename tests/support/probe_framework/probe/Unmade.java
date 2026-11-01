package probe;

import sbt.testing.Fingerprint;

/** A test framework that cannot be made: its constructor throws, which ends the run. */
public final class Unmade implements sbt.testing.Framework {
    public Unmade() {
        throw new IllegalStateException("unmade");
    }

    public String name() {
        return "unmade";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[0];
    }

    public sbt.testing.Runner runner(String[] args, String[] remoteArgs, ClassLoader loader) {
        throw new IllegalStateException("never made");
    }
}
