package probe;
import sbt.testing.*;
/** A test framework whose runner's `tasks` of a suite throws {@link BrokenReport}'s throwable, whose `toString()` throws:
 * reporting the run's error throws, which the worker writes as an internal error past sbt's host, which made the runner
 * (ForkTestMain.Run.run, internalError); its suites are the subclasses of {@link BrokenSuite}. */
public final class BrokenTasks implements sbt.testing.Framework {
  static final SubclassFingerprint PRINT = new SubclassFingerprint() {
    public boolean isModule() { return false; }
    public String superclassName() { return "probe.BrokenSuite"; }
    public boolean requireNoArgConstructor() { return true; }
  };
  public String name() { return "broken-tasks"; }
  public Fingerprint[] fingerprints() { return new Fingerprint[] { PRINT }; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    return new Runner() {
      public String[] args() { return args; }
      public String[] remoteArgs() { return remote; }
      public String done() { return ""; }
      public Task[] tasks(TaskDef[] defs) {
        if (defs.length > 0) throw new BrokenReport.Broken();
        return new Task[0];
      }
    };
  }
}
