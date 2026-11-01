package probe;
import sbt.testing.*;
/** A test framework whose suite returns a task of a definition whose fingerprint's `toString()` throws: the worker makes
 * its debug line of the definition before it executes the task (ForkTestMain.runTest), so the task throws before it
 * runs ({@code CHILD RAN} never printed). */
public final class DebugAudit implements sbt.testing.Framework {
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
        if (child) { System.out.println("CHILD RAN"); return new Task[0]; }
        SubclassFingerprint broken = new SubclassFingerprint() {
          public boolean isModule() { return false; }
          public String superclassName() { return "probe.Suite"; }
          public boolean requireNoArgConstructor() { return true; }
          public String toString() { throw new IllegalStateException("debug-read-error"); }
        };
        return new Task[]{task(new TaskDef("probe.Child", broken, false, new Selector[]{new SuiteSelector()}), true)};
      }
    };
  }
}
