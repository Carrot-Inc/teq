package probe;
import sbt.testing.*;
/** A test framework that says when its runner is made, runs a suite and is done ({@code idle: runner},
 * {@code idle: ran <suite>}, {@code idle: done}): sbt's host makes a runner of it, and gives it to its worker, only where
 * a suite of it (a subclass of {@link IdleSuite}) is selected (Defaults.allTestGroupsTask's filteredFrameworks). */
public final class Idle implements sbt.testing.Framework {
  static final SubclassFingerprint PRINT = new SubclassFingerprint() {
    public boolean isModule() { return false; }
    public String superclassName() { return "probe.IdleSuite"; }
    public boolean requireNoArgConstructor() { return true; }
  };
  public String name() { return "idle"; }
  public Fingerprint[] fingerprints() { return new Fingerprint[] { PRINT }; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    System.out.println("idle: runner");
    return new Runner() {
      public String[] args() { return args; }
      public String[] remoteArgs() { return remote; }
      public String done() { System.out.println("idle: done"); return ""; }
      public Task[] tasks(TaskDef[] defs) {
        Task[] tasks = new Task[defs.length];
        for (int i = 0; i < defs.length; i++) {
          TaskDef def = defs[i];
          tasks[i] = new Task() {
            public String[] tags() { return new String[0]; }
            public TaskDef taskDef() { return def; }
            public Task[] execute(EventHandler handler, Logger[] loggers) {
              System.out.println("idle: ran " + def.fullyQualifiedName());
              return new Task[0];
            }
          };
        }
        return tasks;
      }
    };
  }
}
