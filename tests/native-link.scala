//> using file ../tools/script
//> using file support/null-guards-judge.scala
// Tests of the native link, its lowering and its LLVM IR: the program of tests/nir/fixtures (main
// class fix.Shapes, written by Scala Native's compiler plugin; tests/nir/README) linked against the
// jars of Scala Native that tests/nir/digests.txt names. What `teq nir --link` prints, and what
// `teq nir --lower --null-guards=scala-native` prints and writes with --ll=<directory> (the lowering
// by Scala Native's own rule of where a null check is needed), have to have the SHA-256 that
// tests/native-link/digests.txt records of Scala Native's own output, once as its build decides on
// threads and once with threads; and of each first output the definitions of the classes named
// below have to be, byte for byte, the ones kept whole in tests/native-link/linked.txt, lowered.txt
// and ll.txt (tests/native-link/README says what wrote the expectations). Each lowered listing and
// module of teq's own rule (`teq nir --lower`, which checks a value again where a way into its block
// did not check it, and a monitor's receiver before it is entered) has to be the reference's or differ
// from it only by added null checks, as
// tests/support/null_guards.py judges them, every listing and module judged; the two rules' lowerings of a
// cell run at once, and the classifier beside the next cells' lowerings. The same of the programs
// of tests/native-link/collisions, whose names share a hash, and of tests/native-link/directories,
// whose sources lie in directories of one hash (their READMEs). Each digests file's records are read
// whole, a line of another form failing, and have to cover every cell of their program, each kind
// by name. The program of tests/native-link/diamond is linked at each depth its digests.txt names,
// within a bound. The judges of the digests, the modules, the links and the classifier's verdicts
// are held to planted records first, a self-test that needs no jar (`judgesSelfTest`,
// `linksSelfTest`, `guardsSelfTest`; the classifier runs its own each time). A link reads every jar:
// where one is not in the coursier cache (tests/support/jars.sh names them for the fetch) the suite
// says so on a skip line and passes nothing. Run from the repository's root:
// `./teq interp tests/native-link.scala [-- --teq <binary>]`, the compiler `--teq`, else $TEQ,
// else target/release/teq.
import java.nio.file.{Files, Path, Paths}

/** The definitions of the named classes in a listing or a module, as they stand there. A definition
 *  starts on a line that is no label (a NIR label starts with `%`, an LLVM one ends with `:`), no
 *  instruction, no closing brace and no empty line; the first name on that line is its own: `@"`,
 *  `_S` in a module, `T` or `M`, the length of the class's name, the name. */
def definitionsOf(text: String, classes: Set[String]): String =
  val kept = new StringBuilder
  var keep = false
  var start = 0
  while start < text.length do
    val newline = text.indexOf('\n', start)
    val end = if newline < 0 then text.length else newline + 1
    val first = text.charAt(start)
    if first != ' ' && first != '%' && first != '}' && first != '\n' && !text.substring(start, end).trim.endsWith(":") then
      val quote = text.indexOf("@\"", start)
      val kind = if text.startsWith("_S", quote + 2) then quote + 4 else quote + 2
      var digits = kind + 1
      while quote >= 0 && digits < end && text.charAt(digits).isDigit do digits += 1
      keep = quote >= 0 && quote < end && digits > kind + 1 && {
        val length = text.substring(kind + 1, digits).toInt
        digits + length <= end && classes(text.substring(digits, digits + length))
      }
    if keep then kept ++= text.substring(start, end)
    start = end
  kept.toString

def sha256(bytes: Array[Byte]): String =
  java.security.MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

def lineCount(bytes: Array[Byte]): Int = bytes.count(_ == '\n')

def filesIn(dir: Path): Vector[String] =
  val s = Files.list(dir)
  try s.toArray.toVector.map(_.asInstanceOf[Path].getFileName.toString)
  finally s.close()

/** A manifest of modules: of each, the SHA-256 of Scala Native's text, its lines and the module's file. */
type Manifest = List[(String, String, String)]

/** What is wrong with the modules a run of `teq nir --lower --ll=<dir>` left: nothing when the run ended well, the
 *  manifest names each file once and the directory holds each, of its digest, and no other file. */
def moduleProblems(ended: Boolean, dir: Path, manifest: Manifest): List[String] =
  val named = manifest.map(_._3)
  val twice = named.diff(named.distinct).distinct.map(name => s"$name: named twice by the expectations")
  val wrong = manifest.flatMap { case (sha, lines, name) =>
    val file = dir.resolve(name)
    if !Files.isRegularFile(file) then Some(s"$name: not written, Scala Native's $lines lines")
    else
      val bytes = Files.readAllBytes(file)
      Option.when(sha256(bytes) != sha)(s"$name: ${lineCount(bytes)} lines of sha256 ${sha256(bytes)}, Scala Native's $lines lines")
  }
  val stray = filesIn(dir).filterNot(named.contains).sorted.map(name => s"$name: not Scala Native's")
  Option.when(!ended)("the run did not end well").toList ++ Option.when(manifest.isEmpty)("no module expected").toList ++ twice ++ wrong ++ stray

/** The classes of `classes` whose methods a module defines: the first name of each `define` line, as definitionsOf
 *  reads one. */
def definedIn(text: String, classes: Set[String]): Set[String] =
  text.linesIterator.filter(_.startsWith("define ")).flatMap(line => classes.filter(c => definitionsOf(line + "\n", Set(c)).nonEmpty)).toSet

/** The modules of a program and their sources (tests/native-link/directories/modules.txt): of each, the module's
 *  file, the directory its sources lie in and their classes. */
type Owners = List[(String, String, Set[String])]

/** What is wrong with whose definitions the modules hold: each module of the owners defines methods of its own classes
 *  and of no other module's, whatever its digest. */
def ownerProblems(dir: Path, owners: Owners): List[String] =
  val all = owners.flatMap(_._3).toSet
  owners.flatMap { case (name, _, own) =>
    val file = dir.resolve(name)
    if !Files.isRegularFile(file) then List(s"$name: not written, so none of its classes defined")
    else
      val defined = definedIn(Files.readString(file), all)
      Option.when(defined.isEmpty)(s"$name: defines no method of ${own.toList.sorted.mkString(", ")}").toList ++
        (defined -- own).toList.sorted.map(c => s"$name: defines methods of $c, another directory's")
  }

/** A record of a digests file: the SHA-256 of Scala Native's text and its lines, the cell (the main class and how
 *  threads were decided), the kind (linked, lowered or ll) and, of ll, the module's file. */
final case class Record(sha: String, lines: String, main: String, threads: String, kind: String, module: String)

/** The records of a digests file and what is wrong with its lines. A line that is not empty or a comment is the
 *  SHA-256, the lines, the main class (where `main` gives none: a file of one program has no column of them), the
 *  threads (detected or true), the kind and, of ll alone, the module's file; a line of another form or kind, or a
 *  second record of one cell, kind and module, is wrong, never passed over. */
def readRecords(text: String, main: Option[String]): (List[Record], List[String]) =
  val read = text.linesIterator.zipWithIndex.filter { case (line, _) => line.nonEmpty && !line.startsWith("#") }.toList.map { case (line, i) =>
    val fields = line.split(" ", -1).toList
    val cell = (main, fields) match
      case (Some(m), sha :: lines :: threads :: kind :: rest) => Some((sha, lines, m, threads, kind, rest))
      case (None, sha :: lines :: m :: threads :: kind :: rest) => Some((sha, lines, m, threads, kind, rest))
      case _ => None
    val record = cell.collect {
      case (sha, lines, m, threads, kind, rest) if (kind == "linked" || kind == "lowered") && rest.isEmpty => Record(sha, lines, m, threads, kind, "")
      case (sha, lines, m, threads, "ll", List(module)) if module.endsWith(".ll") => Record(sha, lines, m, threads, "ll", module)
    }
    val wrong = record match
      case None => Some("not a record of a kind it knows (sha256, lines, " + (if main.isEmpty then "main class, " else "") + "threads, linked, lowered or ll and its module)")
      case Some(r) if !r.sha.matches("[0-9a-f]{64}") => Some("no SHA-256")
      case Some(r) if !r.lines.matches("[0-9]+") => Some("no count of lines")
      case Some(r) if r.threads != "detected" && r.threads != "true" => Some("threads neither detected nor true")
      case _ => None
    (record.filter(_ => wrong.isEmpty), wrong.map(w => s"line ${i + 1}: $w: $line"))
  }
  val records = read.flatMap(_._1)
  val twice = records.groupBy(r => (r.main, r.threads, r.kind, r.module)).toList.sortBy(_._1).collect {
    case ((m, threads, kind, module), rs) if rs.length > 1 => s"$m $kind${if module.isEmpty then "" else " " + module}, threads $threads: ${rs.length} records"
  }
  (records, read.flatMap(_._2) ++ twice)

/** What is wrong with the records against the cells they have to cover, each main class with threads detected and
 *  true: each cell has its linked record, its lowered record and its modules' (one at least, and each owner's by
 *  name), and no record is of another cell. */
def coverProblems(records: List[Record], mains: Set[String], owners: Owners): List[String] =
  val cells = for main <- mains.toList.sorted; threads <- List("detected", "true") yield (main, threads)
  val missing = cells.flatMap { case (main, threads) =>
    val of = records.filter(r => r.main == main && r.threads == threads)
    val modules = of.filter(_.kind == "ll").map(_.module)
    List("linked", "lowered").filterNot(k => of.exists(_.kind == k)).map(k => s"$main, threads $threads: no $k record") ++
      Option.when(modules.isEmpty)(s"$main, threads $threads: no module's record").toList ++
      owners.map(_._1).filterNot(modules.contains).map(name => s"$main, threads $threads: no record of $name")
  }
  val stray = records.filterNot(r => cells.contains((r.main, r.threads))).map(r => s"${r.main}, threads ${r.threads}: a record of no cell").distinct
  Option.when(mains.isEmpty)("no cell to cover").toList ++ missing ++ stray

/** The main classes of a directory of .nir files: those whose file defines a static `main(Array[String])`. */
def mainsIn(nir: Path): Set[String] =
  val s = Files.walk(nir)
  try s.toArray.toList.map(_.asInstanceOf[Path]).filter(_.getFileName.toString.endsWith(".nir"))
    .filter(f => new String(Files.readAllBytes(f), "ISO-8859-1").contains("D4mainLAL16java.lang.String_uEo"))
    .map(f => nir.relativize(f).toString.stripSuffix(".nir").replace(java.io.File.separatorChar, '.')).toSet
  finally s.close()

/** What is wrong with the owners' names: the directories in their bytes' order, each module is named the hexadecimal
 *  String.hashCode of its directory, and the next ones of a hash already taken `<hash>-1`, `<hash>-2`, ... */
def nameProblems(owners: Owners): List[String] =
  val dirs = owners.map(_._2).sortBy(_.getBytes("UTF-8").map(b => (b & 0xff).toChar).mkString)
  val expected = dirs.zipWithIndex.map { case (dir, i) =>
    val hash = Integer.toHexString(dir.hashCode)
    val before = dirs.take(i).count(_.hashCode == dir.hashCode)
    dir -> (if before == 0 then s"$hash.ll" else s"$hash-$before.ll")
  }.toMap
  owners.collect { case (name, dir, _) if expected(dir) != name => s"$name: the module of $dir is ${expected(dir)}" }

/** The judges of the digests and the modules (`readRecords`, `coverProblems`, `moduleProblems`, `ownerProblems`,
 *  `nameProblems`) held to planted records before they are trusted: a program of four modules, three of one hash, as
 *  the judges see it. Each judge has to pass the control, and each planted record has to fail the judge it names, that
 *  judge alone. How many records were planted and how many controls, and what is wrong with the judges, or nothing. */
def judgesSelfTest(): (Int, Int, List[String]) =
  val root = Files.createTempDirectory("teq-native-link-self.")
  Script.atExit(Sh.remove(root))
  def module(owner: String, method: String) = "define i32 @\"_SM" + owner.length + owner + "D" + method + "\"(ptr %_1) {\n_2.0:\n  ret i32 0\n}\n"
  val a = module("dirs.A$", "5valueiiEO") + module("dirs.A$$$Lambda$1", "13apply$mcVI$spiuEO")
  val b = module("dirs.B$", "5valueiEO")
  val c = module("dirs.C$", "5valueiEO")
  val main = "declare i32 @\"_SM7dirs.B$D5valueiEO\"(ptr)\n" + module("dirs.Main$", "4mainLAL16java.lang.String_uEO")
  val owners: Owners = List(
    ("1f0080.ll", "AaAa", Set("dirs.A", "dirs.A$", "dirs.A$$$Lambda$1")), ("1f0080-1.ll", "AaBB", Set("dirs.B", "dirs.B$")),
    ("1f0080-2.ll", "BBBB", Set("dirs.C", "dirs.C$")), ("3305b9.ll", "main", Set("dirs.Main", "dirs.Main$")))
  val good = List("1f0080.ll" -> a, "1f0080-1.ll" -> b, "1f0080-2.ll" -> c, "3305b9.ll" -> main)
  def manifestOf(files: List[(String, String)]): Manifest =
    files.map { case (name, text) => (sha256(text.getBytes("UTF-8")), lineCount(text.getBytes("UTF-8")).toString, name) }
  val expected = manifestOf(good)
  val shas = (0 until 8).map(i => sha256(Array(i.toByte)))
  // A fixture's digests (a column of main classes) and a file of one program's (none).
  val lines = List("detected", "true").flatMap { threads =>
    List(s"${shas(0)} 63768 dirs.Main $threads linked", s"${shas(1)} 110478 dirs.Main $threads lowered") ++
      expected.map { case (sha, count, name) => s"$sha $count dirs.Main $threads ll $name" }
  }
  val digests = ("# a comment" :: lines).mkString("\n")
  val oneProgram = List(s"${shas(2)} 66415 detected linked", s"${shas(3)} 115337 detected lowered", s"${shas(4)} 4980 detected ll 12b61281.ll",
    s"${shas(5)} 95316 true linked", s"${shas(6)} 160000 true lowered", s"${shas(7)} 4980 true ll 12b61281.ll").mkString("\n")
  def read(text: String) = readRecords(text, None)._2
  def without(text: String, line: String) = text.linesIterator.filterNot(_ == line).mkString("\n")
  def changed(text: String, line: String, to: String) = text.linesIterator.map(l => if l == line then to else l).mkString("\n")
  val linkedLine = lines.head
  val records = readRecords(digests, None)._1
  def covered(rs: List[Record], mains: Set[String] = Set("dirs.Main")) = coverProblems(rs, mains, owners)
  def dropped(kind: String, threads: String, module: String = "") = records.filterNot(r => r.kind == kind && r.threads == threads && r.module == module)
  var n = 0
  def written(files: List[(String, String)]): Path =
    n += 1
    val dir = Files.createDirectory(root.resolve(n.toString))
    files.foreach { case (name, text) => Files.writeString(dir.resolve(name), text) }
    dir
  def modules(files: List[(String, String)], manifest: Manifest = expected, ended: Boolean = true) = moduleProblems(ended, written(files), manifest)
  def owned(files: List[(String, String)]) = ownerProblems(written(files), owners)
  def put(name: String, text: String) = good.map { case (m, t) => m -> (if m == name then text else t) }
  val controls = List(
    "readRecords" -> (read(digests) ++ Option.when(records.length != lines.length)(s"${records.length} records of ${lines.length} lines")),
    "readRecords, one program" -> readRecords(oneProgram, Some("fix.Shapes"))._2,
    "coverProblems" -> covered(records), "coverProblems, one program" -> coverProblems(readRecords(oneProgram, Some("fix.Shapes"))._1, Set("fix.Shapes"), Nil),
    "moduleProblems" -> modules(good), "ownerProblems" -> owned(good), "nameProblems" -> nameProblems(owners))
  val planted = List(
    ("a record with one field too few", "readRecords", read(changed(digests, linkedLine, linkedLine.replace(" detected ", " ")))),
    ("a record with one field too many", "readRecords", read(changed(digests, linkedLine, linkedLine + " 1f0080.ll"))),
    ("a module's record with its module missing", "readRecords", read(changed(digests, lines(2), lines(2).substring(0, lines(2).lastIndexOf(' '))))),
    ("a record of one program with one field too many", "readRecords", readRecords(changed(oneProgram, oneProgram.linesIterator.next(), "x " + oneProgram.linesIterator.next()), Some("fix.Shapes"))._2),
    ("a duplicated record", "readRecords", read(digests + "\n" + linkedLine)),
    ("a record of an unknown kind", "readRecords", read(changed(digests, linkedLine, linkedLine.replace(" linked", " linkd")))),
    ("threads neither detected nor true", "readRecords", read(changed(digests, linkedLine, linkedLine.replace(" detected ", " maybe ")))),
    ("a SHA-256 that is not one", "readRecords", read(changed(digests, linkedLine, linkedLine.replace(shas(0), shas(0).take(63))))),
    ("a linked record deleted, its lowered one left", "coverProblems", covered(dropped("linked", "detected"))),
    ("a lowered record deleted, its linked one left", "coverProblems", covered(dropped("lowered", "true"))),
    ("an owner's module record deleted", "coverProblems", covered(dropped("ll", "true", "1f0080-2.ll"))),
    ("a cell with no module's record", "coverProblems", coverProblems(records.filterNot(r => r.kind == "ll" && r.threads == "detected"), Set("dirs.Main"), Nil)),
    ("a cell of one program with no module's record", "coverProblems", coverProblems(readRecords(oneProgram, Some("fix.Shapes"))._1.filterNot(_.module == "12b61281.ll"), Set("fix.Shapes"), Nil)),
    ("a cell with no record at all", "coverProblems", covered(records.filterNot(_.threads == "true"))),
    ("a record of no cell", "coverProblems", covered(records :+ Record(records.head.sha, records.head.lines, "dirs.Other", "detected", "linked", ""))),
    ("no cell to cover", "coverProblems", coverProblems(Nil, Set.empty, Nil)),
    ("a module not written", "moduleProblems", modules(good.filterNot(_._1 == "1f0080-2.ll"))),
    ("two modules under one name, the later written over the earlier", "moduleProblems", modules(put("1f0080.ll", b).filterNot(_._1 == "1f0080-1.ll"))),
    ("two expectations under one name", "moduleProblems", modules(good, expected :+ expected.head)),
    ("a module holding another's bytes", "moduleProblems", modules(put("1f0080-1.ll", c).map { case (m, t) => m -> (if m == "1f0080-2.ll" then b else t) })),
    ("an empty module", "moduleProblems", modules(put("1f0080.ll", ""))),
    ("a truncated module", "moduleProblems", modules(put("1f0080.ll", a.dropRight(1)))),
    ("a stray module", "moduleProblems", modules(good :+ ("1f0080-3.ll" -> c))),
    ("a run that timed out or failed", "moduleProblems", modules(good, ended = false)),
    ("an expectation missing", "moduleProblems", modules(good, expected.filterNot(_._3 == "3305b9.ll"))),
    ("a cell with no module expected and none written", "moduleProblems", modules(Nil, Nil)),
    ("a module not written", "ownerProblems", owned(good.filterNot(_._1 == "1f0080-2.ll"))),
    ("a module holding another directory's definitions under a digest of its own", "ownerProblems", owned(put("1f0080.ll", b))),
    ("a module named against the rule", "nameProblems", nameProblems(owners.map(o => if o._2 == "BBBB" then ("1f0080-1.ll", o._2, o._3) else o))))
  (planted.size, controls.size, controls.flatMap { case (judge, problems) => problems.map(p => s"the control fails $judge: $p") } ++
    planted.collect { case (what, judge, Nil) => s"planted, $what: $judge passed it" })

/** A run of a link: its exit code, whether its bound stopped it, its time and what it listed. */
case class Link(status: Int, timedOut: Boolean, millis: Long, listing: Array[Byte])

def linkOf(run: Sh.Result): Link = Link(run.status, run.timedOut, run.millis, Files.readAllBytes(run.outFile))

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
  val got = sha256(link.listing)
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
def linksSelfTest(): (Int, Int, List[String]) =
  val heads = (diamondOf(2).toList.sorted.map(name => s"trait @\"T${name.length}$name\"").filterNot(_.contains("Leaf")) :+
    "class @\"T13diamond.Leaf2\" : @\"T16java.lang.Object\", @\"T10diamond.A2\", @\"T10diamond.B2\"") ++
    List("def @\"M13diamond.Leaf2RE\" : (@\"T13diamond.Leaf2\") => unit {", "  ret %1 : unit", "}",
      "module @\"T14diamond.Main2$\" : @\"T16java.lang.Object\"")
  val sound = (heads.mkString("\n") + "\n").getBytes("UTF-8")
  val bound = 60.0
  def judged(link: Link, sha: String = ""): List[String] =
    linkProblems(link, bound, if sha.isEmpty then sha256(link.listing) else sha, diamondOf(2), isDiamonds)
  def listing(lines: List[String]): Link = Link(0, false, 100, (lines.mkString("\n") + "\n").getBytes("UTF-8"))
  val good = Link(0, false, 100, sound)
  val wanted = sha256(sound)
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

  // The judges of the modules, the checks of the links, refusals and the diamond and the judge of the classifier's
  // verdicts first, held to planted records (`judgesSelfTest`, `linksSelfTest`, `guardsSelfTest`).
  val selfTests = List(judgesSelfTest(), linksSelfTest(), guardsSelfTest())
  val selfWrong = selfTests.flatMap(_._3)
  if selfWrong.isEmpty then
    pass += 1
    println(s"native-link self-test: ${selfTests.map(_._1).sum} planted records failed, ${selfTests.map(_._2).sum} sound ones passed")
  else
    fail += 1
    selfWrong.foreach(p => println(s"FAIL native-link self-test: $p"))

  // The listings and modules of teq's own null-check rule, each against the reference's (`guardsOf`): how many were
  // equal, how many differed by added null checks and by how many checks, and how many were neither or not judged.
  var guardsEqual = 0
  var guardsAdded = 0
  var guardsChecks = 0
  var guardsOtherwise = 0
  // The classifier runs beside the lowerings, a few runs at once, each judged in its turn, the oldest first, the last
  // ones at the end (`judgeAll`).
  val judgesAtOnce = 4
  val judging = scala.collection.mutable.Queue.empty[(String, Sh.Result, Sh.Running, Int)]
  def nullGuards(label: String, reference: Path, written: Sh.Result, default: Path, listings: Int): Unit =
    if judging.length >= judgesAtOnce then judgeOldest()
    judging.enqueue((label, written, Sh("python3", "-I", "-B", "tests/support/null_guards.py", reference.toString, default.toString).mergeErr.timeout(600).check(false).spawn(), listings))
  def judgeAll(): Unit = while judging.nonEmpty do judgeOldest()
  def judgeOldest(): Unit =
    val (label, written, running, listings) = judging.dequeue()
    val judged = running.await()
    guardsOf(written.status, written.timedOut, judged.status, judged.timedOut, judged.out, listings) match
      case Right(guards) =>
        pass += 1
        guardsEqual += guards.equal
        guardsAdded += guards.added
        guardsChecks += guards.checks
      case Left(problems) =>
        fail += 1
        guardsOtherwise += listings
        println(s"FAIL $label, teq's null-check rule against Scala Native's: ${problems.mkString("; ")}")
        judged.lines.filter(l => l.startsWith("UNCLASSIFIED") || l.startsWith("UNRECONCILED") || l.startsWith("null_guards: cannot")).take(8).foreach(l => println(s"  $l"))
        written.err.linesIterator.take(5).foreach(println)

  // The class path: the program's own files, then Scala Native's jars.
  val caches = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).toList ++
    List("/Library/Caches/Coursier/v1", "/.cache/coursier/v1").map(System.getProperty("user.home") + _)
  val wanted = Files.readString(Paths.get("tests/nir/digests.txt")).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split(" ").last).toList
  val found = wanted.map(path => path -> caches.map(c => Paths.get(s"$c/https/repo1.maven.org/maven2/$path")).find(Files.isRegularFile(_)))
  for (path, jar) <- found if jar.isEmpty do println(s"skip ${path.substring(path.lastIndexOf('/') + 1)}: not in the coursier cache")

  if found.forall(_._2.nonEmpty) then
    val sep = if java.io.File.separatorChar == '\\' then ";" else ":"
    val classPath = ("tests/nir/fixtures" :: found.map(_._2.get.toString)).mkString(sep)
    // The classes whose definitions are kept whole: a case class, a class of which the link keeps
    // little, a trait with a method of its own, a lambda's class and a module of C functions.
    val shown = Set("fix.Circle", "fix.Counters", "fix.Named", "fix.Shapes$$$Lambda$1", "fix.libm$")
    def commandOn(main: String, path: String, verb: String, threads: String, more: String*) =
      val options = if threads == "detected" then Nil else List(s"--threads=$threads")
      Sh((List(teq, "nir", verb, main, path, "--triple=x86_64-unknown-linux-musl") ++ options ++ more)*).timeout(300).check(false)
    def runOn(main: String, path: String, verb: String, threads: String, more: String*) = commandOn(main, path, verb, threads, more*).run()
    def run(verb: String, threads: String, more: String*) = runOn("fix.Shapes", classPath, verb, threads, more*)
    // teq's own rule's lowering, started before the reference's and waited for after it, the two running at once.
    def ownLowering(main: String, path: String, threads: String, more: String*): Sh.Running = commandOn(main, path, "--lower", threads, more*).spawn()
    def tempDir(): Path =
      val dir = Files.createTempDirectory("teq-native-link.")
      Script.atExit(Sh.remove(dir))
      dir
    // The text a failure is read against: the named classes' definitions in the first output.
    def keptWhole(label: String, text: String, kept: String, classes: Set[String] = shown): Unit =
      val cut = Files.createTempFile("teq-native-link.", ".txt")
      Script.atExit(Files.deleteIfExists(cut))
      Files.writeString(cut, definitionsOf(text, classes))
      if java.util.Arrays.equals(Files.readAllBytes(cut), Files.readAllBytes(Paths.get(kept))) then pass += 1
      else
        fail += 1
        println(s"FAIL $label: the classes kept whole ($kept)")
        Sh("diff", cut.toString, kept).check(false).run().lines.take(20).foreach(println)

    // A digests file's records (`readRecords`), then the cells they cover (`coverProblems`): the main classes of the
    // program's .nir files, each with threads detected and true.
    def recordsOf(file: Path, main: Option[String], nir: Path, owners: Owners): List[Record] =
      val (records, wrong) = readRecords(Files.readString(file), main)
      for (what, problems) <- List("its records" -> wrong, "the cells they cover" -> coverProblems(records, mainsIn(nir), owners)) do
        if problems.isEmpty then pass += 1
        else
          fail += 1
          println(s"FAIL $file, $what: ${problems.length} problems")
          problems.take(8).foreach(p => println(s"  $p"))
      records

    val expected = recordsOf(Paths.get("tests/native-link/digests.txt"), Some("fix.Shapes"), Paths.get("tests/nir/fixtures"), Nil)
    for r <- expected if r.kind != "ll" do
      val label = s"${r.kind}, threads ${r.threads}"
      val own = Option.when(r.kind == "lowered")(ownLowering("fix.Shapes", classPath, r.threads))
      val listing = if r.kind == "linked" then run("--link", r.threads) else run("--lower", r.threads, ScalaNativeGuards)
      val bytes = Files.readAllBytes(listing.outFile)
      if listing.ok && sha256(bytes) == r.sha then pass += 1
      else
        fail += 1
        println(s"FAIL $label: exit code ${listing.status}, ${lineCount(bytes)} lines of sha256 ${sha256(bytes)}, Scala Native's ${r.lines} of ${r.sha}")
        listing.err.linesIterator.take(5).foreach(println)
      if r.threads == "detected" && listing.ok then keptWhole(label, listing.outAll(), s"tests/native-link/${r.kind}.txt")
      for running <- own do
        val written = running.await()
        nullGuards(label, listing.outFile, written, written.outFile, 1)

    // The modules a run wrote, against the manifest (`moduleProblems`) and whatever more is asked of them; the
    // modules' text, in their names' order, for what is kept whole.
    def modulesOf(label: String, written: Sh.Result, dir: Path, modules: Manifest, more: List[String] = Nil): Option[String] =
      val problems = moduleProblems(written.ok && !written.timedOut, dir, modules) ++ more
      if problems.isEmpty then pass += 1
      else
        fail += 1
        println(s"FAIL $label: exit code ${written.status}${if written.timedOut then ", timed out" else ""}, ${problems.length} problems")
        problems.take(8).foreach(p => println(s"  $p"))
        written.err.linesIterator.take(5).foreach(println)
      Option.when(written.ok)(filesIn(dir).sortBy(_.stripSuffix(".ll")).map(name => Files.readString(dir.resolve(name))).mkString)
    // The modules of teq's own null-check rule are judged each against the reference's module of its name (`nullGuards`).
    for (threads, modules) <- expected.filter(_.kind == "ll").groupBy(_.threads).toList.sortBy(_._1) do
      val label = s"ll, threads $threads"
      val (dir, ownDir) = (tempDir(), tempDir())
      val own = ownLowering("fix.Shapes", classPath, threads, s"--ll=$ownDir")
      val written = run("--lower", threads, s"--ll=$dir", ScalaNativeGuards)
      val text = modulesOf(label, written, dir, modules.map(r => (r.sha, r.lines, r.module)))
      if threads == "detected" then text.foreach(keptWhole(label, _, "tests/native-link/ll.txt"))
      nullGuards(label, dir, own.await(), ownDir, modules.length)

    // Programs of class paths of their own (tests/native-link/<fixture>/nir before the jars), each one's link, lowering
    // and modules byte for byte what its digests.txt records of Scala Native's (its records and their cells judged as
    // tests/native-link/digests.txt's are); of the first lowering, the
    // definitions of the program's own classes kept whole; its modules written `runs` times, each run held to the
    // manifest, and the modules of its owners (modules.txt, where there is one) each holding its own directory's
    // definitions.
    def programs(fixture: String, runs: Int): Unit =
      val root = Paths.get("tests/native-link").resolve(fixture)
      val path = (root.resolve("nir").toString :: found.map(_._2.get.toString)).mkString(sep)
      val owners: Owners =
        val table = root.resolve("modules.txt")
        if !Files.isRegularFile(table) then Nil
        else Files.readString(table).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split(" ").toList).toList.map {
          case name :: dir :: classes => (name, dir, classes.toSet)
          case other => Log.die(s"$table: a line of no module and directory: ${other.mkString(" ")}")
        }
      if owners.nonEmpty then
        val wrong = nameProblems(owners)
        if wrong.isEmpty then pass += 1
        else
          fail += 1
          wrong.foreach(p => println(s"FAIL $root/modules.txt: $p"))
      val records = recordsOf(root.resolve("digests.txt"), None, root.resolve("nir"), owners)
      for r <- records if r.kind != "ll" do
        val label = s"${r.main} ${r.kind}, threads ${r.threads}"
        val own = Option.when(r.kind == "lowered")(ownLowering(r.main, path, r.threads))
        val listing = if r.kind == "linked" then runOn(r.main, path, "--link", r.threads) else runOn(r.main, path, "--lower", r.threads, ScalaNativeGuards)
        val bytes = Files.readAllBytes(listing.outFile)
        if listing.ok && sha256(bytes) == r.sha then pass += 1
        else
          fail += 1
          println(s"FAIL $label: exit code ${listing.status}, ${lineCount(bytes)} lines of sha256 ${sha256(bytes)}, Scala Native's ${r.lines} of ${r.sha}")
          listing.err.linesIterator.take(5).foreach(println)
        if r.threads == "detected" && r.kind == "lowered" && listing.ok then
          val pkg = r.main.substring(0, r.main.lastIndexOf('.'))
          val own = filesIn(root.resolve("nir").resolve(pkg.replace('.', '/'))).map(f => pkg + "." + f.stripSuffix(".nir")).toSet
          keptWhole(label, listing.outAll(), root.resolve(s"${r.main}.txt").toString, own)
        for running <- own do
          val written = running.await()
          nullGuards(label, listing.outFile, written, written.outFile, 1)
      for ((main, threads), modules) <- records.filter(_.kind == "ll").groupBy(r => (r.main, r.threads)).toList.sortBy(_._1) do
        val manifest = modules.map(r => (r.sha, r.lines, r.module))
        for round <- 1 to runs do
          val (dir, ownDir) = (tempDir(), tempDir())
          val own = ownLowering(main, path, threads, s"--ll=$ownDir")
          val written = runOn(main, path, "--lower", threads, s"--ll=$dir", ScalaNativeGuards)
          val label = if runs == 1 then s"$main ll, threads $threads" else s"$main ll, threads $threads, run $round of $runs"
          modulesOf(label, written, dir, manifest, ownerProblems(dir, owners))
          nullGuards(label, dir, own.await(), ownDir, modules.length)
    // Programs whose names share a String.hashCode (tests/native-link/collisions/README): a class's virtual methods
    // and a trait's methods in the order of the set of its called signatures, two of one hash in the order the link
    // met them.
    programs("collisions", 1)
    // A program whose sources lie in directories of one String.hashCode (tests/native-link/directories/README): a
    // module for each directory, under Scala Native's name where the hash is the directory's alone and numbered within
    // a hash, each with its own directory's definitions, the same in every run.
    programs("directories", 3)

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
    judgeAll()
    println(s"native-link: $guardsEqual equal, $guardsAdded by added null checks ($guardsChecks checks), $guardsOtherwise otherwise")

  println(s"native-link: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}
