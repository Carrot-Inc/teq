package probe;

import sbt.testing.Fingerprint;

/** A test framework whose fingerprints cannot be had: its `fingerprints()` throws, which ends the run. */
public final class Unprinted implements sbt.testing.Framework {
    public String name() {
        return "unprinted";
    }

    public Fingerprint[] fingerprints() {
        throw new IllegalStateException("fingerprints-error");
    }

    public sbt.testing.Runner runner(String[] args, String[] remoteArgs, ClassLoader loader) {
        throw new IllegalStateException("never made");
    }
}
