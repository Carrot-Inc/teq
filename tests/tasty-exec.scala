//> using file ../tools/script
// The execution gate: the programs of tests/cases, and of tests/tasty/exec/cases (programs teq's own
// builds do not run as scalac's do yet), that tests/tasty/exec/closure.txt lists, whose products hold
// every body written (no `ELIDED`), compiled to class files by scalac 3.8.4 from teq's TASTy alone
// (-from-tasty, the tree checker after every phase: tests/tasty/fromtasty/Gate.scala's `regen`) and
// run on the JVM with scala-library alone beside them (tests/tasty/exec/Launch.java runs the main
// `mainOf` names), each printing its .expected, which scalac's own build printed. A listed program
// whose products withhold a body fails the line (the closure shrank); `--update` rewrites the list
// as every case without jars, Scala.js or the interpreter, of one main, that builds with every body
// written, regenerates and runs as expected, to be read against the previous list. Without
// scala-cli, java or javac a failure (incomplete validation). The products are built and the
// programs run as many at once as the machine has processors, the regenerations in a sixth of them
// (one to six). Run from the repository's root: `./teq interp tests/tasty-exec.scala [-- --update]
// [--teq <binary>]`, the compiler `--teq`, else $TEQ, else target/release/teq.
import java.nio.file.{Files, Path, Paths}
import java.util.regex.Pattern

@main def tastyExec(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/tasty-exec.scala [--update] [--teq <binary>]")
  val update = args.flag("update")
  val teq = Paths.get(args.teq("./target/release/teq")).toAbsolutePath.toString
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/tasty/exec")) then Log.die("tests/tasty-exec.scala runs from the repository's root")
  for tool <- List("scala-cli", "java", "javac") if Sh.which(tool).isEmpty do
    println(s"tasty-exec: no $tool (incomplete validation)")
    println("tasty-exec: 0 passed, 1 failed")
    Script.exit(1)
  val m2 = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).getOrElse(System.getProperty("user.home") + "/Library/Caches/Coursier/v1") + "/https/repo1.maven.org/maven2"
  val lib = s"$m2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
  val work = Files.createTempDirectory(Paths.get(Option(System.getenv("TMPDIR")).filter(_.nonEmpty).getOrElse("/tmp")), "tasty-exec.")
  Script.atExit(Sh.remove(work))
  for d <- List("p", "o", "logs", "launch") do Files.createDirectories(work.resolve(d))
  if !Sh("javac", "-d", work.resolve("launch").toString, "tests/tasty/exec/Launch.java").inherit.check(false).run().ok then
    println("tasty-exec: Launch.java does not compile")
    Script.exit(1)
  val closure = Paths.get("tests/tasty/exec/closure.txt")
  def listed(dir: String): Vector[Path] =
    val s = Files.list(Paths.get(dir))
    try s.toArray.toVector.map(_.asInstanceOf[Path]).sortBy(_.getFileName.toString) finally s.close()
  val names: Vector[String] =
    if update then
      // The files of tests/cases, its directories, then the files of tests/tasty/exec/cases.
      def named(dir: String, keep: Path => Boolean) =
        listed(dir).filter(p => keep(p) && !p.getFileName.toString.startsWith(".")).map(_.getFileName.toString.stripSuffix(".scala"))
      named("tests/cases", p => p.toString.endsWith(".scala") && !Files.isDirectory(p)) ++ named("tests/cases", Files.isDirectory(_)) ++
        named("tests/tasty/exec/cases", _.toString.endsWith(".scala"))
    else Files.readAllLines(closure).toArray.toVector.map(_.toString).filterNot(_.startsWith("#"))
  val cpus = Runtime.getRuntime.availableProcessors()

  // A case's sources: the file, or the directory's .scala files.
  def sources(src: Path): Vector[Path] =
    if Files.isDirectory(src) then listed(src.toString).filter(p => p.toString.endsWith(".scala") && !p.getFileName.toString.startsWith(".")) else Vector(src)
  def lines(src: Path): Vector[String] = sources(src).flatMap(f => Files.readString(f).linesIterator)

  // Each program's products, its census and its regeneration job; `why` says why one left.
  val why = scala.collection.mutable.Map.empty[String, String]
  val expected = scala.collection.mutable.Map.empty[String, Path]
  val mains = scala.collection.mutable.Map.empty[String, String]
  val jobs = scala.collection.mutable.Map.empty[String, String]
  Sh.pool(cpus, names) { name =>
    val dir = if Files.exists(Paths.get(s"tests/cases/$name.scala")) || Files.exists(Paths.get(s"tests/cases/$name")) then "tests/cases" else "tests/tasty/exec/cases"
    val src = if Files.isRegularFile(Paths.get(s"$dir/$name.scala")) then Paths.get(s"$dir/$name.scala") else Paths.get(s"$dir/$name")
    val text = if Files.exists(src) then lines(src) else Vector.empty
    if !Files.exists(src) then
      why(name) = "no case"
      Sh.End
    else if text.exists(l => l.startsWith("// jars:") || l.startsWith("//> using platform js") || (l.startsWith("// teq:") && l.contains("--target interp"))) then
      why(name) = "jars, Scala.js or the interpreter"
      Sh.End
    else if !Files.exists(Paths.get(s"$dir/$name.expected")) then
      why(name) = "no expected output"
      Sh.End
    else
      expected(name) = Paths.get(s"$dir/$name.expected")
      mainOf(src) match
        case None =>
          why(name) = "no main the source names alone"
          Sh.End
        case Some(main) =>
          mains(name) = main
          val flags = text.find(_.startsWith("// teq: ")).map(_.stripPrefix("// teq: ").split(" ").toVector.filter(_.nonEmpty)).getOrElse(Vector.empty)
          val kept = flags.filterNot(f => Set("--target", "jvm", "js", "--all-mains").contains(f))
          val products = work.resolve("p").resolve(name)
          val census = work.resolve(s"$name.census")
          val check = Sh((Vector(teq, "compiler", "check", "--products", products.toString, src.toString) ++ kept)*)
            .withEnv("TEQ_BODIES_CENSUS" -> census.toString).stdout(work.resolve("logs").resolve(s"$name.log")).mergeErr.timeout(60)
          Sh.Then(check, r => {
            if !r.ok then why(name) = "the build fails"
            else
              val withheld = (if Files.exists(census) then Files.readAllLines(census).toArray.toVector.map(_.toString) else Vector.empty)
                .map(_.split("\t", -1)).collectFirst { case f if f.length > 2 && f(1).startsWith("elided") && f(2).toLongOption.exists(_ > 0) => f(1).substring(8) }
              withheld match
                case Some(body) => why(name) = s"a body withheld: $body"
                case None =>
                  val tastys =
                    val s = Files.walk(products)
                    try s.toArray.toVector.map(_.toString).filter(_.endsWith(".tasty")).sorted finally s.close()
                  // The options the source gives scalac but its warnings' (`//> using option -Xmax-inlines 80`),
                  // which the regeneration's expansions of its inline bodies need as its compilation did.
                  val options = text.filter(l => l.startsWith("//> using option ") || l.startsWith("//> using options ")).map(_.replaceFirst("^//> using options? ", ""))
                    .flatMap(_.split(" ", -1)).filterNot(_.startsWith("-W")).map(_ + "\t").mkString
                  Files.createDirectories(work.resolve("o").resolve(name))
                  jobs(name) = s"regen\t$name\t$products\t${work.resolve("o").resolve(name)}\t$options${tastys.mkString("\t")}\n"
            Sh.End
          })
  }
  // The regenerations, the jobs dealt out to the parts in turn, the parts at once.
  val parts = (cpus / 4).max(1).min(6)
  val all = jobs.keys.toVector.sorted.map(jobs(_))
  for i <- 0 until parts do Files.writeString(work.resolve(s"part.$i"), all.indices.filter(_ % parts == i).map(all(_)).mkString)
  val root = Paths.get("").toAbsolutePath
  Sh.pool(parts, 0 until parts) { i =>
    Sh.Then(Sh("scala-cli", "--power", "run", "-S", "3.8.4", "--jvm", "system", "--server=false", "--offline", "-q",
      root.resolve("tests/tasty/fromtasty/Gate.scala").toString, "--dep", "org.scala-lang:scala3-compiler_3:3.8.4", "--", s"part.$i")
      .in(work).withEnv("COURSIER_MODE" -> "offline").stdout(work.resolve(s"part.$i.out")).stderr(work.resolve(s"part.$i.err")).timeout(330), _ => Sh.End)
  }
  val regen = (0 until parts).flatMap { i =>
    val f = work.resolve(s"part.$i.out")
    if Files.exists(f) then Files.readAllLines(f).toArray.toVector.map(_.toString) else Vector.empty
  }.toVector
  val regenerated = regen.filter(_.startsWith("ok ")).map(_.stripPrefix("ok ")).toSet
  // The regenerated programs run, each against its expectation.
  val ran = scala.collection.mutable.Set.empty[String]
  Sh.pool(cpus, names.filter(regenerated.contains)) { name =>
    val out = work.resolve("o").resolve(s"$name.out")
    val launch = Sh((Vector("java", "-XX:-UsePerfData", "-cp", s"${work.resolve("o").resolve(name)}:$lib:${work.resolve("launch")}", "Launch") ++ mains(name).split(" "))*)
      .stdout(out).stderr(work.resolve("o").resolve(s"$name.err")).timeout(20)
    Sh.Then(launch, r => {
      if r.ok && java.util.Arrays.equals(Files.readAllBytes(out), Files.readAllBytes(expected(name))) then ran += name
      Sh.End
    })
  }
  var pass = 0
  var fail = 0
  val kept = new StringBuilder
  for name <- names do
    if ran(name) then
      pass += 1
      kept.append(name).append('\n')
    else
      // In --update, each case left out of the closure with its reason.
      val left = if update then "left" else
        fail += 1
        "FAIL"
      if why.contains(name) then println(s"$left $name: ${why(name)}")
      else if !regenerated(name) then
        println(s"$left $name: scalac's regeneration")
        if !update then
          val block = regen.dropWhile(_ != s"FAIL $name").drop(1).takeWhile(l => !l.startsWith("ok ") && !l.startsWith("FAIL "))
          block.take(3).foreach(println)
      else
        println(s"$left $name: the regenerated program's run")
        if !update then
          val out = work.resolve("o").resolve(s"$name.out")
          Sh("diff", out.toString, expected(name).toString).check(false).run().lines.take(5).foreach(println)
          val err = work.resolve("o").resolve(s"$name.err")
          if Files.exists(err) then Files.readAllLines(err).toArray.take(3).foreach(println)
  if update then
    Files.writeString(closure, "# The programs of tests/cases and tests/tasty/exec/cases the execution gate runs (tests/tasty-exec.scala -- --update).\n" + kept)
    println(s"tasty-exec: $pass programs in the closure")
    Script.exit(0)
  println(s"tasty-exec: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}

// The class and the static method a case runs as its main, where the source names one: `p.S$package m`
// of `@main def m` in the file S.scala of the package p, `p.O main` of an object's `def main`; none
// where it names none or several. A `def main` of a trait or class makes the objects that inherit it
// mains too, so that a source with more `def main`s than objects' names none.
def mainOf(src: Path): Option[String] =
  val files =
    if Files.isRegularFile(src) then Vector(src)
    else
      val s = Files.list(src)
      try s.toArray.toVector.map(_.asInstanceOf[Path]).filter(_.toString.endsWith(".scala")).sortBy(_.toString) finally s.close()
  val found = scala.collection.mutable.ArrayBuffer.empty[String]
  var mains = 0
  val pkg = Pattern.compile("^package\\s+([\\w.]+)\\s*$", Pattern.MULTILINE)
  val entry = Pattern.compile("^@main\\s+def\\s+(\\w+)", Pattern.MULTILINE)
  val obj = Pattern.compile("^object\\s+(\\w+)[^\\n]*\\n(?:(?!^\\S).*\\n)*?\\s+def main\\(", Pattern.MULTILINE)
  val defs = Pattern.compile("\\bdef main\\(")
  for f <- files do
    val t = Files.readString(f)
    val pks = scala.collection.mutable.ArrayBuffer.empty[String]
    val pm = pkg.matcher(t)
    while pm.find() do pks += pm.group(1)
    val pre = if pks.nonEmpty then pks.mkString(".") + "." else ""
    val stem = f.getFileName.toString.dropRight(6).replace("-", "$minus")
    val em = entry.matcher(t)
    while em.find() do found += s"$pre$stem$$package ${em.group(1)}"
    val om = obj.matcher(t)
    while om.find() do found += s"$pre${om.group(1)} main"
    val dm = defs.matcher(t)
    while dm.find() do mains += 1
  if found.length == 1 && mains == found.count(_.endsWith(" main")) then Some(found.head) else None
