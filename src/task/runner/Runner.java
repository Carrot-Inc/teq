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
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Set;

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
 *                                    "go": the suites run, their output written and their events sent as they come,
 *                                    then "done &lt;ms&gt;"
 * ping                               answered by "pong"
 * quit
 * </pre>
 * An event is sent typed, for the daemon to count and report (src/task/events.rs): "event &lt;suite&gt; &lt;status
 * ordinal&gt; &lt;name&gt; &lt;selector&gt; &lt;a&gt; &lt;b&gt; &lt;ms&gt;", the selector suite, test, nestedSuite, nestedTest or wildcard
 * with its names in a and b; a suite whose task, or a framework whose runner, threw is "crashed &lt;name&gt;". Before
 * either, what it threw: "throwable &lt;toString&gt;", then "cause &lt;toString&gt;" for each cause, each followed by a
 * "frame &lt;class&gt; &lt;toString&gt;" per frame of its stack.
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
        Events events = new Events();
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
                    runFramework(i, defs, loader, loggers, events);
                }
            }
        } finally {
            thread.setContextClassLoader(context);
        }
        out.flush();
        say("done", (System.nanoTime() - start) / 1_000_000);
    }

    private void runFramework(int i, List<TaskDef> defs, ClassLoader loader, Logger[] loggers, Events events) {
        sbt.testing.Runner runner;
        Task[] tasks;
        try {
            runner = framework(i).runner(arguments.get(i), new String[0], loader);
            tasks = runner.tasks(defs.toArray(new TaskDef[0]));
        } catch (Throwable e) {
            for (TaskDef d : defs) {
                events.crashed(d.fullyQualifiedName(), e);
            }
            return;
        }
        execute(tasks, events, loggers);
        try {
            String summary = runner.done();
            if (summary != null && !summary.isEmpty()) {
                out.println(summary);
            }
        } catch (Throwable e) {
            events.crashed(frameworks.get(i), e);
        }
    }

    private void execute(Task[] tasks, Events events, Logger[] loggers) {
        for (Task task : tasks) {
            String suite = task.taskDef().fullyQualifiedName();
            EventHandler handler = event -> events.handle(suite, event);
            try {
                execute(task.execute(handler, loggers), events, loggers);
            } catch (Throwable e) {
                events.crashed(suite, e);
            }
        }
    }

    /** The events of a run, each sent typed as it comes, with what it threw. */
    private final class Events {
        synchronized void handle(String suite, Event event) {
            thrown(event.throwable().isDefined() ? event.throwable().get() : null);
            Selector selector = event.selector();
            String kind = "suite";
            String a = "";
            String b = "";
            if (selector instanceof TestSelector s) {
                kind = "test";
                a = s.testName();
            } else if (selector instanceof NestedTestSelector s) {
                kind = "nestedTest";
                a = s.suiteId();
                b = s.testName();
            } else if (selector instanceof NestedSuiteSelector s) {
                kind = "nestedSuite";
                a = s.suiteId();
            } else if (selector instanceof TestWildcardSelector s) {
                kind = "wildcard";
                a = s.testWildcard();
            }
            say("event", suite, event.status().ordinal(), event.fullyQualifiedName(), kind, a, b, event.duration());
        }

        /** A suite whose task threw (its initialiser, or the framework's own code), or a framework's runner. */
        synchronized void crashed(String name, Throwable e) {
            thrown(e);
            say("crashed", name);
        }

        /** The throwable, its causes, and the frames of each; a cause met again ends the chain. */
        private void thrown(Throwable e) {
            if (e == null) {
                return;
            }
            Set<Throwable> seen = Collections.newSetFromMap(new IdentityHashMap<>());
            String line = "throwable";
            for (Throwable t = e; t != null && seen.add(t); t = t.getCause()) {
                say(line, t);
                for (StackTraceElement frame : t.getStackTrace()) {
                    say("frame", frame.getClassName(), frame);
                }
                line = "cause";
            }
        }
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
