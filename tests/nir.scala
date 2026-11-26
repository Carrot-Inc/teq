//> using file ../tools/script
// Tests of the NIR reader and writer: `teq nir` over the fixtures under tests/nir/fixtures (written by Scala
// Native's compiler plugin from tests/nir/src; see tests/nir/README) has to print, byte for byte,
// what Scala Native's own reader and printer printed for them, tests/nir/expected.txt; an input of no
// NIR file (a filter that selects none, a directory without one) is refused, naming it; damaged
// files have to be refused with a message, never a crash, and so has a file that nests deeper than
// the reader follows; and for each jar of tests/nir/digests.txt
// the listing has to have the SHA-256 of Scala Native's own. The writer has to write every fixture and
// every file of those jars again, from what the reader decoded, as the bytes the plugin wrote
// (`teq nir --rewrite`), a file it writes otherwise has to be named, and the damaged files must not crash it.
// A jar that is not in the coursier
// cache (tests/support/jars.sh names them for the fetch) is skipped on a line that says so, and
// passes nothing. Run from the repository's root:
// `./teq interp tests/nir.scala [-- --teq <binary>]`, the compiler `--teq`, else $TEQ, else
// target/release/teq.
import java.nio.file.{Files, Path, Paths}

@main def nir(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/nir.scala [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/nir")) then Log.die("tests/nir.scala runs from the repository's root")
  var pass = 0
  var fail = 0

  // The fixtures against Scala Native's own printing of them.
  val listing = Sh(teq, "nir", "tests/nir/fixtures").timeout(60).check(false).run()
  if listing.ok && java.util.Arrays.equals(Files.readAllBytes(listing.outFile), Files.readAllBytes(Paths.get("tests/nir/expected.txt"))) then pass += 1
  else
    fail += 1
    println(s"FAIL fixtures: exit code ${listing.status}")
    Sh("diff", listing.outFile.toString, "tests/nir/expected.txt").check(false).run().lines.take(20).foreach(println)
    listing.err.linesIterator.take(3).foreach(println)

  // A filter selects the files whose name holds it.
  val one = Sh(teq, "nir", "tests/nir/fixtures", "fix/Shape.nir").timeout(60).check(false).run()
  if one.ok && one.lines.count(_.startsWith("## ")) == 1 && one.lines.headOption.contains("## fix/Shape.nir") then pass += 1
  else
    fail += 1
    println("FAIL filter")
  // Each verb over an input of no NIR file, a filter's or a directory's, says so on one line and exits 1, printing
  // nothing else; over a filter that selects a file it does its work.
  val noNir = Files.createTempDirectory("teq-nir-empty.")
  Script.atExit(Sh.remove(noNir))
  val verbs: List[(List[String], String => Boolean)] = List(
    (Nil, (out: String) => out.linesIterator.count(_.startsWith("## ")) == 1 && out.startsWith("## fix/Shape.nir\n")),
    (List("--verify"), (out: String) => out.trim.matches("1 files, 4 definitions, listing sha256 [0-9a-f]{64}")),
    (List("--rewrite"), (out: String) => out.trim == "1 files, 4 definitions, 1 rewritten byte for byte, 0 differ"))
  for (verb, selects) <- verbs do
    val refusals = List(List("tests/nir/fixtures", "Nope") -> "tests/nir/fixtures: no NIR file whose name holds Nope", List(noNir.toString) -> s"$noNir: no NIR file")
    for (input, line) <- refusals do
      val r = Sh((List(teq, "nir") ++ verb ++ input)*).timeout(60).check(false).run()
      if r.status == 1 && r.out.isEmpty && r.err.trim == line then pass += 1
      else
        fail += 1
        println(s"FAIL teq nir ${(verb ++ input).mkString(" ")}: exit code ${r.status}, ${r.out.take(120)}${r.err.take(120)}")
    val r = Sh((List(teq, "nir") ++ verb ++ List("tests/nir/fixtures", "fix/Shape.nir"))*).timeout(60).check(false).run()
    if r.ok && selects(r.out) then pass += 1
    else
      fail += 1
      println(s"FAIL teq nir ${(verb ++ List("tests/nir/fixtures", "fix/Shape.nir")).mkString(" ")}: exit code ${r.status}, ${r.out.take(120)}")

  // Damaged input: a message and exit code 1, or a listing where the damage fell into a text,
  // never a panic (101) or a signal.
  val bad = Files.createTempDirectory("teq-nir.")
  Script.atExit(Sh.remove(bad))
  val data = Files.readAllBytes(Paths.get("tests/nir/fixtures/fix/Shapes$.nir"))
  for n <- List(0, 4, 44, 45, 46, 100, 1000, data.length / 2, data.length - 1) do Files.write(bad.resolve(s"trunc$n.nir"), data.take(n))
  var seed = 7L
  def draw(bound: Int): Int =
    seed = (seed * 6364136223846793005L + 1442695040888963407L)
    ((seed >>> 33) % bound).toInt
  for i <- 0 until 120 do
    val d = data.clone()
    for _ <- 0 to draw(4) do
      // Half of them within the prelude and the tables that follow it.
      val at = if i % 2 == 0 then draw(400) else draw(d.length)
      d(at) = draw(256).toByte
    Files.write(bad.resolve(s"corrupt$i.nir"), d)
  // A file nested deeper than the reader follows, with no circle in it: a method whose one instruction
  // jumps to a successor of 5000 handlers, each inside the one before.
  val sections = List(
    List(0, 0, 5, 0), // the table: the method, then its end
    List(1, 65, 0, 6, 68, 49, 102, 117, 69, 79), // the strings "A", "" and "D1fuEO"
    List(2, 0, 0), // a position in no file
    List(3, 3, 3, 2, 0, 1), // the symbols: the method A.f, the class A, none
    List(13, 0, 3, 18), // the types () => unit and unit
    List(4, 0, 0, 0, 0, 0, 0, 0), // the method: its type, its instructions, no names, no scopes
    List(), // no values
    List(1, 5, 0) ++ List.fill(5000)(List(2, 0, 3)).flatten ++ List(1))
  val starts = sections.scanLeft(45)(_ + _.length).init
  val prelude = (List(0x2e4e4952, 6, 11) ++ starts).flatMap(w => List(w >>> 24, w >>> 16, w >>> 8, w)) ++ List(0)
  Files.write(bad.resolve("deep.nir"), (prelude ++ sections.flatten).map(_.toByte).toArray)
  var crashed = false
  var refused = 0
  val damaged =
    val s = Files.list(bad)
    try s.toArray.toVector.map(_.asInstanceOf[Path]).sortBy(_.getFileName.toString)
    finally s.close()
  for f <- damaged do
    val r = Sh(teq, "nir", f.toString).timeout(30).check(false).run()
    if r.status == 1 then refused += 1
    if r.status > 1 || r.err.contains("panicked") then
      crashed = true
      println(s"FAIL ${f.getFileName}: exit code ${r.status}")
      r.err.linesIterator.take(3).foreach(println)
  if !crashed && refused >= 40 then pass += 1
  else
    fail += 1
    println(s"FAIL damaged input: $refused of ${damaged.length} refused")
  // The writer over what the reader makes of the damaged files: each written as it is, written otherwise or
  // refused, all of them counted, never a crash.
  val damagedAgain = Sh(teq, "nir", "--rewrite", bad.toString).timeout(60).check(false).run()
  if damagedAgain.status <= 1 && !damagedAgain.err.contains("panicked") && damagedAgain.lines.headOption.exists(_.startsWith(s"${damaged.length} files, ")) then
    pass += 1
  else
    fail += 1
    println(s"FAIL damaged input rewritten: exit code ${damagedAgain.status}, ${damagedAgain.lines.headOption.getOrElse("")}")
    damagedAgain.err.linesIterator.take(3).foreach(println)
  val short = Sh(teq, "nir", bad.resolve("trunc44.nir").toString).mergeErr.timeout(30).check(false).run()
  if short.status == 1 && short.out.contains("no NIR file: shorter than a prelude") then pass += 1
  else
    fail += 1
    println(s"FAIL a file shorter than a prelude: ${short.out}")
  val deep = Sh(teq, "nir", bad.resolve("deep.nir").toString).mergeErr.timeout(30).check(false).run()
  if deep.status == 1 && deep.out.contains("nested deeper than") then pass += 1
  else
    fail += 1
    println(s"FAIL a file nested deeper than the reader follows: exit code ${deep.status}, ${deep.out.take(120)}")

  // The writer: each fixture written again from what was read, byte for byte.
  val rewritten = Sh(teq, "nir", "--rewrite", "tests/nir/fixtures").mergeErr.timeout(60).check(false).run()
  if rewritten.ok && rewritten.out.trim == "30 files, 261 definitions, 30 rewritten byte for byte, 0 differ" then pass += 1
  else
    fail += 1
    println(s"FAIL fixtures rewritten: ${rewritten.out.linesIterator.take(5).mkString("\n")}")
  // A file that is not what the writer writes is named with the first byte that differs: the prelude's last
  // byte says that a file holds an entry point, and Shape.nir holds none.
  val flagged = Files.readAllBytes(Paths.get("tests/nir/fixtures/fix/Shape.nir"))
  flagged(44) = 1
  val flaggedFile = bad.resolve("flagged.nir")
  Files.write(flaggedFile, flagged)
  val named = Sh(teq, "nir", "--rewrite", flaggedFile.toString).mergeErr.timeout(30).check(false).run()
  val naming = List("1 files, 4 definitions, 0 rewritten byte for byte, 1 differ", s"differs: $flaggedFile: first difference at byte 44 of 247/247")
  if named.status == 1 && named.out.linesIterator.toList == naming then pass += 1
  else
    fail += 1
    println(s"FAIL a file the writer writes otherwise: exit code ${named.status}, ${named.out.take(300)}")

  // Whole jars against the digests of Scala Native's own listings, and written again byte for byte.
  val caches = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).toList ++
    List("/Library/Caches/Coursier/v1", "/.cache/coursier/v1").map(System.getProperty("user.home") + _)
  for line <- Files.readString(Paths.get("tests/nir/digests.txt")).linesIterator if line.nonEmpty && !line.startsWith("#") do
    val Array(sha, files, definitions, path) = line.split(" ")
    val name = path.substring(path.lastIndexOf('/') + 1)
    caches.map(c => Paths.get(s"$c/https/repo1.maven.org/maven2/$path")).find(Files.isRegularFile(_)) match
      case None => println(s"skip $name: not in the coursier cache")
      case Some(jar) =>
        val verified = Sh(teq, "nir", "--verify", jar.toString).mergeErr.timeout(120).check(false).run()
        val said = verified.out.trim
        if verified.ok && said == s"$files files, $definitions definitions, listing sha256 $sha" then pass += 1
        else
          fail += 1
          println(s"FAIL $name: $said")
        val again = Sh(teq, "nir", "--rewrite", jar.toString).mergeErr.timeout(120).check(false).run()
        if again.ok && again.out.trim == s"$files files, $definitions definitions, $files rewritten byte for byte, 0 differ" then pass += 1
        else
          fail += 1
          println(s"FAIL $name rewritten: ${again.out.linesIterator.take(5).mkString("\n")}")

  println(s"nir: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}
