package probe;
import sbt.testing.*;
/** A test framework that replaces its fingerprint array's entry when its runner is made: sbt's
 * host takes the array's members when it discovers (TestFramework.getFingerprints' toList), so its definitions keep the
 * fingerprint discovered; its suites are the probe framework's, run by the probe framework's runner. */
public final class Mutating implements sbt.testing.Framework {
  private Fingerprint fp(String name) {
    return new SubclassFingerprint() {
      public boolean isModule() { return false; }
      public String superclassName() { return name; }
      public boolean requireNoArgConstructor() { return true; }
      public String toString() { return name; }
    };
  }
  private final Fingerprint[] prints = {fp("probe.Suite")};
  public String name() { return "mutating"; }
  public Fingerprint[] fingerprints() { return prints; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    prints[0] = fp("probe.Unrelated");
    return new probe.Framework().runner(args, remote, loader);
  }
}
