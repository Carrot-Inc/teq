//> using file ../tools/script
// Tests of the class file reader: `teq classfile` over the fixtures under tests/classfile/fixtures
// (compiled from tests/classfile/src with javac; see tests/classfile/README) has to print what
// tests/classfile/expected holds (`--update` rewrites it); truncated and corrupted class files
// have to be refused with a message, never a crash; every class of java.base in the JDK's ct.sym
// and every class file of scala-library has to parse; and the printer has to agree with
// `javap -s -p` on java.lang.String and java.util.Map. The JDK, the jar and javap parts skip, and
// count as passed, when they are not found. Run from the repository's root:
// `./teq interp tests/classfile.scala [-- --update] [--teq <binary>]`, the compiler `--teq`, else
// $TEQ, else target/release/teq.
import java.nio.file.{Files, Path, Paths}

@main def classfile(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/classfile.scala [--update] [--teq <binary>]")
  val update = args.flag("update")
  val teq = args.teq("./target/release/teq")
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/classfile")) then Log.die("tests/classfile.scala runs from the repository's root")
  var pass = 0
  var fail = 0
  def chomp(s: String): String = s.replaceAll("\n+$", "")
  def listed(dir: Path, suffix: String): Vector[Path] =
    val s = Files.list(dir)
    try s.toArray.toVector.map(_.asInstanceOf[Path]).filter(_.getFileName.toString.endsWith(suffix)).sortBy(_.getFileName.toString)
    finally s.close()
  val scratch = Files.createTempDirectory("teq-classfile.")
  Script.atExit(Sh.remove(scratch))
  // `diff <a> <b> | head -n`, the lines the old script showed.
  def diffHead(a: Path, b: Path, n: Int): String =
    Sh("diff", a.toString, b.toString).check(false).run().lines.take(n).mkString("\n")

  // The fixtures against their expectations.
  val expectedDir = Paths.get("tests/classfile/expected")
  Files.createDirectories(expectedDir)
  for f <- listed(Paths.get("tests/classfile/fixtures/fix"), ".class") do
    val name = f.getFileName.toString.stripSuffix(".class")
    val expected = expectedDir.resolve(s"$name.txt")
    val actual = chomp(Sh(teq, "classfile", f.toString).mergeErr.timeout(30).check(false).run().out)
    if update then Files.writeString(expected, actual + "\n")
    else if Files.exists(expected) && actual == chomp(Files.readString(expected)) then pass += 1
    else
      fail += 1
      println(s"FAIL $name")
      val shown = scratch.resolve("actual")
      Files.writeString(shown, actual + "\n")
      val d = diffHead(shown, expected, 20)
      if d.nonEmpty then println(d)
  if update then Script.exit(0)

  // Malformed input: a message and exit code 1, never a panic (101) or a signal. The truncations
  // and corruptions of tests/classfile.sh's, made by the same generator (Python's random, seed 7).
  val bad = Files.createTempDirectory("teq-classfile.")
  Script.atExit(Sh.remove(bad))
  val data = Files.readAllBytes(Paths.get("tests/classfile/fixtures/fix/Generics.class"))
  for n <- List(0, 4, 9, 10, 40, 200, 700, data.length / 2, data.length - 1) do Files.write(bad.resolve(s"trunc$n.class"), data.take(n))
  val rnd = new PyRandom(7)
  for i <- 0 until 60 do
    val d = data.clone()
    for _ <- 1 to rnd.randint(1, 4) do
      val at = if i % 2 == 0 then rnd.randrange(300) else d.length - 1 - rnd.randrange(300)
      d(at) = rnd.randrange(256).toByte
    Files.write(bad.resolve(s"corrupt$i.class"), d)
  Files.write(bad.resolve("lengths.class"), data.take(10) ++ Array(0xff.toByte, 0xff.toByte) ++ data.drop(12))
  var crashed = false
  var refused = 0
  val malformed = listed(bad, ".class")
  for f <- malformed do
    val r = Sh(teq, "classfile", f.toString).timeout(30).check(false).run()
    if r.status == 1 then refused += 1
    if r.status > 1 || r.err.contains("panicked") then
      crashed = true
      println(s"FAIL ${f.getFileName}: exit code ${r.status}")
      r.err.linesIterator.take(3).foreach(println)
  if !crashed && refused >= 10 then pass += 1
  else
    fail += 1
    println(s"FAIL malformed input: $refused of ${malformed.length} refused")
  if Sh(teq, "classfile", bad.resolve("trunc200.class").toString).mergeErr.timeout(30).check(false).run().out.contains("truncated class file") then pass += 1
  else
    fail += 1
    println("FAIL truncation message")

  // Every class of java.base in the JDK's ct.sym, and every class file of scala-library.
  val ct = Sh(teq, "classfile", "--verify", "--jdk", "java.base/").mergeErr.timeout(120).check(false).run()
  val ctOut = chomp(ct.out)
  if ct.ok then
    pass += 1
    println(ctOut.linesIterator.toVector.lastOption.getOrElse(""))
  else if ctOut.contains("no JDK found") then println(s"skip ct.sym: $ctOut")
  else
    fail += 1
    println(s"FAIL ct.sym: $ctOut")
  val m2 = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).getOrElse(System.getProperty("user.home") + "/Library/Caches/Coursier/v1") + "/https/repo1.maven.org/maven2"
  val jar = Paths.get(s"$m2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")
  if Files.isRegularFile(jar) then
    val verified = Sh(teq, "classfile", "--verify", jar.toString).mergeErr.timeout(120).check(false).run()
    if verified.ok then
      pass += 1
      println(chomp(verified.out).linesIterator.toVector.lastOption.getOrElse(""))
    else
      fail += 1
      println(s"FAIL scala-library: ${chomp(verified.out)}")
    // The Java class files selected by their missing TASTy siblings are the ones without a Scala attribute.
    val stats = chomp(Sh(teq, "classfile", "--stats", jar.toString).mergeErr.timeout(120).check(false).run().out)
    if stats.contains("; 0 carry a Scala attribute anyway; 58 Java") && verified.out.contains("3600 written by a Scala compiler") then pass += 1
    else
      fail += 1
      println("FAIL scala-library Java selection")
      stats.linesIterator.take(3).foreach(println)
  else println("skip scala-library: not in the coursier cache")

  // The printer against javap on the same class files.
  val javap = Sh.which("javap").isDefined
  if javap && Sh(teq, "classfile", "--list", "--jdk", "java.base/java/lang/String.sig").timeout(30).check(false).run().ok then
    val dir = Files.createTempDirectory("teq-javap.")
    Script.atExit(Sh.remove(dir))
    val ctsym = Sh(teq, "classfile", "--jdk-path").timeout(30).check(false).run().out.trim
    val zip = new java.util.zip.ZipFile(ctsym)
    try
      for entry <- List("java.base/java/lang/String.sig", "java.base/java/util/Map.sig") do
        val full = Sh(teq, "classfile", "--list", "--jdk", entry).timeout(30).run().lines.headOption.map(_.trim.split("\\s+")(2)).getOrElse("")
        val in = zip.getInputStream(zip.getEntry(full))
        try Files.write(dir.resolve(Paths.get(entry).getFileName.toString.stripSuffix(".sig") + ".class"), in.readAllBytes())
        finally in.close()
    finally zip.close()
    for f <- List(dir.resolve("String.class"), dir.resolve("Map.class"), Paths.get("tests/classfile/fixtures/fix/Generics.class"), Paths.get("tests/classfile/fixtures/fix/Named.class")) do
      val r = Sh("python3", "tests/classfile/javap_compare.py", teq, f.toString).mergeErr.timeout(120).check(false).run()
      if r.ok then pass += 1
      else
        fail += 1
        println(s"FAIL javap ${f.getFileName}: ${chomp(r.out)}")
  else println("skip javap comparison: no javap or no ct.sym")

  // The erasure of type parameters, value classes and arrays as scalac 3.8.4's: the members of teq's
  // class files of tests/classfile/erasure/erasure.scala as javap describes them (javap_members.py)
  // have to be expected.txt, the same list of scalac's class files, made by
  //   scala-cli compile -S 3.8.4 --jvm system --server=false tests/classfile/erasure/erasure.scala -d <dir>
  //   python3 tests/classfile/javap_members.py <dir> tests/classfile/erasure/excluded.txt > tests/classfile/erasure/expected.txt
  // The members excluded.txt names are left out on both sides: their presence differs whatever
  // their erasure.
  if javap && Files.isRegularFile(jar) then
    val dir = Files.createTempDirectory("teq-erasure.")
    Script.atExit(Sh.remove(dir))
    val log = dir.resolve("log")
    val built = Sh(teq, "compiler", "build", "tests/classfile/erasure/erasure.scala", "--target", "jvm", "--classpath", jar.toString, "--products", dir.resolve("out").toString)
      .stdout(log).mergeErr.timeout(60).check(false).run().ok
    val members = if built then Some(Sh("python3", "tests/classfile/javap_members.py", dir.resolve("out").toString, "tests/classfile/erasure/excluded.txt").timeout(120).check(false).run()) else None
    val actual = dir.resolve("actual")
    members.filter(_.ok).foreach(m => Files.writeString(actual, m.lines.filter(_.startsWith("erasure/")).map(_ + "\n").mkString))
    val diffed = if members.exists(_.ok) then Some(Sh("diff", "tests/classfile/erasure/expected.txt", actual.toString).check(false).run()) else None
    if diffed.exists(_.ok) && members.exists(_.lines.exists(_.startsWith("erasure/"))) then pass += 1
    else
      fail += 1
      println("FAIL erasure: the members are not scalac's")
      val shown = List(log) ++ diffed.map(_.outFile)
      shown.foreach { f =>
        println(s"==> $f <==")
        if Files.exists(f) then Files.readAllLines(f).toArray.take(20).foreach(println)
      }
  else println("skip erasure: no javap, or scala-library not in the coursier cache")
  println(s"classfile: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}

// Python's `random.Random(seed)` for a small int seed, as tests/classfile.sh drew its corruptions:
// MT19937 seeded by `init_by_array([seed])`, `randrange(n)` as `_randbelow` takes it (the
// getrandbits of n's bit length, drawn again while too big), `randint(a, b)` over `randrange`.
final class PyRandom(seed: Int):
  private val n = 624
  private val mt = new Array[Long](n)
  private var index = n
  private def initGenrand(s: Long): Unit =
    mt(0) = s & 0xffffffffL
    var i = 1
    while i < n do
      mt(i) = (1812433253L * (mt(i - 1) ^ (mt(i - 1) >>> 30)) + i) & 0xffffffffL
      i += 1
  locally {
    initGenrand(19650218L)
    val key = Array(seed.toLong & 0xffffffffL)
    var i = 1
    var j = 0
    var k = Math.max(n, key.length)
    while k > 0 do
      mt(i) = ((mt(i) ^ ((mt(i - 1) ^ (mt(i - 1) >>> 30)) * 1664525L)) + key(j) + j) & 0xffffffffL
      i += 1
      j += 1
      if i >= n then
        mt(0) = mt(n - 1)
        i = 1
      if j >= key.length then j = 0
      k -= 1
    k = n - 1
    while k > 0 do
      mt(i) = ((mt(i) ^ ((mt(i - 1) ^ (mt(i - 1) >>> 30)) * 1566083941L)) - i) & 0xffffffffL
      i += 1
      if i >= n then
        mt(0) = mt(n - 1)
        i = 1
      k -= 1
    mt(0) = 0x80000000L
  }
  private def next32(): Long =
    if index >= n then
      var kk = 0
      while kk < n do
        val y = (mt(kk) & 0x80000000L) | (mt((kk + 1) % n) & 0x7fffffffL)
        mt(kk) = mt((kk + 397) % n) ^ (y >>> 1) ^ (if (y & 1L) != 0 then 0x9908b0dfL else 0L)
        kk += 1
      index = 0
    var y = mt(index)
    index += 1
    y ^= y >>> 11
    y ^= (y << 7) & 0x9d2c5680L
    y ^= (y << 15) & 0xefc60000L
    y ^= y >>> 18
    y & 0xffffffffL
  def randrange(limit: Int): Int =
    val bits = 32 - Integer.numberOfLeadingZeros(limit)
    var r = next32() >>> (32 - bits)
    while r >= limit do r = next32() >>> (32 - bits)
    r.toInt
  def randint(a: Int, b: Int): Int = a + randrange(b - a + 1)
