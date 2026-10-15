package probe;

import java.util.ArrayList;
import java.util.List;
import java.util.TreeSet;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

import sbt.testing.Event;
import sbt.testing.EventHandler;
import sbt.testing.Fingerprint;
import sbt.testing.Logger;
import sbt.testing.OptionalThrowable;
import sbt.testing.Selector;
import sbt.testing.Status;
import sbt.testing.Task;
import sbt.testing.TaskDef;
import sbt.testing.TestSelector;

/**
 * What a suite's task gives it: the loggers, the event handler, the run's barriers and latches (each wait bounded,
 * a wait that times out thrown), the tasks it returns, and the run's record.
 */
public final class Context {
    /** How long a suite waits for the others at a barrier or a latch. */
    static final long WAIT_SECONDS = 20;

    private final Framework.Run run;
    private final TaskDef def;
    private final EventHandler handler;
    private final Logger[] loggers;
    final List<Task> nested = new ArrayList<>();

    Context(Framework.Run run, TaskDef def, EventHandler handler, Logger[] loggers) {
        this.run = run;
        this.def = def;
        this.handler = handler;
        this.loggers = loggers;
    }

    public void info(String message) {
        for (Logger l : loggers) {
            l.info(message);
        }
    }

    public void error(String message) {
        for (Logger l : loggers) {
            l.error(message);
        }
    }

    /** A throwable given to the loggers' trace. */
    public void trace(Throwable t) {
        for (Logger l : loggers) {
            l.trace(t);
        }
    }

    /** A line on the suite's stdout. */
    public void out(String line) {
        System.out.println(line);
    }

    public void pass(String test) {
        event(test, Status.Success, null);
    }

    public void fail(String test, Throwable t) {
        event(test, Status.Failure, t);
    }

    public void event(String test, Status status, Throwable t) {
        event(new TestSelector(test), status, t);
    }

    /** An event of the selector given, whatever its kind. */
    public void event(Selector selector, Status status, Throwable t) {
        String name = def.fullyQualifiedName();
        Fingerprint print = def.fingerprint();
        OptionalThrowable thrown = t == null ? new OptionalThrowable() : new OptionalThrowable(t);
        handler.handle(new Event() {
            public String fullyQualifiedName() {
                return name;
            }

            public Fingerprint fingerprint() {
                return print;
            }

            public Selector selector() {
                return selector;
            }

            public Status status() {
                return status;
            }

            public OptionalThrowable throwable() {
                return thrown;
            }

            public long duration() {
                return 1;
            }
        });
    }

    /** Waits until `parties` suites of the run are at the barrier `name`, which then opens for the next ones. */
    public void arrive(String name, int parties) throws Exception {
        CyclicBarrier barrier = run.barriers.computeIfAbsent(name, k -> new CyclicBarrier(parties));
        try {
            barrier.await(WAIT_SECONDS, TimeUnit.SECONDS);
        } catch (TimeoutException e) {
            throw new IllegalStateException("fewer than " + parties + " suites at the barrier " + name + " at once");
        }
    }

    public void signal(String latch) {
        latch(latch).countDown();
    }

    public void await(String latch) throws InterruptedException {
        if (!latch(latch).await(WAIT_SECONDS, TimeUnit.SECONDS)) {
            throw new IllegalStateException("the latch " + latch + " was not signalled");
        }
    }

    private CountDownLatch latch(String name) {
        return run.latches.computeIfAbsent(name, k -> new CountDownLatch(1));
    }

    /** The suite named returned as a task of this one, run after this one's wave. */
    public void nest(String suite) {
        nested.add(new Framework.Job(run, Framework.Job.of(suite)));
    }

    /** A resource the framework's done() releases: a file beside the record (a temporary one without a record),
     * its path recorded. */
    public void resource() throws java.io.IOException {
        java.nio.file.Path file = run.args.length == 0 || run.args[0].isEmpty()
            ? java.nio.file.Files.createTempFile("probe", ".resource")
            : java.nio.file.Files.write(java.nio.file.Paths.get(run.args[0] + ".resource"), new byte[0]);
        run.resources.add(file);
        record("resource acquired");
    }

    /** A line of the run's record. */
    public void record(String line) {
        run.record(line);
    }

    /** The pools whose threads are alive (`pool-<n>` of the executors' `pool-<n>-thread-<m>`), and whether this
     * thread's context class loader is the one the run's runner was given. */
    public String pools() {
        TreeSet<String> pools = new TreeSet<>();
        for (Thread t : Thread.getAllStackTraces().keySet()) {
            String n = t.getName();
            if (n.startsWith("pool-") && n.indexOf("-thread-") > 0) {
                pools.add(n.substring(0, n.indexOf("-thread-")));
            }
        }
        boolean own = Thread.currentThread().getContextClassLoader() == run.loader;
        return pools.size() + " " + (own ? "own-loader" : "other-loader");
    }
}
