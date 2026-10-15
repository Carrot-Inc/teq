package dev.teq.runner;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.io.Serializable;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Queue;
import java.util.Set;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;

import sbt.testing.AnnotatedFingerprint;
import sbt.testing.Event;
import sbt.testing.EventHandler;
import sbt.testing.Fingerprint;
import sbt.testing.Framework;
import sbt.testing.Logger;
import sbt.testing.NestedSuiteSelector;
import sbt.testing.NestedTestSelector;
import sbt.testing.Selector;
import sbt.testing.Status;
import sbt.testing.SubclassFingerprint;
import sbt.testing.SuiteSelector;
import sbt.testing.Task;
import sbt.testing.TaskDef;
import sbt.testing.TestSelector;
import sbt.testing.TestWildcardSelector;

/**
 * The test runner of `teq test` over sbt's test interface, a JVM for each run as sbt's forked worker is, which the
 * daemon starts once the run before ends, so that a run inherits no thread, static or registration of another
 * (docs/TOOLING.md, "Tests."). Its own class path holds this class, test-interface, the frameworks and the
 * libraries, none of which a build rewrites; the class and resource directories, which a build does rewrite, are
 * read through a class loader made for the run, the run's threads' context loader.
 *
 * <p>A run is one of sbt 2.0.8's forked test worker (`ForkTestMain` of sbt's worker module, docs/TOOLING.md,
 * "Tests."): the frameworks one after another on one pool of the run's size, each given every suite whose
 * fingerprint one of its own matches (two frameworks of one fingerprint both running it), a framework's tasks
 * submitted as a wave, their results awaited in order and the tasks they returned the next wave, the runner's
 * `done()` once the last wave is drained, what it returns printed nowhere; a task's events sent together once it returns, a task
 * that throws reported as an error of its suite after its error is logged and its fork error sent; a framework that
 * fails to make its runner or its tasks, or whose done() throws, ending the run; the loggers forwarded as they are
 * called. The pool is made under the run's class loader, which its threads take as their context loader, and shut
 * down before the loader is closed.
 *
 * <pre>
 * java -cp &lt;this&gt;:&lt;jars&gt; dev.teq.runner.Runner &lt;token&gt;
 * </pre>
 * answers "java &lt;feature version&gt;", then reads commands on stdin, fields separated by tabs
 * (a tab, a line break and a backslash in a field escaped as \t, \n, \r and \\):
 * <pre>
 * framework &lt;class&gt; &lt;argument&gt;...   a framework with its arguments, each once
 * start                              answered by "fingerprint &lt;framework&gt; &lt;index&gt; subclass|annotated &lt;module&gt; &lt;name&gt;"
 *                                    per fingerprint, "missing &lt;framework&gt; &lt;why&gt;" per framework that does not load,
 *                                    then "ready"
 * run &lt;verbose&gt; &lt;ansi&gt; &lt;threads&gt;     then "path &lt;directory&gt;" and "suite &lt;framework&gt; &lt;fingerprint&gt; &lt;name&gt;" lines and
 *                                    "go": the suites run on a pool of the threads (auto: the processors this JVM
 *                                    counts), their output written and their events sent as they come, then
 *                                    "done &lt;ms&gt;"
 * ping                               answered by "pong"
 * quit
 * </pre>
 * An event is sent typed, for the daemon to count and report (src/task/events.rs): "event &lt;suite&gt; &lt;status
 * ordinal&gt; &lt;name&gt; &lt;selector&gt; &lt;a&gt; &lt;b&gt; &lt;ms&gt;", the selector suite, test, nestedSuite, nestedTest or wildcard
 * with its names in a and b; a fork error, which ends the run (what a task threw, before its event, what a
 * framework's logger traced, or what a framework threw making its runner or its tasks, or done() threw), is
 * "forkError". Before each, what it threw: "throwable &lt;toString&gt;", then
 * "cause &lt;toString&gt;" for each cause, each followed by a "frame &lt;class&gt; &lt;toString&gt;" per frame of its
 * stack; a task's events and
 * what they threw are written at once, so that no other line comes between.
 * Every line of the protocol it writes opens with the token, so that no output of a suite is
 * taken for one, and escapes its fields as the commands do; a suite's stdout and stderr are this
 * process's stdout, in the order written.
 */
public final class Runner {
    private final String token;
    private final PrintStream out;
    private final List<String> frameworks = new ArrayList<>();
    private final List<String[]> arguments = new ArrayList<>();
    private final List<Fingerprint[]> fingerprints = new ArrayList<>();
    private final List<Boolean> loaded = new ArrayList<>();

    private Runner(String token, PrintStream out) {
        this.token = token;
        this.out = out;
    }

    public static void main(String[] args) throws IOException {
        PrintStream out = new PrintStream(new FileOutputStream(FileDescriptor.out), true, StandardCharsets.UTF_8);
        System.setOut(out);
        System.setErr(out);
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        System.setIn(new ByteArrayInputStream(new byte[0]));
        Runner runner = new Runner(args[0], out);
        runner.say("java", Runtime.version().feature());
        String line;
        while ((line = in.readLine()) != null) {
            String[] fields = fields(line);
            switch (fields[0]) {
                case "framework" -> {
                    runner.frameworks.add(fields[1]);
                    runner.arguments.add(Arrays.copyOfRange(fields, 2, fields.length));
                }
                case "start" -> runner.start();
                case "run" -> runner.run(fields[1].equals("1"), fields[2].equals("1"), fields[3], in);
                case "ping" -> runner.say("pong");
                case "quit" -> System.exit(0);
                default -> runner.say("unknown", fields[0]);
            }
        }
        System.exit(0);
    }

    private void start() {
        for (int i = 0; i < frameworks.size(); i++) {
            try {
                Fingerprint[] prints = framework(i).fingerprints();
                for (int j = 0; j < prints.length; j++) {
                    if (prints[j] instanceof SubclassFingerprint s) {
                        say("fingerprint", i, j, "subclass", s.isModule(), s.superclassName());
                    } else if (prints[j] instanceof AnnotatedFingerprint a) {
                        say("fingerprint", i, j, "annotated", a.isModule(), a.annotationName());
                    }
                }
                fingerprints.add(prints);
                loaded.add(true);
            } catch (Throwable e) {
                fingerprints.add(new Fingerprint[0]);
                loaded.add(false);
                say("missing", i, e);
            }
        }
        say("ready");
    }

    /** Whether two fingerprints are of the same suites, as the worker matches them (ForkTestMain.Run.matches). */
    private static boolean matches(Fingerprint f1, Fingerprint f2) {
        if (f1 instanceof SubclassFingerprint s1 && f2 instanceof SubclassFingerprint s2) {
            return s1.isModule() == s2.isModule() && s1.superclassName().equals(s2.superclassName());
        } else if (f1 instanceof AnnotatedFingerprint a1 && f2 instanceof AnnotatedFingerprint a2) {
            return a1.isModule() == a2.isModule() && a1.annotationName().equals(a2.annotationName());
        }
        return false;
    }

    private Framework framework(int i) throws ReflectiveOperationException {
        return (Framework) Class.forName(frameworks.get(i)).getDeclaredConstructor().newInstance();
    }

    private void run(boolean verbose, boolean ansi, String threads, BufferedReader in) throws IOException {
        List<URL> urls = new ArrayList<>();
        List<String[]> suites = new ArrayList<>();
        String line;
        while ((line = in.readLine()) != null && !line.equals("go")) {
            String[] fields = fields(line);
            if (fields[0].equals("path")) {
                urls.add(new File(fields[1]).toURI().toURL());
            } else if (fields[0].equals("suite")) {
                suites.add(fields);
            }
        }
        long start = System.nanoTime();
        Events events = new Events();
        Logger[] loggers = { new Forwarded(verbose, ansi, events) };
        Thread thread = Thread.currentThread();
        ClassLoader context = thread.getContextClassLoader();
        try (URLClassLoader loader = new URLClassLoader(urls.toArray(new URL[0]), Runner.class.getClassLoader())) {
            thread.setContextClassLoader(loader);
            ExecutorService pool = pool(threads, loggers[0]);
            try {
                for (int i = 0; i < frameworks.size(); i++) {
                    // One that did not load at the start is named by the daemon and run by no one.
                    if (!loaded.get(i)) {
                        continue;
                    }
                    // Every suite whose fingerprint one of the framework's matches, in the order of its fingerprints
                    // (ForkTestMain.runTests): a suite of two frameworks' fingerprint is each one's.
                    LinkedHashSet<TaskDef> defs = new LinkedHashSet<>();
                    for (Fingerprint own : fingerprints.get(i)) {
                        for (String[] s : suites) {
                            Fingerprint print = fingerprints.get(Integer.parseInt(s[1]))[Integer.parseInt(s[2])];
                            if (matches(own, print)) {
                                defs.add(new TaskDef(s[3], print, false, new Selector[] { new SuiteSelector() }));
                            }
                        }
                    }
                    runFramework(i, new ArrayList<>(defs), loader, loggers, events, pool);
                }
            } catch (Throwable t) {
                // A framework that fails to make its runner or its tasks, or whose done() throws, ends the run, no
                // framework after it run (ForkTestMain.Run.run).
                loggers[0].error("Uncaught exception when running tests: " + t);
                events.forkError(t);
            } finally {
                // Its threads, whose context loader is the run's, end before the loader is closed.
                pool.shutdown();
                try {
                    pool.awaitTermination(Long.MAX_VALUE, TimeUnit.NANOSECONDS);
                } catch (InterruptedException e) {
                    thread.interrupt();
                }
            }
        } finally {
            thread.setContextClassLoader(context);
        }
        out.flush();
        say("done", (System.nanoTime() - start) / 1_000_000);
    }

    /** The run's pool, as the worker makes it (ForkTestMain.executorService): one thread, or a fixed pool of the
     * count, auto the processors this JVM counts now. */
    private static ExecutorService pool(String threads, Logger logger) {
        int n = threads.equals("auto") ? Runtime.getRuntime().availableProcessors() : Integer.parseInt(threads);
        if (n == 1) {
            logger.debug("Create a single-thread test executor");
            return Executors.newSingleThreadExecutor();
        }
        logger.debug("Create a test executor with a thread pool of " + n + " threads.");
        return Executors.newFixedThreadPool(n);
    }

    /** A framework's suites: its tasks run in waves on the pool, then its runner's done(), what it returns printed
     * nowhere, a hook calling done() when the JVM ends before it (ForkTestMain.runTests); what the framework throws
     * making its runner or its tasks, or done() throws, the caller's, which ends the run. */
    private void runFramework(int i, List<TaskDef> defs, ClassLoader loader, Logger[] loggers, Events events, ExecutorService pool) throws ReflectiveOperationException {
        sbt.testing.Runner runner = framework(i).runner(arguments.get(i), new String[0], loader);
        Task[] tasks = runner.tasks(defs.toArray(new TaskDef[0]));
        Thread doneOnShutdown = new Thread(runner::done);
        Runtime.getRuntime().addShutdownHook(doneOnShutdown);
        runTasks(pool, tasks, events, loggers);
        runner.done();
        Runtime.getRuntime().removeShutdownHook(doneOnShutdown);
    }

    /** A wave of tasks on the pool, their results awaited in the wave's order, then the tasks they returned as the
     * next wave (ForkTestMain.runTestTasks): a task's children wait for the whole of its wave. */
    private void runTasks(ExecutorService pool, Task[] tasks, Events events, Logger[] loggers) {
        while (tasks.length > 0) {
            List<Future<Task[]>> wave = new ArrayList<>();
            for (Task task : tasks) {
                wave.add(pool.submit(() -> runTask(task, events, loggers)));
            }
            List<Task> nested = new ArrayList<>();
            for (Future<Task[]> result : wave) {
                try {
                    nested.addAll(Arrays.asList(result.get()));
                } catch (Exception e) {
                    loggers[0].error("Failed to execute task " + result);
                }
            }
            tasks = nested.toArray(new Task[0]);
        }
    }

    /** A task, its events sent together once it returns (ForkTestMain.runTest); one that throws has its error
     * logged and is an error of its suite, with no duration, the events it sent dropped. Sending them is outside
     * the task's catch, as the worker's writeEvents is: one the worker could not write fails the task's future. */
    private Task[] runTask(Task task, Events events, Logger[] loggers) {
        String suite = task.taskDef().fullyQualifiedName();
        Queue<Object> sent = new ConcurrentLinkedQueue<>();
        Task[] nested;
        List<Object> batch;
        try {
            nested = task.execute(event -> sent.add(events.event(suite, event)), loggers);
            // The worker reads the tasks returned here: none at all is a task that threw.
            if (nested == null) {
                throw new NullPointerException("the task of " + suite + " returned no array of tasks");
            }
            batch = new ArrayList<>(sent);
        } catch (Throwable t) {
            nested = new Task[0];
            loggers[0].error("Uncaught exception when running " + suite + ": " + t);
            events.forkError(t);
            batch = List.of((Object) events.error(suite, t));
        }
        events.send(batch);
        return nested;
    }

    /** An event of a selector the worker's gson cannot write, of the selector's class. */
    private record Unsendable(String type) {
    }

    /** The lines of a run's events, each with what it threw before it, made as the event comes. */
    private final class Events {
        /** An event checked as the worker's `ForkEvent` checks it before it is buffered, an event it refuses thrown
         * to the framework, which a task that lets it through then throws (the throwing task's path): a fingerprint
         * of neither kind, or a selector that is not `Serializable` (ForkTestMain.ForkEvent). One of a selector
         * of a kind gson's adapter does not know (WorkerMain.mkGson) is taken, as the worker takes it, to fail when
         * its task's events are sent. */
        Object event(String suite, Event event) {
            Fingerprint print = event.fingerprint();
            if (print instanceof SubclassFingerprint s) {
                s.isModule();
                s.superclassName();
                s.requireNoArgConstructor();
            } else {
                AnnotatedFingerprint a = (AnnotatedFingerprint) print;
                a.isModule();
                a.annotationName();
            }
            Selector selector = event.selector();
            if (!(selector instanceof Serializable)) {
                throw new UnsupportedOperationException("Selector implementation must be Serializable, but " + selector.getClass().getName() + " is not.");
            }
            String kind;
            String a = "";
            String b = "";
            if (selector.getClass() == SuiteSelector.class) {
                kind = "suite";
            } else if (selector.getClass() == TestSelector.class) {
                kind = "test";
                a = ((TestSelector) selector).testName();
            } else if (selector.getClass() == NestedTestSelector.class) {
                kind = "nestedTest";
                a = ((NestedTestSelector) selector).suiteId();
                b = ((NestedTestSelector) selector).testName();
            } else if (selector.getClass() == NestedSuiteSelector.class) {
                kind = "nestedSuite";
                a = ((NestedSuiteSelector) selector).suiteId();
            } else if (selector.getClass() == TestWildcardSelector.class) {
                kind = "wildcard";
                a = ((TestWildcardSelector) selector).testWildcard();
            } else {
                return new Unsendable(selector.getClass().getName());
            }
            String thrown = thrown(event.throwable().isDefined() ? event.throwable().get() : null);
            return thrown + line("event", suite, event.status().ordinal(), event.fullyQualifiedName(), kind, a, b, event.duration());
        }

        /** A task that threw: an error of its suite, as the worker makes it (ForkTestMain.testError). */
        String error(String suite, Throwable t) {
            return thrown(t) + line("event", suite, Status.Error.ordinal(), suite, "suite", "", "", 0);
        }

        /** A task's lines, written at once. */
        void send(List<Object> batch) {
            StringBuilder text = new StringBuilder();
            for (Object e : batch) {
                if (e instanceof Unsendable u) {
                    // gson's RuntimeTypeAdapterFactory, writing the worker's batch, refuses the selector's class.
                    throw new UnsupportedOperationException("cannot serialize " + u.type() + "; did you forget to register a subtype?");
                }
                text.append(e);
            }
            if (!batch.isEmpty()) {
                out.print(text);
            }
        }

        /** What the worker sends as a fork error (ForkTestMain.writeError): what a task threw, or what a framework's
         * logger traced, which ends the run as sbt's ForkTests fails its harness. */
        void forkError(Throwable t) {
            out.print(thrown(t) + line("forkError"));
        }

        /** The throwable, its causes, and the frames of each; a cause met again ends the chain. */
        private String thrown(Throwable e) {
            if (e == null) {
                return "";
            }
            StringBuilder lines = new StringBuilder();
            Set<Throwable> seen = Collections.newSetFromMap(new IdentityHashMap<>());
            String kind = "throwable";
            for (Throwable t = e; t != null && seen.add(t); t = t.getCause()) {
                lines.append(line(kind, t));
                for (StackTraceElement frame : t.getStackTrace()) {
                    lines.append(line("frame", frame.getClassName(), frame));
                }
                kind = "cause";
            }
            return lines.toString();
        }
    }

    /** A framework's logger: info, warnings and errors always, debug when verbose, and a trace sent as the worker's
     * remote logger sends it (ForkTestMain.remoteLogger), a fork error. */
    private final class Forwarded implements Logger {
        private final boolean verbose;
        private final boolean ansi;
        private final Events events;

        Forwarded(boolean verbose, boolean ansi, Events events) {
            this.verbose = verbose;
            this.ansi = ansi;
            this.events = events;
        }

        @Override
        public boolean ansiCodesSupported() {
            return ansi;
        }

        @Override
        public void error(String message) {
            print("[error] ", message);
        }

        @Override
        public void warn(String message) {
            print("[warn] ", message);
        }

        @Override
        public void info(String message) {
            print("", message);
        }

        @Override
        public void debug(String message) {
            if (verbose) {
                print("[debug] ", message);
            }
        }

        @Override
        public void trace(Throwable t) {
            events.forkError(t);
        }

        /** The message's lines written at once. */
        private void print(String prefix, String message) {
            StringBuilder lines = new StringBuilder();
            message.lines().forEach(line -> lines.append(prefix).append(line).append('\n'));
            out.print(lines);
        }
    }

    private void say(Object... fields) {
        out.print(line(fields));
    }

    /** A line of the protocol, its fields escaped. */
    private String line(Object... fields) {
        StringBuilder line = new StringBuilder(token);
        for (Object field : fields) {
            line.append('\t');
            String text = String.valueOf(field);
            for (int i = 0; i < text.length(); i++) {
                char c = text.charAt(i);
                switch (c) {
                    case '\\' -> line.append("\\\\");
                    case '\t' -> line.append("\\t");
                    case '\n' -> line.append("\\n");
                    case '\r' -> line.append("\\r");
                    default -> line.append(c);
                }
            }
        }
        return line.append('\n').toString();
    }

    private static String[] fields(String line) {
        String[] fields = line.split("\t", -1);
        for (int i = 0; i < fields.length; i++) {
            fields[i] = unescape(fields[i]);
        }
        return fields;
    }

    private static String unescape(String field) {
        if (field.indexOf('\\') < 0) {
            return field;
        }
        StringBuilder out = new StringBuilder(field.length());
        for (int i = 0; i < field.length(); i++) {
            char c = field.charAt(i);
            if (c == '\\' && i + 1 < field.length()) {
                char next = field.charAt(++i);
                out.append(next == 't' ? '\t' : next == 'n' ? '\n' : next == 'r' ? '\r' : next);
            } else {
                out.append(c);
            }
        }
        return out.toString();
    }
}
