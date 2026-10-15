package probe;

import sbt.testing.Event;
import sbt.testing.EventHandler;
import sbt.testing.Fingerprint;
import sbt.testing.Logger;
import sbt.testing.OptionalThrowable;
import sbt.testing.Selector;
import sbt.testing.Status;
import sbt.testing.SuiteSelector;
import sbt.testing.Task;
import sbt.testing.TaskDef;

/**
 * A test framework whose fingerprint is the probe framework's ({@link Framework}): the worker gives it every suite of
 * that fingerprint too, each task printing `twin <suite>` and reporting a success.
 */
public final class Twin implements sbt.testing.Framework {
    public String name() {
        return "twin";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[] { Framework.PRINT };
    }

    public sbt.testing.Runner runner(String[] args, String[] remoteArgs, ClassLoader loader) {
        return new sbt.testing.Runner() {
            public String[] args() {
                return args;
            }

            public String[] remoteArgs() {
                return remoteArgs;
            }

            public Task[] tasks(TaskDef[] defs) {
                Task[] tasks = new Task[defs.length];
                for (int i = 0; i < defs.length; i++) {
                    TaskDef def = defs[i];
                    tasks[i] = new Task() {
                        public String[] tags() {
                            return new String[0];
                        }

                        public TaskDef taskDef() {
                            return def;
                        }

                        public Task[] execute(EventHandler handler, Logger[] loggers) {
                            System.out.println("twin " + def.fullyQualifiedName());
                            handler.handle(new Event() {
                                public String fullyQualifiedName() {
                                    return def.fullyQualifiedName();
                                }

                                public Fingerprint fingerprint() {
                                    return def.fingerprint();
                                }

                                public Selector selector() {
                                    return new SuiteSelector();
                                }

                                public Status status() {
                                    return Status.Success;
                                }

                                public OptionalThrowable throwable() {
                                    return new OptionalThrowable();
                                }

                                public long duration() {
                                    return 1;
                                }
                            });
                            return new Task[0];
                        }
                    };
                }
                return tasks;
            }

            public String done() {
                return "";
            }
        };
    }
}
