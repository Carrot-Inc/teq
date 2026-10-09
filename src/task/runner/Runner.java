package dev.teq.runner;

import java.io.BufferedReader;
import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import sbt.testing.AnnotatedFingerprint;
import sbt.testing.Event;
import sbt.testing.EventHandler;
import sbt.testing.Fingerprint;
import sbt.testing.Framework;
import sbt.testing.Logger;
import sbt.testing.NestedSuiteSelector;
import sbt.testing.NestedTestSelector;
import sbt.testing.Selector;
import sbt.testing.SubclassFingerprint;
import sbt.testing.SuiteSelector;
import sbt.testing.Task;
import sbt.testing.TaskDef;
import sbt.testing.TestSelector;
import sbt.testing.TestWildcardSelector;

/**
 * The test runner of `teq test` over sbt's test interface, one JVM per test configuration
 * kept warm by the daemon (docs/TARGETS.md, "The export and the project verbs"). Its own class path holds this class,
 * test-interface, the frameworks and the libraries, none of which a build rewrites; the class
 * and resource directories, which a build does rewrite, are read through a class loader made
 * afresh for every run, so that a run never sees the classes of the one before.
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
 * run &lt;verbose&gt; &lt;ansi&gt;               then "path &lt;directory&gt;" and "suite &lt;framework&gt; &lt;fingerprint&gt; &lt;name&gt;" lines and
 *                                    "go": the suites run, their output written, then "done &lt;failed&gt; &lt;passed&gt; &lt;ms&gt;"
 * ping                               answered by "pong"
 * quit
 * </pre>
 * Every line of the protocol it writes opens with the token, so that no output of a suite is
 * taken for one; a suite's stdout and stderr are this process's stdout, in the order written.
 */
public final class Runner {
    private final String token;
    private final PrintStream out;
    private final List<String> frameworks = new ArrayList<>();
    private final List<String[]> arguments = new ArrayList<>();
    private final List<Fingerprint[]> fingerprints = new ArrayList<>();

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
                case "run" -> runner.run(fields[1].equals("1"), fields[2].equals("1"), in);
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
                fingerprints.add(prints);
                for (int j = 0; j < prints.length; j++) {
                    if (prints[j] instanceof SubclassFingerprint s) {
                        say("fingerprint", i, j, "subclass", s.isModule(), s.superclassName());
                    } else if (prints[j] instanceof AnnotatedFingerprint a) {
                        say("fingerprint", i, j, "annotated", a.isModule(), a.annotationName());
                    }
                }
            } catch (Throwable e) {
                fingerprints.add(new Fingerprint[0]);
                say("missing", i, e);
            }
        }
        say("ready");
    }

    private Framework framework(int i) throws ReflectiveOperationException {
        return (Framework) Class.forName(frameworks.get(i)).getDeclaredConstructor().newInstance();
    }

    private void run(boolean verbose, boolean ansi, BufferedReader in) throws IOException {
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
        Counts counts = new Counts(verbose);
        Logger[] loggers = { new Forwarded(verbose, ansi) };
        Thread thread = Thread.currentThread();
        ClassLoader context = thread.getContextClassLoader();
        try (URLClassLoader loader = new URLClassLoader(urls.toArray(new URL[0]), Runner.class.getClassLoader())) {
            thread.setContextClassLoader(loader);
            for (int i = 0; i < frameworks.size(); i++) {
                List<TaskDef> defs = new ArrayList<>();
                for (String[] s : suites) {
                    if (Integer.parseInt(s[1]) == i) {
                        Fingerprint print = fingerprints.get(i)[Integer.parseInt(s[2])];
                        defs.add(new TaskDef(s[3], print, false, new Selector[] { new SuiteSelector() }));
                    }
                }
                if (!defs.isEmpty()) {
                    runFramework(i, defs, loader, loggers, counts);
                }
            }
        } finally {
            thread.setContextClassLoader(context);
        }
        out.flush();
        say("done", counts.failed, counts.passed, (System.nanoTime() - start) / 1_000_000);
    }

    private void runFramework(int i, List<TaskDef> defs, ClassLoader loader, Logger[] loggers, Counts counts) {
        sbt.testing.Runner runner;
        Task[] tasks;
        try {
            runner = framework(i).runner(arguments.get(i), new String[0], loader);
            tasks = runner.tasks(defs.toArray(new TaskDef[0]));
        } catch (Throwable e) {
            for (TaskDef d : defs) {
                counts.crashed(d.fullyQualifiedName(), e);
            }
            return;
        }
        execute(tasks, counts, loggers);
        try {
            String summary = runner.done();
            if (summary != null && !summary.isEmpty()) {
                out.println(summary);
            }
        } catch (Throwable e) {
            counts.crashed(frameworks.get(i), e);
        }
    }

    private void execute(Task[] tasks, Counts counts, Logger[] loggers) {
        for (Task task : tasks) {
            String suite = task.taskDef().fullyQualifiedName();
            EventHandler handler = event -> counts.handle(suite, event);
            try {
                execute(task.execute(handler, loggers), counts, loggers);
            } catch (Throwable e) {
                counts.crashed(suite, e);
            }
        }
    }

    /**
     * The passes and failures of a run, each failure printed as it comes with the first line of
     * its throwable and the throwable's frames in the suite's code; the whole trace when verbose.
     */
    private final class Counts {
        private final boolean verbose;
        int failed;
        int passed;

        Counts(boolean verbose) {
            this.verbose = verbose;
        }

        synchronized void handle(String suite, Event event) {
            switch (event.status()) {
                case Success -> passed++;
                case Failure, Error -> {
                    failed++;
                    failure(suite, named(event), event.throwable().isDefined() ? event.throwable().get() : null);
                }
                default -> { }
            }
        }

        /** A suite whose task threw: its initialiser, or the framework's own code. */
        synchronized void crashed(String suite, Throwable e) {
            failed++;
            failure(suite, suite + " crashed", e);
        }

        /** The failure, its throwable, and the throwable's frames in the suite's own code. */
        private void failure(String suite, String what, Throwable e) {
            if (e == null) {
                out.println("[fail] " + what);
                return;
            }
            if (verbose) {
                out.print("[fail] " + what + ": ");
                e.printStackTrace(out);
                return;
            }
            out.println("[fail] " + what + ": " + String.valueOf(e).lines().findFirst().orElse(""));
            Throwable cause = e;
            while (cause.getCause() != null && cause.getStackTrace().length == 0) {
                cause = cause.getCause();
            }
            StackTraceElement[] frames = cause.getStackTrace();
            int shown = 0;
            for (StackTraceElement frame : frames) {
                String owner = frame.getClassName();
                boolean own = owner.equals(suite) || owner.startsWith(suite + "$");
                if (own && shown < 5) {
                    out.println("[fail]     at " + frame);
                    shown++;
                }
            }
            if (shown == 0 && frames.length > 0) {
                out.println("[fail]     at " + frames[0]);
            }
            if (cause != e) {
                out.println("[fail]   caused by " + cause);
            }
        }
    }

    /** The test an event is of: its name and the test its selector names, said once. */
    private static String named(Event event) {
        String name = event.fullyQualifiedName();
        Selector selector = event.selector();
        String test = "";
        if (selector instanceof TestSelector s) {
            test = s.testName();
        } else if (selector instanceof NestedTestSelector s) {
            test = s.suiteId() + " " + s.testName();
        } else if (selector instanceof NestedSuiteSelector s) {
            test = s.suiteId();
        } else if (selector instanceof TestWildcardSelector s) {
            test = s.testWildcard();
        }
        return test.isEmpty() || test.equals(name) ? name : test.startsWith(name) ? test : name + " " + test;
    }

    /** A framework's logger: info, warnings and errors always, debug and traces when verbose. */
    private final class Forwarded implements Logger {
        private final boolean verbose;
        private final boolean ansi;

        Forwarded(boolean verbose, boolean ansi) {
            this.verbose = verbose;
            this.ansi = ansi;
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
            if (verbose) {
                t.printStackTrace(out);
            }
        }

        private void print(String prefix, String message) {
            synchronized (out) {
                message.lines().forEach(line -> out.println(prefix + line));
            }
        }
    }

    private void say(Object... fields) {
        StringBuilder line = new StringBuilder(token);
        for (Object field : fields) {
            line.append('\t').append(String.valueOf(field).replace('\t', ' ').replace('\n', ' ').replace('\r', ' '));
        }
        synchronized (out) {
            out.println(line);
        }
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
