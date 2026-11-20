import dotty.tools.dotc.reporting.Diagnostic;
import dotty.tools.dotc.reporting.StoreReporter;
import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/** scalac 3.8.4 over many programs in one JVM: `Judge <classpath> <list>`, the list a directory a line, each directory's
 *  .java and .scala sources one compilation into its own `scalac-out`. Prints `<directory>\taccepts`,
 *  `<directory>\trefuses: <line>:<column>: <the first error's first line>\t<every error: id@file:line:column, by commas>`
 *  (the line 1-based, the column 0-based, as scalac prints them), or `<directory>\tcrashes: <the exception>` where the
 *  compiler ends by an exception before any error. */
public class Judge {
  public static void main(String[] a) throws Exception {
    String cp = a[0];
    for (String d : Files.readAllLines(Path.of(a[1]))) {
      if (d.isEmpty()) continue;
      List<String> args = new ArrayList<>(List.of("-color:never", "-classpath", cp, "-d", d + "/scalac-out"));
      new File(d + "/scalac-out").mkdirs();
      File[] fs = new File(d).listFiles((x, n) -> n.endsWith(".java") || n.endsWith(".scala"));
      java.util.Arrays.sort(fs);
      for (File f : fs) args.add(f.getPath());
      StoreReporter r = new StoreReporter(null, false);
      String verdict;
      String crash = null;
      try {
        dotty.tools.dotc.Main.process(args.toArray(new String[0]), r, null);
      } catch (Throwable t) {
        crash = t.getClass().getName();
      }
      if (r.hasErrors()) {
        Diagnostic.Error e = r.allErrors().head();
        String msg = e.message().split("\n", 2)[0].trim();
        StringBuilder all = new StringBuilder();
        scala.collection.Iterator<Diagnostic.Error> it = r.allErrors().iterator();
        while (it.hasNext()) {
          Diagnostic.Error x = it.next();
          if (all.length() > 0) all.append(',');
          all.append(x.msg().errorId()).append('@')
             .append(x.pos().exists() ? x.pos().source().file().name() + ":" + (x.pos().line() + 1) + ":" + x.pos().column() : "-");
        }
        verdict = "refuses: " + (e.pos().exists() ? (e.pos().line() + 1) + ":" + e.pos().column() + ": " : "") + msg.replace('\t', ' ') + "\t" + all;
      } else if (crash != null) {
        verdict = "crashes: " + crash;
      } else {
        verdict = "accepts";
      }
      System.out.println(d + "\t" + verdict);
      System.out.flush();
    }
  }
}
