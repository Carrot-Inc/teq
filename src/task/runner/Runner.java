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
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
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
import sbt.testing.OptionalThrowable;
import sbt.testing.Selector;
import sbt.testing.Status;
import sbt.testing.SubclassFingerprint;
import sbt.testing.SuiteSelector;
import sbt.testing.Task;
import sbt.testing.TaskDef;
import sbt.testing.TestSelector;
import sbt.testing.TestWildcardSelector;

/**
 * The test runner of `teq test` over sbt's test interface, a JVM for each invocation as sbt's forked worker is, which
 * the daemon starts once the one before ends, so that a run inherits no thread, static or registration of another
 * (docs/TOOLING.md, "Tests."). Its own class path holds this class and test-interface alone (its two packages), as sbt
 * 2's worker in its virtual layout (`WorkerExchange.startWorker`, `WorkerMain.test`): every entry of the test class
 * path, the frameworks, the libraries, the class and resource directories, is read through one class loader made for
 * the invocation once its build is done, below this class's, in the class path's order, the frameworks loaded and
 * their fingerprints taken through it, the run's threads' context loader.
 *
 * <p>A run is one of sbt 2.0.8's forked test worker (`ForkTestMain` of sbt's worker module, docs/TOOLING.md,
 * "Tests."), its suites in ordered phases, the configuration's groups, as sbt runs its groups in process: in each
 * phase the frameworks one after another on one pool of the run's size, each given every suite of the phase whose
 * fingerprint one of its own matches (two frameworks of one fingerprint both running it), a framework's tasks
 * submitted as a wave, their results awaited in order and the tasks they returned the next wave, each runner's
 * `done()` once the last phase is drained, what it returns printed nowhere; a task's events sent together once it returns, a task
 * that throws reported as an error of its suite after its error is logged and its fork error sent; a framework that
 * fails to load (but for a class not found), to make its runner or its tasks, or whose done() throws, ending the run;
 * the loggers forwarded as they are called; a fork error ending the run after its phase. A framework of sbt's older interface (`org.scalatools.testing`) runs
 * through the worker's `FrameworkWrapper`. The pool is made under the run's class loader, which its threads take as
 * their context loader, and shut down before the loader is closed.
 *
 * <pre>
 * java -cp &lt;this&gt;:&lt;test-interface&gt; dev.teq.runner.Runner &lt;token&gt;
 * </pre>
 * answers "java &lt;feature version&gt;", then reads commands on stdin, fields separated by tabs
 * (a tab, a line break and a backslash in a field escaped as \t, \n, \r and \\):
 * <pre>
 * path &lt;entry&gt;                       an entry of the test class path, in its order
 * framework &lt;class&gt; &lt;argument&gt;...   a framework with its arguments, each once
 * load &lt;host&gt;                        the run's class loader made over the entries and the frameworks loaded through
 *                                    it (host 1: as sbt's host loads them, for an export of sbt's; 0: a build tool's
 *                                    of no host, Mill's), answered by "fingerprint &lt;framework&gt; &lt;index&gt;
 *                                    subclass|annotated &lt;module&gt; &lt;name&gt;" per fingerprint, "missing &lt;framework&gt;
 *                                    &lt;why&gt;" per framework whose class is not found, or the run's fork error where
 *                                    loading one throws otherwise, then "loaded"
 * run &lt;verbose&gt; &lt;ansi&gt; &lt;threads&gt; &lt;phases&gt;
 *                                    after load, then for sbt's host "def &lt;framework&gt; &lt;fingerprint&gt; &lt;name&gt;"
 *                                    lines, the definitions its selection starts from, and "filter &lt;position&gt;..."
 *                                    lines, its ordered filters, each the positions of the definitions it accepts;
 *                                    then "suite &lt;framework&gt; &lt;fingerprint&gt; &lt;phase&gt; &lt;explicit&gt; &lt;name&gt;" lines in the
 *                                    selection's order (explicit 1 for a suite named whole, its definition's
 *                                    explicitlySpecified)
 *                                    and "go": the phases one after another, each answered by "phase &lt;phase&gt;" as
 *                                    it is entered and "ended &lt;phase&gt; &lt;ms&gt;" once drained, its suites run on a
 *                                    pool of the threads (auto: the processors this JVM counts), their output written
 *                                    and their events sent as they come; then the frameworks' done() and
 *                                    "done &lt;ms&gt;"
 * ping                               answered by "pong"
 * quit
 * </pre>
 * An event is sent typed, for the daemon to count and report (src/task/events.rs): "event &lt;suite&gt; &lt;status
 * ordinal, -1 for none&gt; &lt;name&gt; &lt;selector&gt; &lt;a&gt; &lt;b&gt; &lt;ms&gt;", the selector suite, test, nestedSuite, nestedTest or wildcard
 * with its names in a and b; a fork error, which ends the run (what a task threw, before its event, what a
 * framework's logger traced, or what a framework threw loading, making its runner or its tasks, or done() threw), is
 * "forkError", as is what sbt's host throws, and what reporting such a framework's throw threw "internalError &lt;text&gt;",
 * which ends the run's phases, the worker then answering as it does at its end;
 * after a task's events, "ran &lt;suite&gt;", its suite run, a task that sent no event too. Before each, what it threw, copied
 * as the worker copies it (ForkError): "throwable &lt;class&gt;: &lt;message&gt;", then "cause &lt;class&gt;: &lt;message&gt;"
 * for each cause, each followed by a "frame &lt;class&gt; &lt;toString&gt;" per frame of its stack; a task's events and
 * what they threw are written at once, so that no other line comes between.
 * Every line of the protocol it writes opens with the token, so that no output of a suite is
 * taken for one, and escapes its fields as the commands do; a suite's stdout and stderr are this
 * process's stdout, in the order written.
 */
public final class Runner {
    private final String token;
    private final PrintStream out;
    private final List<URL> paths = new ArrayList<>();
    private final List<String> frameworks = new ArrayList<>();
    private final List<String[]> arguments = new ArrayList<>();
    /** Each framework as `load` made it, null where its class was not found: sbt's host's (`loadedTestFrameworks`),
     * the worker making its own, or, for a build tool of no host (Mill), the run's. */
    private final List<Framework> instances = new ArrayList<>();
    /** Whether the run plays sbt's host before and after its worker (an export of sbt's), which `load` is told. */
    private boolean host;
    /** Each framework's fingerprints as `load` asked them, its own objects, which the suites' copies are made of. */
    private final List<Fingerprint[]> fingerprints = new ArrayList<>();
    /** The run's class loader, which `load` makes, and the thread's context loader before it. */
    private URLClassLoader loader;
    private ClassLoader context;

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
        // What leaves the commands, the worker ends on (WorkerMain.main): its trace written, the JVM's exit 1, whatever
        // threads a framework left running.
        try {
            runner.say("java", Runtime.version().feature());
            String line;
            while ((line = in.readLine()) != null) {
                String[] fields = fields(line);
                switch (fields[0]) {
                    case "path" -> runner.paths.add(new File(fields[1]).toURI().toURL());
                    case "framework" -> {
                        runner.frameworks.add(fields[1]);
                        runner.arguments.add(Arrays.copyOfRange(fields, 2, fields.length));
                    }
                    case "load" -> runner.load(fields[1].equals("1"));
                    case "run" -> runner.run(fields[1].equals("1"), fields[2].equals("1"), fields[3], Integer.parseInt(fields[4]), in);
                    case "ping" -> runner.say("pong");
                    case "quit" -> System.exit(0);
                    default -> runner.say("unknown", fields[0]);
                }
            }
        } catch (Throwable e) {
            e.printStackTrace();
            System.exit(1);
        }
        System.exit(0);
    }

    /**
     * The run's class loader made over the test class path, below this class's loader, as the worker's
     * `WorkerMain.test` makes it in its virtual layout, the thread's context loader from here on
     * (`ForkTestMain.runTests`), and each framework loaded through it and made, as sbt's host makes them
     * (`loadedTestFrameworks`), then their fingerprints asked, reflectively for sbt's host
     * (`TestFramework.getFingerprints`), and read as sbt's host discovery reads them (`Tests.discover`): a class not
     * found leaves the framework out, said, and the next loaded; anything else thrown loading or making one, or asking
     * its fingerprints, is the run's failure, as sbt's host fails its task with it, no framework after it loaded (for a
     * build tool of no host, the run's error, `ForkTestMain.Run.run`).
     */
    private void load(boolean host) {
        this.host = host;
        Thread thread = Thread.currentThread();
        context = thread.getContextClassLoader();
        loader = new URLClassLoader(paths.toArray(new URL[0]), Runner.class.getClassLoader());
        thread.setContextClassLoader(loader);
        try {
            for (int i = 0; i < frameworks.size(); i++) {
                Framework framework;
                try {
                    framework = make(i);
                } catch (ClassNotFoundException e) {
                    instances.add(null);
                    say("missing", i, e);
                    continue;
                }
                instances.add(framework);
            }
            // The frameworks' fingerprints, then what sbt's discovery reads of them (Tests.discover): each subclass
            // one's superclass and module, each annotated one's annotation and module, then the text of its two debug
            // lines, every subclass fingerprint's and every annotated one's.
            // sbt's host asks them reflectively (TestFramework.getFingerprints), what they throw wrapped.
            for (Framework framework : instances) {
                fingerprints.add(framework == null ? new Fingerprint[0] : host ? fingerprintsOf(framework) : framework.fingerprints());
            }
            String[][] superclasses = new String[frameworks.size()][];
            boolean[][] subclassModules = new boolean[frameworks.size()][];
            String[][] annotations = new String[frameworks.size()][];
            boolean[][] annotatedModules = new boolean[frameworks.size()][];
            for (int i = 0; i < frameworks.size(); i++) {
                superclasses[i] = new String[fingerprints.get(i).length];
                subclassModules[i] = new boolean[fingerprints.get(i).length];
                annotations[i] = new String[fingerprints.get(i).length];
                annotatedModules[i] = new boolean[fingerprints.get(i).length];
                for (int j = 0; j < fingerprints.get(i).length; j++) {
                    if (fingerprints.get(i)[j] instanceof SubclassFingerprint s) {
                        superclasses[i][j] = s.superclassName();
                        subclassModules[i][j] = s.isModule();
                    }
                }
            }
            for (int i = 0; i < frameworks.size(); i++) {
                for (int j = 0; j < fingerprints.get(i).length; j++) {
                    if (fingerprints.get(i)[j] instanceof AnnotatedFingerprint a) {
                        annotations[i][j] = a.annotationName();
                        annotatedModules[i][j] = a.isModule();
                    }
                }
            }
            StringBuilder debug = new StringBuilder();
            for (Fingerprint[] prints : fingerprints) {
                for (Fingerprint print : prints) {
                    if (print instanceof SubclassFingerprint) {
                        debug.append(print);
                    }
                }
            }
            for (Fingerprint[] prints : fingerprints) {
                for (Fingerprint print : prints) {
                    if (print instanceof AnnotatedFingerprint) {
                        debug.append(print);
                    }
                }
            }
            // A fingerprint of both kinds discovers by either, as it is in both of discovery's lists.
            for (int i = 0; i < frameworks.size(); i++) {
                for (int j = 0; j < fingerprints.get(i).length; j++) {
                    Fingerprint print = fingerprints.get(i)[j];
                    if (print instanceof SubclassFingerprint) {
                        say("fingerprint", i, j, "subclass", subclassModules[i][j], superclasses[i][j]);
                    }
                    if (print instanceof AnnotatedFingerprint) {
                        say("fingerprint", i, j, "annotated", annotatedModules[i][j], annotations[i][j]);
                    }
                }
            }
        } catch (Throwable t) {
            Events events = new Events();
            if (host) {
                // sbt's host fails its task with it (`loadedTestFrameworks`, `definedTests`), before any worker starts.
                hostError(events, t);
            } else {
                try {
                    new Forwarded(false, false, events).error("Uncaught exception when running tests: " + t);
                    events.forkError(t);
                } catch (Throwable t2) {
                    internalError(t2);
                }
            }
            // No framework runs after the run's error.
            Collections.fill(instances, null);
            while (instances.size() < frameworks.size()) {
                instances.add(null);
            }
            fingerprints.clear();
            while (fingerprints.size() < frameworks.size()) {
                fingerprints.add(new Fingerprint[0]);
            }
        }
        say("loaded");
    }

    /** The suites of a phase that a framework's fingerprint matches added to its definitions, as the worker adds them
     * (ForkTestMain.runTests 365-372): the definition carrying the suite's copy and, of sbt's export, the suite's one
     * selectors array; otherwise one of its own. */
    private static void match(Fingerprint mine, List<String[]> phase, Map<String[], Fingerprint> copies, Map<String[], Selector[]> selectors, LinkedHashSet<TaskDef> defs) {
        for (String[] s : phase) {
            Fingerprint print = copies.get(s);
            if (matches(mine, print)) {
                Selector[] selected = selectors.get(s);
                defs.add(new TaskDef(s[5], print, s[4].equals("1"), selected != null ? selected : new Selector[] { new SuiteSelector() }));
            }
        }
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

    /**
     * A run of `phases` ordered phases, one per group of the configuration's suites and the suites of none last, as
     * sbt runs its groups in process (`Defaults.allTestGroupsTask`), played for an export of sbt's as sbt plays it:
     * sbt's host first (`host`: its selection's reads, the frameworks it keeps, their runners, the suites' copies and
     * the runners' arguments), then its worker, then the host once the worker has answered (`summaries`). The worker:
     * one pool, and for each framework the host kept, in its order, the worker's own instance and runner, made the
     * first time it is reached, asked each phase's tasks as the phase is entered (none, for a framework that matches
     * nothing in it) and every wave of them and of the tasks they return drained before the next framework's and the
     * next phase's; each framework's done() once every phase has drained (`allTestGroupsTask` calling every runner's
     * done() once `Tests.foldTasks` has run every group). A fork error ends the run after the phase it happens in, no
     * later phase entered, as sbt's harness fails its group and no later group starts, the frameworks' done() still
     * called and the host's summaries not; one of making a runner or its tasks, or of a done(), ends it at once
     * (ForkTestMain.Run.run), and an internal error, after which the worker answers as it does at its end, the host's
     * summaries made. What the host throws is the run's failure, before any worker, or after it.
     */
    private void run(boolean verbose, boolean ansi, String threads, int phases, BufferedReader in) throws IOException {
        List<List<String[]>> suites = new ArrayList<>();
        for (int p = 0; p < phases; p++) {
            suites.add(new ArrayList<>());
        }
        // The suites in the selection's order (sbt's `Tests.processOptions`'), and for sbt's host the definitions its
        // selection starts from and its ordered filters, each the positions of the definitions it accepts.
        List<String[]> order = new ArrayList<>();
        List<String[]> definitions = new ArrayList<>();
        List<Set<Integer>> filters = new ArrayList<>();
        String line;
        while ((line = in.readLine()) != null && !line.equals("go")) {
            String[] fields = fields(line);
            switch (fields[0]) {
                case "def" -> definitions.add(fields);
                case "filter" -> {
                    Set<Integer> accepted = new HashSet<>();
                    for (int k = 1; k < fields.length; k++) {
                        accepted.add(Integer.parseInt(fields[k]));
                    }
                    filters.add(accepted);
                }
                case "suite" -> {
                    order.add(fields);
                    suites.get(Integer.parseInt(fields[3])).add(fields);
                }
                default -> {
                }
            }
        }
        long start = System.nanoTime();
        Events events = new Events();
        // The worker's own lines go to its own logger (ForkTestMain.Run's log), the frameworks' to the array they are
        // given, which they hold; the host's lines to the same stream, as sbt's console shows both.
        Forwarded log = new Forwarded(verbose, ansi, events);
        Logger[] loggers = { log };
        Thread thread = Thread.currentThread();
        int n = frameworks.size();
        // Each framework the worker is given with the arguments of its runner (none: not given), its instance, its
        // own fingerprints, its runner and the runner's hook; the host's runners.
        String[][] runnerArgs = new String[n][];
        String[][] remoteArgs = new String[n][];
        Framework[] workers = new Framework[n];
        boolean[] absent = new boolean[n];
        Fingerprint[][] own = new Fingerprint[n][];
        sbt.testing.Runner[] runners = new sbt.testing.Runner[n];
        Thread[] hooks = new Thread[n];
        sbt.testing.Runner[] hostRunners = new sbt.testing.Runner[n];
        Map<String[], Fingerprint> copies = new IdentityHashMap<>();
        // Of sbt's export, each suite's selectors, one array its request's definition holds, which the worker gives
        // every framework's definition of the suite (ForkTestMain.runTests 372, `test.selectors()`).
        Map<String[], Selector[]> selectors = new IdentityHashMap<>();
        try (URLClassLoader loader = this.loader) {
            if (host) {
                try {
                    host(definitions, filters, order, hostRunners, runnerArgs, remoteArgs, copies, loader);
                } catch (Throwable t) {
                    hostError(events, t);
                }
                for (String[] suite : order) {
                    selectors.put(suite, new Selector[] { new SuiteSelector() });
                }
            } else {
                // A build tool of no host (Mill's runner): every framework that loaded, made once, its runner given
                // the declared arguments.
                for (int i = 0; i < n; i++) {
                    if (instances.get(i) != null) {
                        workers[i] = instances.get(i);
                        runnerArgs[i] = arguments.get(i);
                        remoteArgs[i] = new String[0];
                    }
                }
            }
            // No selected suite, no worker (ForkTests.apply 60-62): sbt's host has made its selection's calls alone.
            if (!events.failed && (!host || !order.isEmpty())) {
                ExecutorService pool = pool(threads, log);
                try {
                    if (!host) {
                        // Each suite's fingerprint copied, phase by phase, a copy's getters read once, a definition
                        // carrying the copy.
                        for (List<String[]> phase : suites) {
                            for (String[] suite : phase) {
                                if (instances.get(Integer.parseInt(suite[1])) == null) {
                                    continue;
                                }
                                copies.put(suite, copy(printOf(suite)));
                            }
                        }
                    }
                    for (int p = 0; p < phases && !events.failed; p++) {
                        long entered = System.nanoTime();
                        say("phase", p);
                        for (int i = 0; i < n; i++) {
                            // Only the frameworks the host gave the worker, in their order (`filteredFrameworks`).
                            if (runnerArgs[i] == null || absent[i]) {
                                continue;
                            }
                            // The worker's own instance (ForkTestMain.runTests 347-360): a class not found said and
                            // the framework skipped.
                            if (workers[i] == null) {
                                try {
                                    workers[i] = make(i);
                                } catch (ClassNotFoundException e) {
                                    log.error("Framework implementation '" + frameworks.get(i) + "' not present.");
                                    absent[i] = true;
                                    continue;
                                }
                            }
                            // Every suite of the phase whose fingerprint one of the framework's own matches, in the
                            // order of its fingerprints, asked of it the first time it is reached
                            // (ForkTestMain.runTests): a suite of two frameworks' fingerprint is each one's. A
                            // definition carries the copy.
                            LinkedHashSet<TaskDef> defs = new LinkedHashSet<>();
                            if (own[i] == null) {
                                // The worker's loop (363, a for-each): the array taken once and each member read as
                                // the loop reaches it, so that one a getter replaces meanwhile is the one met; of
                                // sbt's export the members as read kept for the later phases, whatever the
                                // framework or its runner does to its array after.
                                Fingerprint[] live = workers[i].fingerprints();
                                Fingerprint[] read = new Fingerprint[live.length];
                                for (int j = 0; j < read.length; j++) {
                                    read[j] = live[j];
                                    match(read[j], suites.get(p), copies, selectors, defs);
                                }
                                own[i] = host ? read : live;
                            } else {
                                for (Fingerprint mine : own[i]) {
                                    match(mine, suites.get(p), copies, selectors, defs);
                                }
                            }
                            if (runners[i] == null) {
                                runners[i] = workers[i].runner(runnerArgs[i], remoteArgs[i], loader);
                            }
                            Task[] tasks = runners[i].tasks(defs.toArray(new TaskDef[0]));
                            log.debug("Runner for " + workers[i].getClass().getName() + " produced " + tasks.length + " initial tasks for " + defs.size() + " tests.");
                            // done() called as the JVM ends, should the run end before it (ForkTestMain.runTests).
                            if (hooks[i] == null) {
                                hooks[i] = new Thread(runners[i]::done);
                                Runtime.getRuntime().addShutdownHook(hooks[i]);
                            }
                            runTasks(pool, tasks, events, loggers, log);
                        }
                        say("ended", p, (System.nanoTime() - entered) / 1_000_000);
                    }
                    // Each runner's done(), what it returns printed nowhere, once every phase has drained.
                    for (int i = 0; i < n; i++) {
                        if (runners[i] != null) {
                            runners[i].done();
                            Runtime.getRuntime().removeShutdownHook(hooks[i]);
                        }
                    }
                } catch (Throwable t) {
                    // A framework that fails to make its runner or its tasks, or whose done() throws, ends the run, no
                    // framework or phase after it run (ForkTestMain.Run.run), what reporting it throws an internal
                    // error.
                    try {
                        log.error("Uncaught exception when running tests: " + t);
                        events.forkError(t);
                    } catch (Throwable t2) {
                        internalError(t2);
                    }
                } finally {
                    // Its threads, whose context loader is the run's, end before the loader is closed.
                    pool.shutdown();
                    try {
                        pool.awaitTermination(Long.MAX_VALUE, TimeUnit.NANOSECONDS);
                    } catch (InterruptedException e) {
                        thread.interrupt();
                    }
                }
                // The host once its worker has answered, which a fork error fails first (ForkTests' React).
                if (host && !events.failed) {
                    try {
                        summaries(hostRunners, log);
                    } catch (Throwable t) {
                        hostError(events, t);
                    }
                }
            }
        } finally {
            thread.setContextClassLoader(context);
        }
        out.flush();
        say("done", (System.nanoTime() - start) / 1_000_000);
    }

    /**
     * sbt 2.0.8's host from its selection to its worker's request, each call of framework code in its order, the
     * suites in the selection's order (`order`, the definitions' positions `definitions` and `filters` the
     * selection's input). `Tests.processOptions` (`MA/Tests.scala` 299-302): the definitions left `distinct`, then the
     * ordered filters' matches of those, in the filters' order, `distinct` again, a definition's hash and equality
     * reading its fingerprint as `TestDefinition` reads it (`TestFramework.scala` 103-108, 201-212). Then
     * `filteredFrameworks` (`Defaults.scala` 1576-1583): each framework that loaded, in its order, whose
     * fingerprints, asked reflectively, one of match a selected suite's (`TestFramework.matches`), kept; the kept
     * frameworks' runners (`createTestRunners`, 1457-1469: the declared arguments and no remote ones); each suite's
     * fingerprint copied (`ForkTests.scala` 103-109, 163-167); each runner's `args()` and `remoteArgs()`, which the
     * worker's runner is made with (110-115).
     */
    private void host(List<String[]> definitions, List<Set<Integer>> filters, List<String[]> selected, sbt.testing.Runner[] hostRunners, String[][] runnerArgs, String[][] remoteArgs, Map<String[], Fingerprint> copies, ClassLoader loader) throws Exception {
        // A framework that did not load defines nothing.
        List<Integer> left = new ArrayList<>();
        for (int k = 0; k < definitions.size(); k++) {
            if (instances.get(Integer.parseInt(definitions.get(k)[1])) != null) {
                left.add(k);
            }
        }
        List<String[]> order = new ArrayList<>();
        for (String[] suite : selected) {
            if (instances.get(Integer.parseInt(suite[1])) != null) {
                order.add(suite);
            }
        }
        List<Integer> unique = distinct(definitions, left);
        if (!filters.isEmpty()) {
            List<Integer> matched = new ArrayList<>();
            for (Set<Integer> filter : filters) {
                for (int k : unique) {
                    if (filter.contains(k)) {
                        matched.add(k);
                    }
                }
            }
            distinct(definitions, matched);
        }
        int n = frameworks.size();
        boolean[] filtered = new boolean[n];
        for (int i = 0; i < n; i++) {
            if (instances.get(i) == null) {
                continue;
            }
            for (Fingerprint t : fingerprintsOf(instances.get(i))) {
                for (String[] suite : order) {
                    if (hostMatches(t, printOf(suite))) {
                        filtered[i] = true;
                        break;
                    }
                }
                if (filtered[i]) {
                    break;
                }
            }
        }
        for (int i = 0; i < n; i++) {
            if (filtered[i]) {
                hostRunners[i] = instances.get(i).runner(arguments.get(i).clone(), new String[0], loader);
            }
        }
        for (String[] suite : order) {
            copies.put(suite, copy(printOf(suite)));
        }
        for (int i = 0; i < n; i++) {
            if (hostRunners[i] != null) {
                runnerArgs[i] = hostRunners[i].args().clone();
                remoteArgs[i] = hostRunners[i].remoteArgs().clone();
            }
        }
    }

    /**
     * Scala's `distinct` of a `List` of the definitions at `in` (`immutable.StrictOptimizedSeqOps.distinctBy` over a
     * `mutable.HashSet`): a list of one or none returned as it is, nothing read; else the first of each, a definition
     * compared only with those kept before it of an equal hash, found by its hash as `HashSet.addElem` finds its bucket,
     * in the order they were kept, until one is equal: the same definition (an ordered filter's repeating one, the same
     * position) without a read, as `HashSet.addElem`'s `==` (`BoxesRunTime.equals`) takes the same reference before it
     * calls `TestDefinition.equals`; returned in order.
     */
    private List<Integer> distinct(List<String[]> definitions, List<Integer> in) {
        if (in.size() <= 1) {
            return new ArrayList<>(in);
        }
        List<Integer> kept = new ArrayList<>();
        Map<Integer, List<Integer>> byHash = new HashMap<>();
        for (int k : in) {
            String[] d = definitions.get(k);
            int hash = tuple2(d[3].hashCode(), printHash(printOf(d)));
            List<Integer> same = byHash.computeIfAbsent(hash, h -> new ArrayList<>());
            boolean seen = false;
            for (int j = 0; j < same.size() && !seen; j++) {
                int other = same.get(j);
                seen = k == other || d[3].equals(definitions.get(other)[3]) && hostMatches(printOf(d), printOf(definitions.get(other)));
            }
            if (!seen) {
                kept.add(k);
                same.add(k);
            }
        }
        return kept;
    }

    /** `TestFramework.hashCode` of a fingerprint (`TestFramework.scala` 201-205): its module then its name read, the
     * pair's hash; 0 for one of neither kind. */
    private static int printHash(Fingerprint f) {
        if (f instanceof SubclassFingerprint s) {
            boolean module = s.isModule();
            return tuple2(module ? 1231 : 1237, Objects.hashCode(s.superclassName()));
        }
        if (f instanceof AnnotatedFingerprint a) {
            boolean module = a.isModule();
            return tuple2(module ? 1231 : 1237, Objects.hashCode(a.annotationName()));
        }
        return 0;
    }

    /** The hash of a Scala pair of these elements' hashes (`MurmurHash3.productHash` of a `Tuple2`). */
    private static int tuple2(int a, int b) {
        int h = mix(mix(mix(0xcafebabe, "Tuple2".hashCode()), a), b) ^ 2;
        h ^= h >>> 16;
        h *= 0x85ebca6b;
        h ^= h >>> 13;
        h *= 0xc2b2ae35;
        return h ^ (h >>> 16);
    }

    private static int mix(int h, int data) {
        int k = Integer.rotateLeft(data * 0xcc9e2d51, 15) * 0x1b873593;
        return Integer.rotateLeft(h ^ k, 13) * 5 + 0xe6546b64;
    }

    /** sbt's host's `TestFramework.matches` (`TestFramework.scala` 206-212): of one kind, the module then the name of
     * each read, the names compared as Scala's `==` compares them. */
    private static boolean hostMatches(Fingerprint a, Fingerprint b) {
        if (a instanceof SubclassFingerprint sa && b instanceof SubclassFingerprint sb) {
            return sa.isModule() == sb.isModule() && Objects.equals(sa.superclassName(), sb.superclassName());
        } else if (a instanceof AnnotatedFingerprint aa && b instanceof AnnotatedFingerprint ab) {
            return aa.isModule() == ab.isModule() && Objects.equals(aa.annotationName(), ab.annotationName());
        }
        return false;
    }

    /** `TestFramework.getFingerprints` (`TestFramework.scala` 190-194): `fingerprints()` called reflectively, what it
     * throws wrapped, anything but an array of them an error naming the framework; the array's members taken as they
     * are then (`toList`), so that a framework that later replaces one of its array's entries leaves them as they
     * were. */
    private static Fingerprint[] fingerprintsOf(Framework framework) throws Exception {
        Object prints = framework.getClass().getMethod("fingerprints").invoke(framework);
        if (prints instanceof Fingerprint[] f) {
            return f.clone();
        }
        throw new RuntimeException("Could not call 'fingerprints' on framework " + framework);
    }

    /** `ForkTests.forkFingerprint` (`ForkTests.scala` 163-167). */
    private static Fingerprint copy(Fingerprint f) {
        if (f instanceof SubclassFingerprint s) {
            return new SubclassFingerscan(s);
        } else if (f instanceof AnnotatedFingerprint a) {
            return new AnnotatedFingerscan(a);
        }
        throw new RuntimeException("Unknown fingerprint type: " + f.getClass());
    }

    /** The fingerprint `load` asked of a suite's or definition's framework, at its index. */
    private Fingerprint printOf(String[] line) {
        return fingerprints.get(Integer.parseInt(line[1]))[Integer.parseInt(line[2])];
    }

    /** The framework of the export at `i`, made through the run's class loader, one of sbt's older interface
     * wrapped (ForkTestMain.runTests, TestFramework.createFramework). */
    private Framework make(int i) throws Exception {
        Object raw = loader.loadClass(frameworks.get(i)).getDeclaredConstructor().newInstance();
        return raw instanceof Framework f ? f : new FrameworkWrapper((org.scalatools.testing.Framework) raw);
    }

    /**
     * sbt's host once its worker has answered (`Defaults.scala` 1664-1666, `TestResultLogger.scala` 115-123): each
     * kept framework's name and its runner's `done()`, in their order, then each summary shown, the framework's name
     * before it where there are several, one that is empty a debug line.
     */
    private void summaries(sbt.testing.Runner[] hostRunners, Forwarded log) {
        List<String> names = new ArrayList<>();
        List<String> texts = new ArrayList<>();
        for (int i = 0; i < hostRunners.length; i++) {
            if (hostRunners[i] != null) {
                names.add(instances.get(i).name());
                texts.add(hostRunners[i].done());
            }
        }
        boolean several = names.size() > 1;
        for (int k = 0; k < names.size(); k++) {
            if (texts.get(k).isEmpty()) {
                log.debug("Summary for " + names.get(k) + " not available.");
            } else {
                if (several) {
                    log.info(names.get(k));
                }
                log.info(texts.get(k));
            }
        }
    }

    /** What sbt's host throws, which fails its task with it, no line of the worker's: sent as the run's fork error, one
     * whose text cannot be made with none. */
    private void hostError(Events events, Throwable t) {
        try {
            events.writeError(new ForkError(t));
        } catch (Throwable t2) {
            events.failed = true;
            say("forkError");
        }
    }

    /** What reporting the run's error threw (ForkTestMain.Run.internalError), written as the worker writes it, on the
     * stream of the suites' output, then sent: the run's phases end there with no fork error, the worker answering as
     * at its end, so that the run's outcome is its suites' (ForkTests.mainTestTask). What making the text throws
     * leaves `main`, which ends the JVM. */
    private void internalError(Throwable t) {
        String text = "Internal error when running tests: " + t;
        System.err.println(text);
        say("internalError", text);
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

    /** A wave of tasks on the pool, their results awaited in the wave's order, then the tasks they returned as the
     * next wave (ForkTestMain.runTestTasks): a task's children wait for the whole of its wave. */
    private void runTasks(ExecutorService pool, Task[] tasks, Events events, Logger[] loggers, Forwarded log) {
        while (tasks.length > 0) {
            List<Future<Task[]>> wave = new ArrayList<>();
            for (Task task : tasks) {
                wave.add(pool.submit(() -> runTask(task, events, loggers, log)));
            }
            List<Task> nested = new ArrayList<>();
            for (Future<Task[]> result : wave) {
                try {
                    nested.addAll(Arrays.asList(result.get()));
                } catch (Exception e) {
                    log.error("Failed to execute task " + result);
                }
            }
            tasks = nested.toArray(new Task[0]);
        }
    }

    /** A task, its events sent together once it returns (ForkTestMain.runTest); one that throws has its error
     * logged and is an error of its suite, with no duration, the events it sent dropped. Sending them is outside
     * the task's catch, as the worker's writeEvents is: one the worker could not write fails the task's future. */
    private Task[] runTask(Task task, Events events, Logger[] loggers, Forwarded log) {
        TaskDef def = task.taskDef();
        Queue<Object> sent = new ConcurrentLinkedQueue<>();
        Task[] nested;
        List<Object> batch;
        try {
            // The worker's debug lines, their text made whether shown or not (ForkTestMain.runTest): the definition's
            // and the returned tasks' reads are the task's, a throw among them its own.
            log.debug("  Running " + def);
            nested = task.execute(event -> sent.add(events.event(event)), loggers);
            if (nested.length > 0 || sent.size() > 0) {
                log.debug("    Produced " + nested.length + " nested tasks and " + sent.size() + " events.");
            }
            batch = new ArrayList<>(sent);
        } catch (Throwable t) {
            nested = new Task[0];
            String message = "Uncaught exception when running " + def.fullyQualifiedName() + ": " + t;
            log.error(message);
            // What was thrown copied once, sent as the fork error and carried by the suite's Error, read as the worker
            // reads any event (ForkTestMain.testError, testEvent, ForkEvent): the fingerprint's getters among the
            // reads, one that throws failing the task's future.
            ForkError fe = new ForkError(t);
            events.writeError(fe);
            batch = List.of(events.event(error(def, fe)));
        }
        // The batch sent under the definition's name (ForkTestMain.writeEvents), read there.
        events.send(batch, def.fullyQualifiedName());
        return nested;
    }

    /** A subclass fingerprint copied, its getters read once (ForkTestMain.SubclassFingerscan). */
    private static final class SubclassFingerscan implements SubclassFingerprint, Serializable {
        private final boolean isModule;
        private final String superclassName;
        private final boolean requireNoArgConstructor;

        SubclassFingerscan(SubclassFingerprint print) {
            isModule = print.isModule();
            superclassName = print.superclassName();
            requireNoArgConstructor = print.requireNoArgConstructor();
        }

        @Override
        public boolean isModule() {
            return isModule;
        }

        @Override
        public String superclassName() {
            return superclassName;
        }

        @Override
        public boolean requireNoArgConstructor() {
            return requireNoArgConstructor;
        }
    }

    /** An annotated fingerprint copied, its getters read once (ForkTestMain.AnnotatedFingerscan). */
    private static final class AnnotatedFingerscan implements AnnotatedFingerprint, Serializable {
        private final boolean isModule;
        private final String annotationName;

        AnnotatedFingerscan(AnnotatedFingerprint print) {
            isModule = print.isModule();
            annotationName = print.annotationName();
        }

        @Override
        public boolean isModule() {
            return isModule;
        }

        @Override
        public String annotationName() {
            return annotationName;
        }
    }

    /** What was thrown, copied as the worker copies it (ForkTestMain.ForkError): its message, its class's name, its
     * frames, and its cause copied the same way; its message `<class>: <message>`. */
    private static final class ForkError extends Exception {
        private final String originalMessage;
        private final String originalName;
        private ForkError cause1;

        ForkError(Throwable t) {
            originalMessage = t.getMessage();
            originalName = t.getClass().getName();
            setStackTrace(t.getStackTrace());
            if (t.getCause() != null) {
                cause1 = new ForkError(t.getCause());
            }
        }

        @Override
        public String getMessage() {
            return originalName + ": " + originalMessage;
        }

        @Override
        public Exception getCause() {
            return cause1;
        }
    }

    /** A task that threw: an Error of its suite, as the worker makes it (ForkTestMain.testError, testEvent): the
     * definition's name and fingerprint, a `SuiteSelector`, what was thrown and no duration. */
    private static Event error(TaskDef def, ForkError t) {
        String name = def.fullyQualifiedName();
        Fingerprint print = def.fingerprint();
        OptionalThrowable thrown = new OptionalThrowable(t);
        return new Event() {
            @Override
            public String fullyQualifiedName() {
                return name;
            }

            @Override
            public Fingerprint fingerprint() {
                return print;
            }

            @Override
            public Selector selector() {
                return new SuiteSelector();
            }

            @Override
            public Status status() {
                return Status.Error;
            }

            @Override
            public OptionalThrowable throwable() {
                return thrown;
            }

            @Override
            public long duration() {
                return 0;
            }
        };
    }

    /** An event of a selector the worker's gson cannot write, of the selector's class. */
    private record Unsendable(String type) {
    }

    /** An event as `Events.event` read it: what it threw, its status, its name, its selector's kind and names, its
     * duration. */
    private record Read(String thrown, int status, String name, String kind, String a, String b, long duration) {
    }

    /** The lines of a run's events, each with what it threw before it, made as the event comes. */
    private final class Events {
        /** An event read as the worker's `ForkEvent` reads it before it is buffered, its getters in that order (its
         * name, its fingerprint's, its selector, its status, its throwable, its duration), an event it refuses thrown
         * to the framework, which a task that lets it through then throws (the throwing task's path): a fingerprint
         * of neither kind, or a selector that is not `Serializable` (ForkTestMain.ForkEvent), and a getter that
         * throws. One of a selector of a kind gson's adapter does not know (WorkerMain.mkGson) is taken, as the
         * worker takes it, to fail when its task's events are sent. */
        Object event(Event event) {
            String name = event.fullyQualifiedName();
            Fingerprint print = event.fingerprint();
            if (print instanceof SubclassFingerprint s) {
                new SubclassFingerscan(s);
            } else {
                new AnnotatedFingerscan((AnnotatedFingerprint) print);
            }
            Selector selector = event.selector();
            if (!(selector instanceof Serializable)) {
                throw new UnsupportedOperationException("Selector implementation must be Serializable, but " + selector.getClass().getName() + " is not.");
            }
            Status status = event.status();
            OptionalThrowable throwable = event.throwable();
            // What was thrown copied (ForkEvent's ForkError); a task's own Error carries its copy already, written as
            // it is where the worker's host reads the copy of the copy.
            String thrown = !throwable.isDefined() ? "" : thrown(throwable.get() instanceof ForkError fe ? fe : new ForkError(throwable.get()));
            long duration = event.duration();
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
            // A status of null carried as the worker's ForkEvent carries it, which sbt's host counts as none
            // (SuiteResult.apply), sent as -1; of Mill's export, which has no such host, the task's throw, as before.
            return new Read(thrown, status == null && host ? -1 : status.ordinal(), name, kind, a, b, duration);
        }

        /** A task's events, then that its suite ran, a task of no event among them (an empty suite's), under the task's
         * name, written at once. */
        void send(List<Object> batch, String suite) {
            StringBuilder text = new StringBuilder();
            for (Object e : batch) {
                if (e instanceof Unsendable u) {
                    // gson's RuntimeTypeAdapterFactory, writing the worker's batch, refuses the selector's class.
                    throw new UnsupportedOperationException("cannot serialize " + u.type() + "; did you forget to register a subtype?");
                }
                Read r = (Read) e;
                text.append(r.thrown()).append(line("event", suite, r.status(), r.name(), r.kind(), r.a(), r.b(), r.duration()));
            }
            text.append(line("ran", suite));
            out.print(text);
        }

        /** Whether a fork error was sent, after which no later phase is entered. */
        volatile boolean failed;

        /** What the worker sends as a fork error of what was thrown (ForkTestMain.writeError of a new ForkError): what
         * a framework's logger traced, or the run's error, which ends the run as sbt's ForkTests fails its harness. */
        void forkError(Throwable t) {
            writeError(new ForkError(t));
        }

        /** A fork error of a copy made: its lines made, then the run marked failed, then written. */
        void writeError(ForkError fe) {
            String text = thrown(fe) + line("forkError");
            failed = true;
            out.print(text);
        }

        /** A copy of what was thrown, its causes' copies, and the frames of each. */
        private String thrown(ForkError e) {
            StringBuilder lines = new StringBuilder();
            String kind = "throwable";
            for (Throwable t = e; t != null; t = t.getCause()) {
                lines.append(line(kind, t.getMessage()));
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
            // A message of null the worker sends without its text, and sbt's console shows nothing of it.
            if (message == null) {
                return;
            }
            StringBuilder lines = new StringBuilder();
            message.lines().forEach(line -> lines.append(prefix).append(line).append('\n'));
            out.print(lines);
        }
    }

    /**
     * A framework of sbt's older interface (`org.scalatools.testing.Framework`) as the new one, the worker's
     * `FrameworkWrapper` (sbt 2.0.8's worker module, as sbt 1's `ForkMain` wraps it too): its fingerprints its
     * `tests()`, a `TestFingerprint` or `SubclassFingerprint` a subclass one and any other an annotated one, and its
     * runner one whose tasks run each suite through the framework's `testRunner`, nested tasks none, `done()` "".
     */
    private static final class FrameworkWrapper implements Framework {
        private final org.scalatools.testing.Framework oldFramework;

        FrameworkWrapper(org.scalatools.testing.Framework oldFramework) {
            this.oldFramework = oldFramework;
        }

        @Override
        public String name() {
            return oldFramework.name();
        }

        @Override
        public Fingerprint[] fingerprints() {
            org.scalatools.testing.Fingerprint[] oldFingerprints = oldFramework.tests();
            int length = oldFingerprints.length;
            Fingerprint[] fingerprints = new Fingerprint[length];
            for (int i = 0; i < length; i++) {
                org.scalatools.testing.Fingerprint oldFingerprint = oldFingerprints[i];
                if (oldFingerprint instanceof org.scalatools.testing.TestFingerprint t) {
                    fingerprints[i] = new SubclassFingerprintWrapper(t);
                } else if (oldFingerprint instanceof org.scalatools.testing.SubclassFingerprint s) {
                    fingerprints[i] = new SubclassFingerprintWrapper(s);
                } else {
                    fingerprints[i] = new AnnotatedFingerprintWrapper((org.scalatools.testing.AnnotatedFingerprint) oldFingerprint);
                }
            }
            return fingerprints;
        }

        @Override
        public sbt.testing.Runner runner(String[] args, String[] remoteArgs, ClassLoader testClassLoader) {
            return new RunnerWrapper(oldFramework, testClassLoader, args);
        }
    }

    /** An old subclass fingerprint, which requires no constructor without arguments. */
    private static final class SubclassFingerprintWrapper implements SubclassFingerprint {
        private final String superclassName;
        private final boolean isModule;
        private final boolean requireNoArgConstructor;

        SubclassFingerprintWrapper(org.scalatools.testing.SubclassFingerprint oldFingerprint) {
            superclassName = oldFingerprint.superClassName();
            isModule = oldFingerprint.isModule();
            requireNoArgConstructor = false;
        }

        @Override
        public boolean isModule() {
            return isModule;
        }

        @Override
        public String superclassName() {
            return superclassName;
        }

        @Override
        public boolean requireNoArgConstructor() {
            return requireNoArgConstructor;
        }
    }

    private static final class AnnotatedFingerprintWrapper implements AnnotatedFingerprint {
        private final String annotationName;
        private final boolean isModule;

        AnnotatedFingerprintWrapper(org.scalatools.testing.AnnotatedFingerprint oldFingerprint) {
            annotationName = oldFingerprint.annotationName();
            isModule = oldFingerprint.isModule();
        }

        @Override
        public boolean isModule() {
            return isModule;
        }

        @Override
        public String annotationName() {
            return annotationName;
        }
    }

    /** An old event handler over a new one, each event an `EventWrapper` of the task's suite and fingerprint. */
    private static final class EventHandlerWrapper implements org.scalatools.testing.EventHandler {
        private final EventHandler newEventHandler;
        private final String fullyQualifiedName;
        private final Fingerprint fingerprint;

        EventHandlerWrapper(EventHandler newEventHandler, String fullyQualifiedName, Fingerprint fingerprint) {
            this.newEventHandler = newEventHandler;
            this.fullyQualifiedName = fullyQualifiedName;
            this.fingerprint = fingerprint;
        }

        @Override
        public void handle(org.scalatools.testing.Event oldEvent) {
            newEventHandler.handle(new EventWrapper(oldEvent, fullyQualifiedName, fingerprint));
        }
    }

    /** An old event as a new one: a `TestSelector` of its test's name, its result's status, no duration (-1). */
    private static final class EventWrapper implements Event {
        private final org.scalatools.testing.Event oldEvent;
        private final String className;
        private final Fingerprint fingerprint;
        private final OptionalThrowable throwable;

        EventWrapper(org.scalatools.testing.Event oldEvent, String className, Fingerprint fingerprint) {
            this.oldEvent = oldEvent;
            this.className = className;
            this.fingerprint = fingerprint;
            Throwable oldThrowable = oldEvent.error();
            if (oldThrowable == null) {
                throwable = new OptionalThrowable();
            } else {
                throwable = new OptionalThrowable(oldThrowable);
            }
        }

        @Override
        public String fullyQualifiedName() {
            return className;
        }

        @Override
        public Fingerprint fingerprint() {
            return fingerprint;
        }

        @Override
        public Selector selector() {
            return new TestSelector(oldEvent.testName());
        }

        @Override
        public Status status() {
            switch (oldEvent.result()) {
                case Success:
                    return Status.Success;
                case Error:
                    return Status.Error;
                case Failure:
                    return Status.Failure;
                case Skipped:
                    return Status.Skipped;
                default:
                    throw new IllegalStateException("Invalid status.");
            }
        }

        @Override
        public OptionalThrowable throwable() {
            return throwable;
        }

        @Override
        public long duration() {
            return -1;
        }
    }

    /** An old framework's runner: a task per definition, each running its suite through the framework's `testRunner`
     * with old loggers over the new ones, a `Runner2` given its fingerprint as it is, an older one a subclass
     * fingerprint's; no nested tasks, `done()` "" and no remote arguments. */
    private static final class RunnerWrapper implements sbt.testing.Runner {
        private final org.scalatools.testing.Framework oldFramework;
        private final ClassLoader testClassLoader;
        private final String[] args;

        RunnerWrapper(org.scalatools.testing.Framework oldFramework, ClassLoader testClassLoader, String[] args) {
            this.oldFramework = oldFramework;
            this.testClassLoader = testClassLoader;
            this.args = args;
        }

        @Override
        public Task[] tasks(TaskDef[] taskDefs) {
            int length = taskDefs.length;
            Task[] tasks = new Task[length];
            for (int i = 0; i < length; i++) {
                TaskDef taskDef = taskDefs[i];
                tasks[i] = createTask(taskDef);
            }
            return tasks;
        }

        private Task createTask(TaskDef taskDef) {
            return new Task() {
                @Override
                public String[] tags() {
                    return new String[0];
                }

                private org.scalatools.testing.Logger createOldLogger(Logger logger) {
                    return new org.scalatools.testing.Logger() {
                        @Override
                        public boolean ansiCodesSupported() {
                            return logger.ansiCodesSupported();
                        }

                        @Override
                        public void error(String msg) {
                            logger.error(msg);
                        }

                        @Override
                        public void warn(String msg) {
                            logger.warn(msg);
                        }

                        @Override
                        public void info(String msg) {
                            logger.info(msg);
                        }

                        @Override
                        public void debug(String msg) {
                            logger.debug(msg);
                        }

                        @Override
                        public void trace(Throwable t) {
                            logger.trace(t);
                        }
                    };
                }

                private void runRunner(org.scalatools.testing.Runner runner, Fingerprint fingerprint, EventHandler eventHandler) {
                    // An old runner takes a subclass fingerprint alone.
                    SubclassFingerprint subclassFingerprint = (SubclassFingerprint) fingerprint;
                    org.scalatools.testing.TestFingerprint oldFingerprint = new org.scalatools.testing.TestFingerprint() {
                        @Override
                        public boolean isModule() {
                            return subclassFingerprint.isModule();
                        }

                        @Override
                        public String superClassName() {
                            return subclassFingerprint.superclassName();
                        }
                    };
                    String name = taskDef.fullyQualifiedName();
                    runner.run(name, oldFingerprint, new EventHandlerWrapper(eventHandler, name, subclassFingerprint), args);
                }

                private void runRunner2(org.scalatools.testing.Runner2 runner, Fingerprint fingerprint, EventHandler eventHandler) {
                    org.scalatools.testing.Fingerprint oldFingerprint;
                    if (fingerprint instanceof SubclassFingerprint subclassFingerprint) {
                        oldFingerprint = new org.scalatools.testing.SubclassFingerprint() {
                            @Override
                            public boolean isModule() {
                                return subclassFingerprint.isModule();
                            }

                            @Override
                            public String superClassName() {
                                return subclassFingerprint.superclassName();
                            }
                        };
                    } else {
                        AnnotatedFingerprint annotatedFingerprint = (AnnotatedFingerprint) fingerprint;
                        oldFingerprint = new org.scalatools.testing.AnnotatedFingerprint() {
                            @Override
                            public boolean isModule() {
                                return annotatedFingerprint.isModule();
                            }

                            @Override
                            public String annotationName() {
                                return annotatedFingerprint.annotationName();
                            }
                        };
                    }
                    String name = taskDef.fullyQualifiedName();
                    runner.run(name, oldFingerprint, new EventHandlerWrapper(eventHandler, name, fingerprint), args);
                }

                @Override
                public Task[] execute(EventHandler eventHandler, Logger[] loggers) {
                    int length = loggers.length;
                    org.scalatools.testing.Logger[] oldLoggers = new org.scalatools.testing.Logger[length];
                    for (int i = 0; i < length; i++) {
                        oldLoggers[i] = createOldLogger(loggers[i]);
                    }
                    org.scalatools.testing.Runner runner = oldFramework.testRunner(testClassLoader, oldLoggers);
                    Fingerprint fingerprint = taskDef.fingerprint();
                    if (runner instanceof org.scalatools.testing.Runner2 runner2) {
                        runRunner2(runner2, fingerprint, eventHandler);
                    } else {
                        runRunner(runner, fingerprint, eventHandler);
                    }
                    return new Task[0];
                }

                @Override
                public TaskDef taskDef() {
                    return taskDef;
                }
            };
        }

        @Override
        public String done() {
            return "";
        }

        @Override
        public String[] args() {
            return args;
        }

        @Override
        public String[] remoteArgs() {
            return new String[0];
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
