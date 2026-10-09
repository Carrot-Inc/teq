//> using file ../tools/script
// Output size: each program under tests/size is built, run under node against its .expected file
// (the output of the same program under Scala.js), and its byte count compared with the budgets
// in tests/size/budgets.txt: the development output, the --release output, and the release
// output through esbuild --minify and gzip, which is what a production bundle ships. Growth
// beyond 1% fails. SIZE_RECORD=1 rewrites the budgets, so that a deliberate change of size shows
// up in the diff of that file. esbuild is taken from $ESBUILD, the PATH or node_modules; without
// it the last column is neither measured nor checked. Run from the repository's root:
// `./teq interp tests/size.scala [-- --teq <binary>]`, the compiler `--teq`, else $TEQ, else
// target/release/teq.
import java.nio.file.{Files, Path, Paths}

@main def size(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/size.scala [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/size")) then Log.die("tests/size.scala runs from the repository's root")
  val budgets = Paths.get("tests/size/budgets.txt")
  val out = Paths.get("out/size")
  Files.createDirectories(out)
  val record = Option(System.getenv("SIZE_RECORD")).exists(_.nonEmpty)
  var pass = 0
  var fail = 0
  val recorded = new StringBuilder
  def bad(what: String): Unit =
    println(s"FAIL $what")
    fail += 1
  val home = System.getProperty("user.home")
  val esbuild: Option[Path] =
    Option(System.getenv("ESBUILD")).filter(_.nonEmpty).map(Paths.get(_))
      .orElse(Sh.which("esbuild"))
      .orElse(List(Paths.get("node_modules/.bin/esbuild"), Paths.get(home, ".config/yarn/global/node_modules/esbuild/bin/esbuild")).find(Files.isExecutable))
  if esbuild.isEmpty then println("note: esbuild not found, the minified column is skipped")
  val limits: Map[String, Vector[String]] =
    Files.readAllLines(budgets).toArray.map(_.toString.split("\\s+").toVector).filter(_.nonEmpty).map(cols => cols.head -> cols).toMap
  // Checks one measured size against its budget column.
  def budget(name: String, what: String, size: Long, column: Int): Unit =
    limits.get(name).flatMap(_.lift(column - 1)).filter(_ != "-") match
      case None => bad(s"$name has no $what budget")
      case Some(text) =>
        val limit = text.toLong
        if size > limit + limit / 100 then bad(s"$name $what is $size bytes, budget $limit")
        else
          pass += 1
          if size < limit - limit / 100 then println(s"note: $name $what is $size bytes, budget $limit; lower it with SIZE_RECORD=1")
  def same(a: Path, b: Path): Boolean = Files.exists(b) && java.util.Arrays.equals(Files.readAllBytes(a), Files.readAllBytes(b))
  // A program's build into `js` with its log, and its run under node against the expectation.
  def builds(src: Path, js: Path, log: Path, flags: String*): Boolean =
    Sh((Vector(teq, "compiler", "build", src.toString, "-o", js.toString) ++ flags)*).stdout(log).mergeErr.timeout(30).check(false).run().ok
  def runs(js: Path, actual: Path): Unit = Sh("node", js.toString).stdout(actual).mergeErr.timeout(20).check(false).run()
  val sources = Files.list(Paths.get("tests/size"))
  val programs = try sources.toArray.map(_.asInstanceOf[Path]).filter(_.toString.endsWith(".scala")).sortBy(_.getFileName.toString) finally sources.close()
  for src <- programs do
    val name = src.getFileName.toString.stripSuffix(".scala")
    val expected = Paths.get(s"tests/size/$name.expected")
    val dev = out.resolve(s"$name.js")
    val release = out.resolve(s"$name.release.js")
    if !builds(src, dev, out.resolve(s"$name.log")) then bad(s"$name build")
    else
      runs(dev, out.resolve(s"$name.actual"))
      if !same(expected, out.resolve(s"$name.actual")) then bad(s"$name output")
      else if !builds(src, release, out.resolve(s"$name.release.log"), "--release") then bad(s"$name release build")
      else
        runs(release, out.resolve(s"$name.release.actual"))
        if !same(expected, out.resolve(s"$name.release.actual")) then bad(s"$name release output")
        else
          val size = Files.size(dev)
          val releaseSize = Files.size(release)
          // The release output minified, then its gzip's byte count, through a file.
          val minified: Either[String, Option[Long]] = esbuild match
            case None => Right(None)
            case Some(es) =>
              val min = out.resolve(s"$name.min.js")
              if !Sh(es.toString, release.toString, "--minify").stdout(min).stderr(out.resolve(s"$name.min.log")).timeout(30).check(false).run().ok then Left(s"$name esbuild")
              else
                runs(min, out.resolve(s"$name.min.actual"))
                if !same(expected, out.resolve(s"$name.min.actual")) then Left(s"$name minified output")
                else
                  val gz = out.resolve(s"$name.min.js.gz")
                  Sh("gzip", "-9").stdinFrom(min).stdout(gz).run()
                  Right(Some(Files.size(gz)))
          minified match
            case Left(what) => bad(what)
            case Right(gzipped) =>
              recorded.append(s"$name $size $releaseSize ${gzipped.fold("-")(_.toString)}\n")
              if !record then
                budget(name, "dev", size, 2)
                budget(name, "release", releaseSize, 3)
                gzipped.foreach(g => budget(name, "release+esbuild+gzip", g, 4))
  if record then
    Files.writeString(budgets, recorded.toString)
    print(recorded)
    Script.exit(if fail == 0 then 0 else 1)
  println(s"$pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}
