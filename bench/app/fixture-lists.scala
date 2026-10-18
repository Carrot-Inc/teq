//> using file ../../tools/script
// The public stand-in for the application's lists: the generated corpus's API side with the model it shares
// (bench/app/gen.py; the budget's realistic-api program, bench/programs.sh) in the format bench/app/app-lists.sh
// writes, for what reads those lists (bench/pairs.py api, bench/app/api-check.sh) on a machine without the
// application. Run from the repository's root:
//
//   ./teq interp bench/app/fixture-lists.scala -- <dir>
//
// The corpus is generated under out/budget/app as bench/programs.sh generates it (again when bench/app/gen.py is
// newer), and <dir> gets api-modules.txt (its header, then the shared tree and the API's, upstream first, relative to
// the repository's root), api-classpath.txt (the program's jars, a line each), api-flags.txt (no flags) and
// lists.env, which exports APP_ROOT (the repository's root), APP_MODULES, APP_CLASSPATH and APP_FLAGS. Status 2 when
// a jar is not in the coursier cache, 1 when the corpus is not generated.
import java.nio.file.{Files, Paths}

@main def fixtureLists(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/app/fixture-lists.scala <dir>")
  val dir = Paths.get(args.exactly(1)(0)).toAbsolutePath
  val root = Paths.get("").toAbsolutePath
  val r = Sh("bash", "-c", "work=out/budget; . bench/programs.sh > /dev/null || exit 1; [ -z \"$left_out\" ] || { printf '%s' \"$left_out\"; exit 3; }; " +
    "printf '%s\\n' $(echo \"$app_cp\" | tr ':' ' ')").check(false).run()
  if r.status == 3 then
    System.err.println(s"fixture-lists: not in the coursier cache: ${r.out.trim}")
    Script.exit(2)
  if !r.ok || !Files.isDirectory(root.resolve("out/budget/app/api")) then
    System.err.println(s"fixture-lists: the corpus was not generated (bench/programs.sh): ${r.errTail}")
    Script.exit(1)
  val jars = r.lines.filter(_.nonEmpty)
  Files.createDirectories(dir)
  def write(name: String, text: String) = Files.writeString(dir.resolve(name), text)
  write("api-modules.txt", "# the generated corpus's API and the model it shares, upstream first (bench/app/fixture-lists.scala)\n" +
    "out/budget/app/shared\nout/budget/app/api\n")
  write("api-classpath.txt", jars.mkString("", "\n", "\n"))
  write("api-flags.txt", "\n")
  def quoted(s: String) = "'" + s.replace("'", "'\\''") + "'"
  write("lists.env", Vector("APP_ROOT" -> root.toString, "APP_MODULES" -> dir.resolve("api-modules.txt").toString,
    "APP_CLASSPATH" -> dir.resolve("api-classpath.txt").toString, "APP_FLAGS" -> "")
    .map((k, v) => s"export $k=${quoted(v)}").mkString("", "\n", "\n"))
  println(s"fixture-lists: ${dir.resolve("lists.env")}, 2 modules, ${jars.length} jars")
}
