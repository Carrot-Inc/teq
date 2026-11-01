package probe;
import sbt.testing.*;
/** A test framework whose suite returns a task that throws, of a definition whose fingerprint's
 * `requireNoArgConstructor()` throws: the worker reads it making the task's Error event (ForkTestMain.testError,
 * testEvent, ForkEvent), so the task's future fails and no event of it is sent. */
public final class Audit implements sbt.testing.Framework {
  static final SubclassFingerprint PRINT = new SubclassFingerprint() {
    public boolean isModule() { return false; }
    public String superclassName() { return "probe.Suite"; }
    public boolean requireNoArgConstructor() { return true; }
  };
  public String name() { return "audit"; }
  public Fingerprint[] fingerprints() { return new Fingerprint[] { PRINT }; }
  public Runner runner(String[] args, String[] remote, ClassLoader loader) {
    return new Runner() {
      public String[] args() { return args; }
      public String[] remoteArgs() { return remote; }
      public String done() { return ""; }
      public Task[] tasks(TaskDef[] defs) {
        Task[] tasks = new Task[defs.length];
        for (int i=0;i<defs.length;i++) tasks[i] = task(defs[i], false);
        return tasks;
      }
    };
  }
  static Task task(TaskDef def, boolean child) {
    return new Task() {
      public String[] tags() { return new String[0]; }
      public TaskDef taskDef() { return def; }
      public Task[] execute(EventHandler handler, Logger[] logs) {
        if (child) throw new IllegalStateException("task-error");
        SubclassFingerprint broken = new SubclassFingerprint() {
          public boolean isModule() { return false; }
          public String superclassName() { return "probe.Suite"; }
          public boolean requireNoArgConstructor() { throw new IllegalStateException("fingerprint-read-error"); }
        };
        return new Task[]{task(new TaskDef("probe.Child", broken, false, new Selector[]{new SuiteSelector()}), true)};
      }
    };
  }
}
