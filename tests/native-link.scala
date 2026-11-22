//> using file ../tools/script
// Tests of the native link and lowering: the program of tests/nir/fixtures (main class fix.Shapes,
// written by Scala Native's compiler plugin; tests/nir/README) linked against the jars of Scala
// Native that tests/nir/digests.txt names. What `teq nir --link` and `teq nir --lower` print has to
// have the SHA-256 that tests/native-link/digests.txt records of Scala Native's own dumps of its
// link and its lowering, once as its build decides on threads and once with threads, and of each
// first text the definitions of the classes named below have to be, byte for byte, the ones kept
// whole in tests/native-link/linked.txt and lowered.txt (tests/native-link/README says what wrote
// the expectations). The same of the programs of tests/native-link/collisions, whose names share a
// hash (its README); the program of tests/native-link/diamond is linked at each depth its
// digests.txt names, within a bound. A link reads every jar: where one
// is not in the coursier cache (tests/support/jars.sh names them for the fetch) the suite says so
// on a skip line and passes nothing. Run from the repository's root:
// `./teq interp tests/native-link.scala [-- --teq <binary>]`, the compiler `--teq`, else $TEQ,
// else target/release/teq.
import java.nio.file.{Files, Path, Paths}

/** The definitions of the named classes in a listing, as they stand there. A definition starts on
 *  a line that is no label, no instruction and no closing brace, and the first name on that line
 *  (`@"T` or `@"M`, the length of the class's name, the name) is its own. */
def definitionsOf(listing: String, classes: Set[String]): String =
  val kept = new StringBuilder
  var keep = false
  var start = 0
  while start < listing.length do
    val newline = listing.indexOf('\n', start)
    val end = if newline < 0 then listing.length else newline + 1
    val first = listing.charAt(start)
    if first != ' ' && first != '%' && first != '}' then
      val name = listing.indexOf("@\"", start)
      var digits = name + 3
      while name >= 0 && digits < end && listing.charAt(digits).isDigit do digits += 1
      keep = name >= 0 && name < end && digits > name + 3 && {
        val length = listing.substring(name + 3, digits).toInt
        digits + length <= end && classes(listing.substring(digits, digits + length))
      }
    if keep then kept ++= listing.substring(start, end)
    start = end
  kept.toString

/** A run of a link: its exit code, whether its bound stopped it, its time and what it listed. */
case class Link(status: Int, timedOut: Boolean, millis: Long, listing: Array[Byte])

def linkOf(run: Sh.Result): Link = Link(run.status, run.timedOut, run.millis, Files.readAllBytes(run.outFile))

def sha256Of(bytes: Array[Byte]): String =
  java.security.MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

/** The classes, traits and modules a listing defines, each with the number of its definitions: a definition's
 *  line (as `definitionsOf` tells one) whose kind is followed by its name, `trait @"T10diamond.A0"`. */
def scopesOf(listing: String): Map[String, Int] =
  val kinds = List("class @\"T", "trait @\"T", "module @\"T")
  val names = listing.linesIterator.filter(l => l.nonEmpty && !" %}".contains(l.charAt(0))).flatMap { line =>
    kinds.map(line.indexOf(_)).find(at => at == 0 || at > 0 && line.charAt(at - 1) == ' ').flatMap { at =>
      val from = line.indexOf("@\"T", at) + 3
      val digits = line.drop(from).takeWhile(_.isDigit)
      Option.when(digits.nonEmpty)(line.substring(from + digits.length).take(digits.toInt))
    }
  }
  names.toList.groupBy(identity).map((name, all) => name -> all.size)

/** Why a link fails its check, none where it passes: an exit code other than 0, a run its bound stopped or past
 *  it, a listing of another SHA-256 than `sha`; and of the classes and traits whose names `owned` takes, each of
 *  `once` defined other than once and any other one defined. */
def linkProblems(link: Link, bound: Double, sha: String, once: Set[String] = Set.empty, owned: String => Boolean = _ => false): List[String] =
  val scopes = scopesOf(new String(link.listing, java.nio.charset.StandardCharsets.UTF_8)).filter((name, _) => owned(name))
  val got = sha256Of(link.listing)
  List(
    Option.when(link.status != 0)(s"exit code ${link.status}"),
    Option.when(link.timedOut || link.millis > bound * 1000)(s"${link.millis} ms, past its bound of $bound s"),
    Option.when(got != sha)(s"${link.listing.count(_ == '\n')} lines of sha256 $got, not $sha")).flatten ++
    once.toList.sorted.filter(scopes.getOrElse(_, 0) != 1).map(name => s"$name defined ${scopes.getOrElse(name, 0)} times") ++
    scopes.keys.filterNot(once).toList.sorted.map(name => s"$name defined, which it should not reach")

/** Why a refusal fails its check: an exit code other than 1, anything listed, another line than `expected`. */
def refusalProblems(status: Int, listed: Int, err: String, expected: String): List[String] =
  List(
    Option.when(status != 1)(s"exit code $status"),
    Option.when(listed != 0)(s"$listed characters listed"),
    Option.when(err.trim != expected)(s"said ${err.take(300)}")).flatten

/** The depths of the diamond the suite links, with whose listing each expectation is: Scala Native's own link's,
 *  up to the depth it ends at, and teq's own past it (tests/native-link/diamond/README). */
val diamondDepths: List[(Int, String)] = (2 to 20).map(_ -> "scala-native").toList ++ List(28 -> "teq", 36 -> "teq")

/** The classes and traits of the diamond of a depth, which its link has to define once each. */
def diamondOf(depth: Int): Set[String] =
  (0 to depth).flatMap(i => List(s"diamond.A$i", s"diamond.B$i")).toSet + s"diamond.Leaf$depth"

def isDiamonds(name: String): Boolean = name.matches("diamond\\.(A|B|Leaf)[0-9]+")

/** Why the manifest of the diamond (its depths and whose listing each is) is not the suite's: a depth without its
 *  line or with two, a line of a depth the suite does not link, a depth whose expectation is another's. */
def manifestProblems(manifest: List[(Int, String)]): List[String] =
  val lines = manifest.groupBy(_._1)
  diamondDepths.flatMap { (depth, whose) =>
    lines.getOrElse(depth, Nil) match
      case List((_, `whose`)) => Nil
      case List((_, other)) => List(s"depth $depth: $other's, not $whose's")
      case found => List(s"depth $depth: ${found.size} lines")
  } ++ lines.keys.filterNot(diamondDepths.toMap.contains).toList.sorted.map(depth => s"depth $depth: not one the suite links")

/** The checks above judging records planted to fail, each of which has to fail, and sound records, which have to
 *  pass: how many of each, and what of them a check misjudges, none where every one holds. */
def selfTest(): (Int, Int, List[String]) =
  val heads = (diamondOf(2).toList.sorted.map(name => s"trait @\"T${name.length}$name\"").filterNot(_.contains("Leaf")) :+
    "class @\"T13diamond.Leaf2\" : @\"T16java.lang.Object\", @\"T10diamond.A2\", @\"T10diamond.B2\"") ++
    List("def @\"M13diamond.Leaf2RE\" : (@\"T13diamond.Leaf2\") => unit {", "  ret %1 : unit", "}",
      "module @\"T14diamond.Main2$\" : @\"T16java.lang.Object\"")
  val sound = (heads.mkString("\n") + "\n").getBytes("UTF-8")
  val bound = 60.0
  def judged(link: Link, sha: String = ""): List[String] =
    linkProblems(link, bound, if sha.isEmpty then sha256Of(link.listing) else sha, diamondOf(2), isDiamonds)
  def listing(lines: List[String]): Link = Link(0, false, 100, (lines.mkString("\n") + "\n").getBytes("UTF-8"))
  val good = Link(0, false, 100, sound)
  val wanted = sha256Of(sound)
  val planted: List[(String, List[String])] = List(
    "a listing with a class twice" -> judged(listing(heads :+ heads.find(_.contains("Leaf2")).get)),
    "a listing with a trait twice" -> judged(listing(heads :+ heads.head)),
    "a listing without a trait" -> judged(listing(heads.filterNot(_.contains("diamond.A1\"")))),
    "a listing without its class" -> judged(listing(heads.filterNot(_.startsWith("class")))),
    "a listing with a trait of a deeper diamond" -> judged(listing(heads :+ "trait @\"T10diamond.A3\"")),
    "a listing with the class of another depth" -> judged(listing(heads :+ "class @\"T13diamond.Leaf3\" : @\"T16java.lang.Object\"")),
    "an empty listing, of its own digest" -> judged(Link(0, false, 100, Array[Byte]())),
    "an empty listing" -> judged(Link(0, false, 100, Array[Byte]()), wanted),
    "a truncated listing" -> judged(Link(0, false, 100, sound.take(sound.length / 2)), wanted),
    "a listing of another byte" -> judged(Link(0, false, 100, sound.updated(5, 'X'.toByte)), wanted),
    "a run its bound stopped" -> judged(Link(124, true, 60000, sound), wanted),
    "a run past its bound" -> judged(Link(0, false, 60001, sound), wanted),
    "a run of exit code 1" -> judged(Link(1, false, 100, sound), wanted),
    "a refusal that exits 0" -> refusalProblems(0, 0, "the class path, entry 2: x: no such jar", "the class path, entry 2: x: no such jar"),
    "a refusal that lists" -> refusalProblems(1, 10, "the class path, entry 2: x: no such jar", "the class path, entry 2: x: no such jar"),
    "a refusal of another line" -> refusalProblems(1, 0, "the class path, entry 3: x: no such jar", "the class path, entry 2: x: no such jar"),
    "a manifest without a depth" -> manifestProblems(diamondDepths.filterNot(_._1 == 7)),
    "a manifest with a depth twice" -> manifestProblems(diamondDepths :+ diamondDepths.head),
    "a manifest with a depth the suite does not link" -> manifestProblems(diamondDepths :+ (21 -> "teq")),
    "a manifest with teq's listing for Scala Native's" -> manifestProblems(diamondDepths.map((d, w) => d -> (if d == 12 then "teq" else w))))
  val soundOnes: List[(String, List[String])] = List(
    "a sound listing" -> judged(good, wanted),
    "a sound refusal" -> refusalProblems(1, 0, "the class path, entry 2: x: no such jar\n", "the class path, entry 2: x: no such jar"),
    "the suite's manifest" -> manifestProblems(diamondDepths))
  (planted.size, soundOnes.size, planted.collect { case (what, Nil) => s"the checks pass $what" } ++
    soundOnes.collect { case (what, problems) if problems.nonEmpty => s"the checks fail $what: ${problems.mkString("; ")}" })

@main def nativeLink(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/native-link.scala [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/native-link")) then Log.die("tests/native-link.scala runs from the repository's root")
  var pass = 0
  var fail = 0

  // The checks of the cells below that call linkProblems, refusalProblems and manifestProblems, held to planted
  // records before any link.
  val (plantedCount, soundCount, misjudged) = selfTest()
  misjudged.foreach(m => println(s"FAIL self-test: $m"))
  if misjudged.isEmpty then println(s"native-link self-test: $plantedCount planted records failed, $soundCount sound ones passed")
  else fail += 1

  // The class path: the program's own files, then Scala Native's jars.
  val caches = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).toList ++
    List("/Library/Caches/Coursier/v1", "/.cache/coursier/v1").map(System.getProperty("user.home") + _)
  val wanted = Files.readString(Paths.get("tests/nir/digests.txt")).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split(" ").last).toList
  val found = wanted.map(path => path -> caches.map(c => Paths.get(s"$c/https/repo1.maven.org/maven2/$path")).find(Files.isRegularFile(_)))
  for (path, jar) <- found if jar.isEmpty do println(s"skip ${path.substring(path.lastIndexOf('/') + 1)}: not in the coursier cache")

  if found.forall(_._2.nonEmpty) then
    val sep = if java.io.File.separatorChar == '\\' then ";" else ":"
    val classPath = ("tests/nir/fixtures" :: found.map(_._2.get.toString)).mkString(sep)
    def sha256(file: Path): String =
      java.security.MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(file)).map(b => f"${b & 0xff}%02x").mkString
    def lineCount(file: Path): Int = Files.readAllBytes(file).count(_ == '\n')
    // The classes whose definitions are kept whole: a case class, a class of which the link keeps
    // little, a trait with a method of its own, a lambda's class and a module of C functions.
    val shown = Set("fix.Circle", "fix.Counters", "fix.Named", "fix.Shapes$$$Lambda$1", "fix.libm$")

    for line <- Files.readString(Paths.get("tests/native-link/digests.txt")).linesIterator if line.nonEmpty && !line.startsWith("#") do
      val Array(sha, lines, threads, what) = line.split(" ")
      val options = if threads == "detected" then Nil else List(s"--threads=$threads")
      val label = s"$what, threads $threads"
      what match
        case "linked" | "lowered" =>
          val verb = if what == "linked" then "--link" else "--lower"
          val listing = Sh((List(teq, "nir", verb, "fix.Shapes", classPath, "--triple=x86_64-unknown-linux-musl") ++ options)*).timeout(300).check(false).run()
          if listing.ok && sha256(listing.outFile) == sha then pass += 1
          else
            fail += 1
            println(s"FAIL $label: exit code ${listing.status}, ${lineCount(listing.outFile)} lines of sha256 ${sha256(listing.outFile)}, Scala Native's $lines of $sha")
            listing.err.linesIterator.take(5).foreach(println)
          // The classes kept whole, of the first link alone.
          if threads == "detected" then
            val cut = Files.createTempFile("teq-native-link.", ".txt")
            Script.atExit(Files.deleteIfExists(cut))
            Files.writeString(cut, definitionsOf(listing.outAll(), shown))
            val kept = s"tests/native-link/$what.txt"
            if listing.ok && java.util.Arrays.equals(Files.readAllBytes(cut), Files.readAllBytes(Paths.get(kept))) then pass += 1
            else
              fail += 1
              println(s"FAIL $label: the classes kept whole ($kept)")
              Sh("diff", cut.toString, kept).check(false).run().lines.take(20).foreach(println)
        case other =>
          fail += 1
          println(s"FAIL tests/native-link/digests.txt: no check of $other")

    // Programs whose names share a String.hashCode (tests/native-link/collisions/README): each one's link and lowering
    // byte for byte Scala Native's, a class's virtual methods and a trait's methods in the order of the set of its
    // called signatures, two of one hash in the order the link met them; of the first lowering, the definitions of
    // the program's own classes kept whole.
    val collisions = Paths.get("tests/native-link/collisions")
    val collisionPath = (collisions.resolve("nir").toString :: found.map(_._2.get.toString)).mkString(sep)
    for line <- Files.readString(collisions.resolve("digests.txt")).linesIterator if line.nonEmpty && !line.startsWith("#") do
      val Array(sha, lines, main, threads, what) = line.split(" ")
      val options = if threads == "detected" then Nil else List(s"--threads=$threads")
      val label = s"$main $what, threads $threads"
      val verb = if what == "linked" then "--link" else "--lower"
      val listing = Sh((List(teq, "nir", verb, main, collisionPath, "--triple=x86_64-unknown-linux-musl") ++ options)*).timeout(300).check(false).run()
      if listing.ok && sha256(listing.outFile) == sha then pass += 1
      else
        fail += 1
        println(s"FAIL $label: exit code ${listing.status}, ${lineCount(listing.outFile)} lines of sha256 ${sha256(listing.outFile)}, Scala Native's $lines of $sha")
        listing.err.linesIterator.take(5).foreach(println)
      if threads == "detected" && what == "lowered" then
        val pkg = main.substring(0, main.lastIndexOf('.'))
        val own = Files.list(collisions.resolve("nir").resolve(pkg.replace('.', '/'))).toArray.map(_.toString).toSet
          .map(f => pkg + "." + Paths.get(f).getFileName.toString.stripSuffix(".nir"))
        val kept = collisions.resolve(s"$main.txt")
        val cut = Files.createTempFile("teq-native-link.", ".txt")
        Script.atExit(Files.deleteIfExists(cut))
        Files.writeString(cut, definitionsOf(listing.outAll(), own))
        if listing.ok && java.util.Arrays.equals(Files.readAllBytes(cut), Files.readAllBytes(kept)) then pass += 1
        else
          fail += 1
          println(s"FAIL $label: the program's classes kept whole ($kept)")
          Sh("diff", cut.toString, kept.toString).check(false).run().lines.take(20).foreach(println)

    // What a link refuses: a main class the class path lacks, with the name and who refers to it.
    val missing = Sh(teq, "nir", "--link", "fix.Absent", classPath).mergeErr.timeout(300).check(false).run()
    if missing.status == 1 && missing.out.contains("not found on the native class path: fix.Absent") then pass += 1
    else
      fail += 1
      println(s"FAIL a main class the class path lacks: exit code ${missing.status}, ${missing.out.take(200)}")

    // What a link refuses before it links, as Scala Native's `VirtualDirectory.real` refuses it: an entry of the
    // class path that is neither a directory nor a jar that opens, named with its place and its number, and
    // nothing listed. The first is the review's: the class path in an argument file, a jar that is not there last.
    val scratch = Files.createTempDirectory("teq-native-link.")
    Script.atExit(Sh.remove(scratch))
    val last = classPath.split(sep).length + 1
    val absent = scratch.resolve("missing.jar").toString
    val listed = scratch.resolve("missing.cp").toString
    Files.writeString(Paths.get(listed), s"$classPath$sep$absent\n")
    Files.writeString(scratch.resolve("notes.txt"), "")
    Files.writeString(scratch.resolve("broken.jar"), "not a zip")
    val notes = scratch.resolve("notes.txt").toString
    val broken = scratch.resolve("broken.jar").toString
    val refusals = List(
      ("a jar that is not there, the class path in an argument file", List(s"@$listed", "--threads=false"), s"the class path, entry $last: $absent: no such jar"),
      ("a jar that is not there", List(s"$classPath$sep$absent"), s"the class path, entry $last: $absent: no such jar"),
      ("a jar that is not there after an empty entry, numbered among the entries kept", List(s"$sep$classPath$sep$absent"), s"the class path, entry $last: $absent: no such jar"),
      ("a file that is not a jar", List(s"$classPath$sep$notes"), s"the class path, entry $last: $notes: neither a jar nor a directory"),
      ("a directory that is not there", List(s"$classPath$sep$scratch/absent"), s"the class path, entry $last: $scratch/absent: neither a jar nor a directory"),
      ("a jar that does not open", List(s"$classPath$sep$broken"), s"the class path, entry $last: $broken: not a zip archive (no end of central directory)"))
    for (what, arguments, expected) <- refusals do
      val refused = Sh((List(teq, "nir", "--link", "fix.Shapes") ++ arguments)*).timeout(300).check(false).run()
      refusalProblems(refused.status, refused.out.length, refused.err, expected) match
        case Nil => pass += 1
        case problems =>
          fail += 1
          println(s"FAIL refused, $what: ${problems.mkString("; ")}")

    // An empty entry of the class path (a leading or trailing separator, or two together) is dropped, as teq's
    // --classpath drops it: the link is the one of the class path without it.
    val detected = Files.readString(Paths.get("tests/native-link/digests.txt")).linesIterator.collectFirst { case l if l.endsWith(" detected linked") => l.split(" ")(0) }.get
    val jars = found.map(_._2.get.toString).mkString(sep)
    val emptyEntries = List(
      "a leading separator" -> s"$sep$classPath",
      "a trailing separator" -> s"$classPath$sep",
      "two separators together" -> s"tests/nir/fixtures$sep$sep$jars")
    for (what, path) <- emptyEntries do
      val run = Sh(teq, "nir", "--link", "fix.Shapes", path, "--triple=x86_64-unknown-linux-musl").timeout(300).check(false).run()
      linkProblems(linkOf(run), 300, detected) match
        case Nil => pass += 1
        case problems =>
          fail += 1
          println(s"FAIL an empty entry, $what: ${problems.mkString("; ")}")

    // The diamond of traits (tests/native-link/diamond/README): each depth linked within the bound, its listing
    // the one recorded, defining each class and trait of its diamond once.
    val diamondBound = 60.0
    val diamondPath = s"tests/native-link/diamond/nir$sep$jars"
    val manifest = Files.readString(Paths.get("tests/native-link/diamond/digests.txt")).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split(" ")).toList
    manifestProblems(manifest.map(fields => fields(2).toInt -> fields(3))) match
      case Nil => pass += 1
      case problems =>
        fail += 1
        println(s"FAIL tests/native-link/diamond/digests.txt: ${problems.mkString("; ")}")
    for fields <- manifest do
      val Array(sha, _, depth, whose) = fields
      val run = Sh(teq, "nir", "--link", s"diamond.Main$depth", diamondPath, "--triple=x86_64-unknown-linux-musl").timeout(diamondBound).check(false).run()
      linkProblems(linkOf(run), diamondBound, sha, diamondOf(depth.toInt), isDiamonds) match
        case Nil => pass += 1
        case problems =>
          fail += 1
          println(s"FAIL the diamond of depth $depth, $whose's listing: ${problems.mkString("; ")}")

  println(s"native-link: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}
