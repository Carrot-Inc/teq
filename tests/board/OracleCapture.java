// OracleCapture.java: scalac's diagnostics of many tests in one JVM, recorded AS THEY ARE REPORTED: one entry a diagnostic, in
// the order the compiler's reporter receives them, across a test's groups. The reporter is scalac's AbstractReporter (so
// its UniqueMessagePositions and HideNonSensicalMessages filters hold, as in vulpix's TestReporter); each entry carries the
// diagnostic's level, its position or none, its message id or none, and its text as vulpix's console renders that one
// diagnostic (MessageRendering.messageAndPos: compiler/test/dotty/tools/dotc/reporting/TestReporter.scala).
//
// Usage: java -cp <scalac's class path>:<this class> OracleCapture <manifest> <out dir> <threads> <limit seconds> <javac>
//   manifest: one test a line: id \t group \x1e group ... ; a group: scalac's arguments \x1f-separated, then optionally
//   \x1d and javac's arguments (vulpix's compileWithJavac after the group's compilation).
//   out: <id>.jsonl (one JSON object a diagnostic: group, level, kind (the Diagnostic's class), id, file, line, column,
//   startColumn (0-based, as SourcePosition gives them; -1 where the diagnostic has no position), console (vulpix's
//   console prints it), phase (the reporting phase's name), rendered) and <id>.status (one line a
//   step: group, step (scalac|javac), outcome (ok|exception|timeout|launch|exit N), detail).
// The working directory is the corpus root: the paths are relative to it, as vulpix's.
import dotty.tools.dotc.core.Contexts;
import dotty.tools.dotc.reporting.AbstractReporter;
import dotty.tools.dotc.reporting.Diagnostic;
import dotty.tools.dotc.util.SourcePosition;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;

public class OracleCapture {
  static String json(String s) {
    StringBuilder b = new StringBuilder("\"");
    for (char c : s.toCharArray()) {
      switch (c) {
        case '"': b.append("\\\""); break;
        case '\\': b.append("\\\\"); break;
        case '\n': b.append("\\n"); break;
        case '\r': b.append("\\r"); break;
        case '\t': b.append("\\t"); break;
        default: if (c < 0x20) b.append(String.format("\\u%04x", (int) c)); else b.append(c);
      }
    }
    return b.append('"').toString();
  }

  // AbstractReporter: Reporter with UniqueMessagePositions, HideNonSensicalMessages and MessageRendering, the mixins of
  // ConsoleReporter and of vulpix's TestReporter (reporting/AbstractReporter.scala: "easier to extend from Java").
  static final class Recorder extends AbstractReporter {
    final int group; final List<String> out;
    Recorder(int group, List<String> out) { this.group = group; this.out = out; }
    // vulpix's TestReporter prints no summary (`override def printSummary()(using Context): Unit = ()`)
    @Override public void printSummary(Contexts.Context ctx) { }
    @Override public void doReport(Diagnostic dia, Contexts.Context ctx) {
      String rendered = messageAndPos(dia, ctx);
      // vulpix's console prints it (TestConsoleReporter.isDiagnosticRelevant): a WARNING or above, or an info of a phase
      // that -Ylog names (Decorators.containsPhase)
      boolean console;
      String phase;
      try {
        @SuppressWarnings("unchecked")
        scala.collection.immutable.List<String> ylog = (scala.collection.immutable.List<String>) ctx.settings().Ylog().value(ctx.settings().Ylog(), ctx);
        console = dia.level() >= 1 || dotty.tools.dotc.core.Decorators$.MODULE$.containsPhase(ylog, ctx.phase());
        phase = ctx.phase().phaseName();
      } catch (RuntimeException e) { console = dia.level() >= 1; phase = "?"; }
      // the position as vulpix keys the marks and as the box's head names it: the call site of an inlined position
      // (ParallelTesting.scala: `d.pos.nonInlined`)
      SourcePosition pos = dia.pos() == null ? null : dia.pos().nonInlined();
      boolean placed = pos != null && pos.exists();
      String file = placed ? pos.source().file().path() : null;
      int n = dia.msg().errorId().errorNumber();
      String id = n >= 0 ? String.format("E%03d", n) : null;
      synchronized (out) {
        out.add("{\"group\":" + group + ",\"level\":" + dia.level() + ",\"kind\":" + json(dia.getClass().getSimpleName())
            + ",\"id\":" + (id == null ? "null" : json(id)) + ",\"file\":" + (file == null ? "null" : json(file))
            + ",\"line\":" + (placed ? pos.line() : -1) + ",\"column\":" + (placed ? pos.column() : -1)
            + ",\"startColumn\":" + (placed ? pos.startColumn() : -1) + ",\"console\":" + console + ",\"phase\":" + json(phase)
            + ",\"rendered\":" + json(rendered) + "}");
      }
    }
  }

  public static void main(String[] argv) throws Exception {
    List<String> lines = Files.readAllLines(Paths.get(argv[0]), StandardCharsets.UTF_8);
    Path out = Paths.get(argv[1]);
    int threads = Integer.parseInt(argv[2]);
    long limit = Long.parseLong(argv[3]);
    String javacCmd = argv[4];
    ThreadFactory tf = r -> { Thread t = new Thread(null, r, "capture", 2L << 20); t.setDaemon(true); return t; };
    ExecutorService pool = Executors.newFixedThreadPool(threads, tf);
    ExecutorService watch = Executors.newCachedThreadPool(tf);
    List<Future<?>> all = new ArrayList<>();
    for (String line : lines) {
      if (line.isEmpty()) continue;
      String[] parts = line.split("\t", 2);
      String id = parts[0];
      String[] groups = parts[1].split("\u001e", -1);
      all.add(pool.submit(() -> {
        List<String> diags = new ArrayList<>();
        StringBuilder status = new StringBuilder();
        for (int i = 0; i < groups.length; i++) {
          String[] both = groups[i].split("\u001d", -1);
          String[] args = both[0].split("\u001f");
          String[] javac = both.length > 1 && !both[1].isEmpty() ? both[1].split("\u001f") : null;
          Recorder rep = new Recorder(i, diags);
          String outcome = "ok", detail = "";
          Future<?> f = watch.submit(() -> { dotty.tools.dotc.Main.process(args, rep, null); return null; });
          try { f.get(limit, TimeUnit.SECONDS); }
          catch (TimeoutException e) { outcome = "timeout"; detail = "over " + limit + " s"; f.cancel(true); }
          catch (ExecutionException e) { outcome = "exception"; detail = String.valueOf(e.getCause()); }
          catch (InterruptedException e) { outcome = "exception"; detail = "interrupted"; }
          status.append(i).append("\tscalac\t").append(outcome).append('\t').append(detail.replace('\t', ' ').replace('\n', ' ')).append('\n');
          if (outcome.equals("timeout")) break;
          if (javac != null) {
            String jo = "ok", jd = "";
            try {
              List<String> cmd = new ArrayList<>(List.of(javacCmd));
              cmd.addAll(Arrays.asList(javac));
              Process p = new ProcessBuilder(cmd).redirectErrorStream(true).start();
              String text = new String(p.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
              int code = p.waitFor();
              if (code != 0) { jo = "exit " + code; jd = text; }
            } catch (IOException e) { jo = "launch"; jd = String.valueOf(e); }
            catch (InterruptedException e) { jo = "launch"; jd = "interrupted"; }
            status.append(i).append("\tjavac\t").append(jo).append('\t').append(jd.replace('\t', ' ').replace('\n', ' ')).append('\n');
          }
        }
        try {
          Files.writeString(out.resolve(id + ".jsonl"), String.join("\n", diags) + (diags.isEmpty() ? "" : "\n"), StandardCharsets.UTF_8);
          Files.writeString(out.resolve(id + ".status"), status.toString(), StandardCharsets.UTF_8);
        } catch (IOException e) { throw new UncheckedIOException(e); }
        System.out.println("done " + id);
        System.out.flush();
        return null;
      }));
    }
    for (Future<?> f : all) { try { f.get(); } catch (ExecutionException e) { System.out.println("failed " + e.getCause()); } }
    System.out.println("all done");
    System.exit(0);
  }
}
