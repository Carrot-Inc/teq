package probe;

import sbt.testing.*;

/** A test framework whose fingerprint cannot be shown (its `toString()` throws), which sbt's discovery shows in a debug
 * line it makes whether shown or not (Tests.discover): the run ends before any suite runs. */
public final class Unshown implements sbt.testing.Framework {
    public String name() {
        return "unshown";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[] { new SubclassFingerprint() {
            public boolean isModule() {
                return false;
            }

            public String superclassName() {
                return "probe.Suite";
            }

            public boolean requireNoArgConstructor() {
                return true;
            }

            @Override
            public String toString() {
                throw new IllegalStateException("shown-error");
            }
        } };
    }

    public sbt.testing.Runner runner(String[] args, String[] remote, ClassLoader loader) {
        throw new IllegalStateException("never made");
    }
}
