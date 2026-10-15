package probe;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.atomic.AtomicInteger;

import sbt.testing.EventHandler;
import sbt.testing.Fingerprint;
import sbt.testing.Logger;
import sbt.testing.Selector;
import sbt.testing.SubclassFingerprint;
import sbt.testing.SuiteSelector;
import sbt.testing.Task;
import sbt.testing.TaskDef;

/**
 * A test framework over sbt's test interface for the fixtures of teq's test runner (tests/task.sh): a suite is a
 * subclass of {@link Suite}, loaded through the run's class loader and run by its task, which counts the tasks
 * running at once and the threads they ran on. A runner's first argument, when given, names a file each task
 * appends its start and end to ({@code start <suite> <thread>}, {@code end <suite> <thread>}), and done() the most
 * tasks at once and the threads ({@code done <most> <threads>}); done() prints a line, releases the resources the
 * suites acquired, and returns a summary.
 * The state of a run is its runner's: barriers and latches by name, met by the suites of one run alone.
 */
public final class Framework implements sbt.testing.Framework {
    static final SubclassFingerprint PRINT = new SubclassFingerprint() {
        public boolean isModule() {
            return false;
        }

        public String superclassName() {
            return "probe.Suite";
        }

        public boolean requireNoArgConstructor() {
            return true;
        }
    };

    public String name() {
        return "probe";
    }

    public Fingerprint[] fingerprints() {
        return new Fingerprint[] { PRINT };
    }

    public sbt.testing.Runner runner(String[] args, String[] remoteArgs, ClassLoader loader) {
        return new Run(args, remoteArgs, loader);
    }

    static final class Run implements sbt.testing.Runner {
        final String[] args;
        final String[] remoteArgs;
        final ClassLoader loader;
        final ConcurrentHashMap<String, CyclicBarrier> barriers = new ConcurrentHashMap<>();
        final ConcurrentHashMap<String, CountDownLatch> latches = new ConcurrentHashMap<>();
        final AtomicInteger running = new AtomicInteger();
        final AtomicInteger most = new AtomicInteger();
        final Set<String> threads = ConcurrentHashMap.newKeySet();
        final java.util.Queue<java.nio.file.Path> resources = new java.util.concurrent.ConcurrentLinkedQueue<>();

        Run(String[] args, String[] remoteArgs, ClassLoader loader) {
            this.args = args;
            this.remoteArgs = remoteArgs;
            this.loader = loader;
        }

        public String[] args() {
            return args;
        }

        public String[] remoteArgs() {
            return remoteArgs;
        }

        public Task[] tasks(TaskDef[] defs) {
            Task[] tasks = new Task[defs.length];
            for (int i = 0; i < defs.length; i++) {
                tasks[i] = new Job(this, defs[i]);
            }
            return tasks;
        }

        public String done() {
            record("done " + most.get() + " " + threads.size());
            System.out.println("probe: done");
            for (java.nio.file.Path resource; (resource = resources.poll()) != null; ) {
                try {
                    Files.deleteIfExists(resource);
                } catch (IOException e) {
                    throw new UncheckedIOException(e);
                }
                System.out.println("probe: resource released");
            }
            return "probe: the summary done() returns";
        }

        synchronized void record(String line) {
            if (args.length == 0 || args[0].isEmpty()) {
                return;
            }
            try {
                Files.write(Paths.get(args[0]), (line + "\n").getBytes(StandardCharsets.UTF_8), StandardOpenOption.CREATE, StandardOpenOption.APPEND);
            } catch (IOException e) {
                throw new UncheckedIOException(e);
            }
        }
    }

    /** A suite's task, or a nested one: the suite named by its definition. */
    static final class Job implements Task {
        final Run run;
        final TaskDef def;

        Job(Run run, TaskDef def) {
            this.run = run;
            this.def = def;
        }

        static TaskDef of(String name) {
            return new TaskDef(name, PRINT, false, new Selector[] { new SuiteSelector() });
        }

        public String[] tags() {
            return new String[0];
        }

        public TaskDef taskDef() {
            return def;
        }

        public Task[] execute(EventHandler handler, Logger[] loggers) {
            String thread = Thread.currentThread().getName();
            run.most.accumulateAndGet(run.running.incrementAndGet(), Math::max);
            run.threads.add(thread);
            run.record("start " + def.fullyQualifiedName() + " " + thread);
            try {
                Suite suite = (Suite) Class.forName(def.fullyQualifiedName(), true, run.loader).getDeclaredConstructor().newInstance();
                Context context = new Context(run, def, handler, loggers);
                suite.run(context);
                return context.nested.toArray(new Task[0]);
            } catch (RuntimeException | Error e) {
                throw e;
            } catch (Exception e) {
                throw new RuntimeException(e);
            } finally {
                run.record("end " + def.fullyQualifiedName() + " " + thread);
                run.running.decrementAndGet();
            }
        }
    }
}
