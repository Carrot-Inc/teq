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

  // The type tests of patterns as scalac's erasure writes them: the `instanceof`, `ifnull`, `ifnonnull`
  // and `checkcast` instructions of teq's class files of tests/classfile/typetests/typetests.scala, per
  // method (javap_tests.py), have to be expected.txt, the same of scalac's class files, made by
  //   scala-cli compile -S 3.8.4 --jvm system --server=false tests/classfile/typetests/typetests.scala -d <dir>
  //   python3 tests/classfile/javap_tests.py <dir> typetests/ > tests/classfile/typetests/expected.txt
  // A run's output cannot tell a class test from a null test where the scrutinee's type passes both.
  if javap && Files.isRegularFile(jar) then
    val dir = Files.createTempDirectory("teq-typetests.")
    Script.atExit(Sh.remove(dir))
    val log = dir.resolve("log")
    val built = Sh(teq, "compiler", "build", "tests/classfile/typetests/typetests.scala", "--target", "jvm", "--classpath", jar.toString, "--products", dir.resolve("out").toString)
      .stdout(log).mergeErr.timeout(60).check(false).run().ok
    val tests = if built then Some(Sh("python3", "tests/classfile/javap_tests.py", dir.resolve("out").toString, "typetests/").timeout(120).check(false).run()) else None
    val actual = dir.resolve("actual")
    tests.filter(_.ok).foreach(t => Files.writeString(actual, t.lines.map(_ + "\n").mkString))
    val diffed = if tests.exists(_.ok) then Some(Sh("diff", "tests/classfile/typetests/expected.txt", actual.toString).check(false).run()) else None
    if diffed.exists(_.ok) && tests.exists(_.lines.nonEmpty) then pass += 1
    else
      fail += 1
      println("FAIL type tests: the instructions are not scalac's")
      val shown = List(log) ++ diffed.map(_.outFile)
      shown.foreach { f =>
        println(s"==> $f <==")
        if Files.exists(f) then Files.readAllLines(f).toArray.take(20).foreach(println)
      }
  else println("skip type tests: no javap, or scala-library not in the coursier cache")

  // The owner each member reference names as scalac 3.8.4 writes it, the qualifier's class and not the
  // member's declaring one (dotty's `BCodeBodyBuilder.genApply` and `genLoad`, after
  // `Erasure.typedSelect`'s adaptation): the references of teq's class files of each program below,
  // per method, to the members owners/names.txt names (javap_owners.py), have to be
  // owners/<program>.expected, the same list of scalac's class files, made by
  //   scala-cli compile -S 3.8.4 --jvm system --server=false [--jar <jar>] <source> -d <dir>
  //   python3 tests/classfile/javap_owners.py <dir> <prefix> $(paste -sd, tests/classfile/owners/names.txt) \
  //     tests/classfile/owners/excluded.txt > tests/classfile/owners/<program>.expected
  // with the jar of tests/support/jars.sh that the program's `// jars:` line names. The public subclass of
  // the Java fixture declares none of the members it inherits: javac writes a public class an
  // accessibility bridge of a non-final public method of a package-private one, which would forward to the
  // member and leave a reference naming the declaring class unseen. The interpreter refuses the Java
  // fixture's program, having no body of a Java class to run (tests/cases/hidden_owner_scala.scala is
  // its Scala program). The forwarders of a trait's `clone` (owners/clone_forwarder) are run by Java
  // clients: a narrower result's, a diamond's over a whole and a split build, a final one's, one of a
  // scalac-built trait's that declares exceptions, a Java default's annotated one, a generic one's raw, a
  // polymorphic one's without a bridge of `Object`'s, one of two traits' clones, none of a private one, and
  // an `equals` with its parameter's annotation, over scalac's products as over teq's. A client built against two versions of a Java library
  // (owners/inline_abi) runs on the second. The jars need javac and scala-cli; without them the part skips.
  if javap && Files.isRegularFile(jar) then
    def jarOf(name: String): Path =
      Paths.get(Sh("bash", "-c", s". tests/support/jars.sh && jar_of $name").timeout(600).check(false).run().out.trim)
    val hidden = jarOf("hiddenowner-lib")
    val bridge = jarOf("bridgetrait-lib")
    val inlineV1 = jarOf("inlineabi-v1")
    val inlineV2 = jarOf("inlineabi-v2")
    val throwsLib = jarOf("throwsclone-lib")
    val javaDefault = jarOf("javadefault-lib")
    val javaDefaultScalac = jarOf("javadefault-scalac")
    val genericLib = jarOf("genericclone-lib")
    val genericScalac = jarOf("genericclone-scalac")
    val polyLib = jarOf("polyclone-lib")
    val polyScalac = jarOf("polyclone-scalac")
    val mixedLib = jarOf("mixedclone-lib")
    val privateLib = jarOf("privateclone-lib")
    val privateScalac = jarOf("privateclone-scalac")
    val paramLib = jarOf("parammeta-lib")
    val paramScalac = jarOf("parammeta-scalac")
    if List(hidden, bridge, inlineV1, inlineV2, throwsLib, javaDefault, javaDefaultScalac, genericLib, genericScalac, polyLib, polyScalac, mixedLib, privateLib, privateScalac, paramLib, paramScalac).forall(Files.isRegularFile(_)) then
      val names = Files.readAllLines(Paths.get("tests/classfile/owners/names.txt")).toArray.map(_.toString).filter(_.nonEmpty).mkString(",")
      val programs = List(
        ("hidden_owner", "tests/classpath/jvm/hidden_owner.scala", Some(hidden), "owners/"),
        ("java_scala_parent", "tests/classpath/jvm/java_scala_parent.scala", Some(bridge), "javaparent/"),
        ("java_scala_parent_trait_first", "tests/classpath/jvm/java_scala_parent_trait_first.scala", Some(bridge), "javaparentfirst/"),
        ("hidden_owner_scala", "tests/cases/hidden_owner_scala.scala", None, "hidden"),
        ("jdk_receiver_owner", "tests/classpath/jvm/jdk_receiver_owner.scala", None, "jdkowner/"),
        ("clone_forwarder", "tests/classfile/owners/clone_forwarder/CloneForwarder.scala", None, "cloneforwarder/")
      )
      for (program, source, lib, prefix) <- programs do
        val dir = Files.createTempDirectory("teq-owners.")
        Script.atExit(Sh.remove(dir))
        val log = dir.resolve("log")
        val cp = (jar :: lib.toList).mkString(":")
        val built = Sh(teq, "compiler", "build", source, "--target", "jvm", "--classpath", cp, "--products", dir.resolve("out").toString)
          .stdout(log).mergeErr.timeout(120).check(false).run().ok
        val listed = if built then Some(Sh("python3", "tests/classfile/javap_owners.py", dir.resolve("out").toString, prefix, names, "tests/classfile/owners/excluded.txt").timeout(120).check(false).run()) else None
        val actual = dir.resolve("actual")
        listed.filter(_.ok).foreach(l => Files.writeString(actual, l.lines.map(_ + "\n").mkString))
        val expected = Paths.get(s"tests/classfile/owners/$program.expected")
        val diffed = if listed.exists(_.ok) then Some(Sh("diff", expected.toString, actual.toString).check(false).run()) else None
        if diffed.exists(_.ok) && listed.exists(_.lines.nonEmpty) then pass += 1
        else
          fail += 1
          println(s"FAIL owners of $program: the references are not scalac's")
          (List(log) ++ diffed.map(_.outFile)).foreach { f =>
            println(s"==> $f <==")
            if Files.exists(f) then Files.readAllLines(f).toArray.take(20).foreach(println)
          }
      // A trait's `clone` of a narrower result, mixed in: the class's forwarder has the overriding
      // method's signature and a bridge of `Object`'s to it (`Mixin.mixinForwarders`, `Bridges`), which
      // a Java client compiled against teq's products calls, by javac's choice and by reflection.
      val javac = Option(System.getenv("JAVAC")).filter(_.nonEmpty)
        .orElse(Option(System.getenv("JAVA_HOME")).filter(_.nonEmpty).map(_ + "/bin/javac")).getOrElse("javac")
      val forwarder = Files.createTempDirectory("teq-forwarder.")
      Script.atExit(Sh.remove(forwarder))
      val cloneSrc = "tests/classfile/owners/clone_forwarder"
      val cloneProducts = forwarder.resolve("out")
      val cloneBuilt = Sh(teq, "compiler", "build", s"$cloneSrc/CloneForwarder.scala", "--target", "jvm", "--classpath", jar.toString, "--products", cloneProducts.toString)
        .stdout(forwarder.resolve("log")).mergeErr.timeout(120).check(false).run().ok
      val clients = forwarder.resolve("client")
      val clientsBuilt = cloneBuilt && Sh(javac, "--release", "17", "-nowarn", "-cp", cloneProducts.toString, "-d", clients.toString, s"$cloneSrc/JavaClient.java", s"$cloneSrc/ReflectClient.java")
        .mergeErr.timeout(120).check(false).run().ok
      val clientRuns = if clientsBuilt then List("JavaClient", "ReflectClient").map(m => Sh("java", "-cp", s"$clients:$cloneProducts:$jar", m).mergeErr.timeout(60).check(false).run().out.trim) else Nil
      val described = Sh("javap", "-v", "-p", "-cp", cloneProducts.toString, "cloneforwarder.E").timeout(60).check(false).run().lines
      val bridged = described.indexWhere(_.trim == "public java.lang.Object clone();") match
        case -1 => false
        case at => described.slice(at, at + 3).exists(_.contains("ACC_PUBLIC, ACC_BRIDGE, ACC_SYNTHETIC"))
      if clientRuns == List("clone", "clone") && bridged && described.exists(_.trim == "public java.lang.String clone();") then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the forwarder of a trait's narrower clone and its bridge: clients ${clientRuns.mkString(", ")}, bridge $bridged")
        val log = forwarder.resolve("log")
        if Files.exists(log) then Files.readAllLines(log).toArray.take(10).foreach(println)
      // The diamond of `clone` overrides (DiamondLibrary.scala): built whole, and split with the client over
      // the library's products, the class mixing both traits in is the same bytes, and a Java client
      // compiled against the whole build calls it by its own signature and by `Root`'s over either build.
      val diamond = Files.createTempDirectory("teq-diamond.")
      Script.atExit(Sh.remove(diamond))
      def diamondBuild(out: String, cp: String, sources: String*): Boolean =
        Sh((List(teq, "compiler", "build") ++ sources.map(f => s"$cloneSrc/$f") ++ List("--target", "jvm", "--classpath", cp, "--products", diamond.resolve(out).toString))*)
          .stdout(diamond.resolve(s"$out.log")).mergeErr.timeout(120).check(false).run().ok
      val diamondBuilt = diamondBuild("whole", jar.toString, "DiamondLibrary.scala", "DiamondClient.scala")
        && diamondBuild("a", jar.toString, "DiamondLibrary.scala")
        && diamondBuild("b", s"$jar:${diamond.resolve("a")}", "DiamondClient.scala")
      val diamondClass = "clonediamond/Diamond.class"
      val sameBytes = diamondBuilt && java.util.Arrays.equals(Files.readAllBytes(diamond.resolve("whole").resolve(diamondClass)), Files.readAllBytes(diamond.resolve("b").resolve(diamondClass)))
      val diamondClient = diamond.resolve("client")
      val diamondClientBuilt = diamondBuilt && Sh(javac, "--release", "17", "-nowarn", "-cp", diamond.resolve("whole").toString, "-d", diamondClient.toString, s"$cloneSrc/DiamondJava.java")
        .mergeErr.timeout(120).check(false).run().ok
      val diamondRuns = if diamondClientBuilt then
        List(s"${diamond.resolve("whole")}", s"${diamond.resolve("b")}:${diamond.resolve("a")}").map(cp => Sh("java", "-cp", s"$diamondClient:$cp:$jar", "DiamondJava").mergeErr.timeout(60).check(false).run().out.trim)
      else Nil
      if sameBytes && diamondRuns == List("right\nright", "right\nright") then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the diamond of clone overrides: built $diamondBuilt, whole and split alike $sameBytes, runs ${diamondRuns.mkString(" | ")}")
        List("whole", "a", "b").map(o => diamond.resolve(s"$o.log")).filter(Files.exists(_)).foreach(f => Files.readAllLines(f).toArray.take(5).foreach(println))
      // A trait's final `clone` (FinalClone.scala): the forwarder is final, so that javac refuses a Java
      // subclass's override as it does against scalac's classes.
      val finalProducts = forwarder.resolve("final")
      val finalBuilt = Sh(teq, "compiler", "build", s"$cloneSrc/FinalClone.scala", "--target", "jvm", "--classpath", jar.toString, "--products", finalProducts.toString)
        .stdout(forwarder.resolve("final.log")).mergeErr.timeout(120).check(false).run().ok
      val overriding = if finalBuilt then Some(Sh(javac, "--release", "17", "-nowarn", "-cp", finalProducts.toString, "-d", forwarder.resolve("final-client").toString, s"$cloneSrc/FinalClient.java")
        .mergeErr.timeout(120).check(false).run()) else None
      if overriding.exists(r => !r.ok && r.out.contains("overridden method is final")) then pass += 1
      else
        fail += 1
        println(s"FAIL owners: a Java override of a trait's final clone's forwarder is not refused: ${overriding.map(_.out.linesIterator.take(3).mkString(" ")).getOrElse("the program does not build")}")
      // A scalac-built trait's clone that declares its exceptions, deprecated and annotated
      // (tests/support/hidden_owner/throwsclone): the forwarder carries each over as the trait's class file
      // holds them (`MixinOps.mkForwarderSym`, `BCodeSkelBuilder.initJMethod`), so that javac refuses a
      // client that leaves what it throws unhandled, as it does against scalac's class.
      val throwsProducts = forwarder.resolve("throws")
      val throwsBuilt = Sh(teq, "compiler", "build", s"$cloneSrc/ThrowsUse.scala", "--target", "jvm", "--classpath", s"$jar:$throwsLib", "--products", throwsProducts.toString)
        .stdout(forwarder.resolve("throws.log")).mergeErr.timeout(120).check(false).run().ok
      val unhandled = if throwsBuilt then Some(Sh(javac, "--release", "17", "-nowarn", "-cp", s"$throwsProducts:$throwsLib", "-d", forwarder.resolve("throws-client").toString, s"$cloneSrc/ThrowsClient.java")
        .mergeErr.timeout(120).check(false).run()) else None
      val throwsDescribed = if throwsBuilt then Sh("javap", "-v", "-p", "-cp", s"$throwsProducts:$throwsLib", "throwsuse.E").timeout(60).check(false).run().lines.map(_.trim) else Nil
      val carried = throwsDescribed.contains("public java.lang.Object clone() throws java.lang.CloneNotSupportedException, java.io.IOException;")
        && throwsDescribed.contains("Deprecated: true") && throwsDescribed.contains("java.lang.Deprecated")
      if carried && unhandled.exists(r => !r.ok && r.out.contains("unreported exception CloneNotSupportedException")) then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the forwarder of a scalac-built trait's clone drops its metadata: carried $carried, javac ${unhandled.map(_.out.linesIterator.take(3).mkString(" ")).getOrElse("not run, the program does not build")}")
      // A Java interface's annotated default clone (tests/support/hidden_owner/javadefault): the class mixing it
      // in, built by teq, has the forwarder scalac's build has, with the annotation (`MixinOps.mkForwarderSym`
      // copies it), which a Java client reads by reflection alike over teq's products and scalac's.
      val mixed = Files.createTempDirectory("teq-mixedin.")
      Script.atExit(Sh.remove(mixed))
      def mixedBuild(out: String, cp: String, sources: String*): Boolean =
        Sh((List(teq, "compiler", "build") ++ sources ++ List("--target", "jvm", "--classpath", cp, "--products", mixed.resolve(out).toString))*)
          .stdout(mixed.resolve(s"$out.log")).mergeErr.timeout(120).check(false).run().ok
      // The fixture's Client.java, compiled by javac against teq's products, run over them and over scalac's.
      def mixedClient(dir: String, lib: Path, products: Path, scalac: Path): List[String] =
        val client = mixed.resolve(s"$dir-client")
        val built = Sh(javac, "--release", "17", "-nowarn", "-cp", s"$products:$lib:$jar", "-d", client.toString, s"tests/support/hidden_owner/$dir/Client.java").mergeErr.timeout(120).check(false).run()
        if built.ok then List(products, scalac).map(p => Sh("java", "-cp", s"$client:$p:$lib:$jar", "Client").mergeErr.timeout(60).check(false).run().out.trim)
        else List(s"javac: ${built.out.linesIterator.take(2).mkString(" ")}")
      val annotated =
        if mixedBuild("javadefault", s"$jar:$javaDefault", "tests/support/hidden_owner/javadefault/Use.scala") then mixedClient("javadefault", javaDefault, mixed.resolve("javadefault"), javaDefaultScalac)
        else Nil
      if annotated == List.fill(2)("@jann.Tag(\"clone\")") then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the forwarder of a Java default's clone lacks its annotation: ${annotated.mkString(" | ")}")
      // A scalac-built trait's generic clone (tests/support/hidden_owner/genericclone), mixed in at String, at the
      // class's own type parameter, and of an array of it: the forwarders are raw, as every method teq writes is
      // (no generic signature, the report's item 13), which a Java client compiled by javac against teq's
      // products uses, and reflection reads without a type variable the class file does not declare. Over
      // scalac's products it reads the generic signatures that item awaits. The trait built by teq, the classes
      // are the same bytes whole and split.
      val generic =
        if mixedBuild("genericclone", s"$jar:$genericLib", "tests/support/hidden_owner/genericclone/Use.scala") then mixedClient("genericclone", genericLib, mixed.resolve("genericclone"), genericScalac)
        else Nil
      val genericSrc = "tests/support/hidden_owner/genericclone"
      val genericSplit = mixedBuild("generic-whole", jar.toString, s"$genericSrc/G.scala", s"$genericSrc/Use.scala")
        && mixedBuild("generic-a", jar.toString, s"$genericSrc/G.scala")
        && mixedBuild("generic-b", s"$jar:${mixed.resolve("generic-a")}", s"$genericSrc/Use.scala")
        && List("E", "Generic", "GenericArray").forall(c => java.util.Arrays.equals(Files.readAllBytes(mixed.resolve(s"generic-whole/genericuse/$c.class")), Files.readAllBytes(mixed.resolve(s"generic-b/genericuse/$c.class"))))
      if generic == List("java.util.List\njava.util.List\njava.lang.Object[]", "java.util.List<java.lang.String>\njava.util.List<B>\nB[]") && genericSplit then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the raw forwarders of a generic clone do not read as raw: ${generic.map(_.replace('\n', ',')).mkString(" | ")}, whole and split alike $genericSplit")
      // A scalac-built trait's polymorphic clone (tests/support/hidden_owner/polyclone), which overrides no
      // `Object.clone()`: the class mixing it in has no bridge of `Object`'s (`Bridges.addBridgeIfNeeded` pairs
      // members whose types match before erasure), so that a cloneable subclass compiled by javac against
      // scalac's products clones itself through `Object.clone` over teq's products as over scalac's. The trait
      // built by teq, the class is the same bytes whole and split.
      val polySrc = "tests/support/hidden_owner/polyclone"
      val polyDriver = mixed.resolve("poly-driver")
      val polyRuns =
        if mixedBuild("polyclone", s"$jar:$polyLib", s"$polySrc/Use.scala") && Sh(javac, "--release", "17", "-nowarn", "-cp", s"$polyScalac:$polyLib:$jar", "-d", polyDriver.toString, s"$polySrc/Driver.java").mergeErr.timeout(120).check(false).run().ok then
          List(mixed.resolve("polyclone"), polyScalac).map(p => Sh("java", "-cp", s"$polyDriver:$p:$polyLib:$jar", "Driver").mergeErr.timeout(60).check(false).run().out.trim)
        else Nil
      val polySplit = mixedBuild("poly-whole", jar.toString, s"$polySrc/G.scala", s"$polySrc/Use.scala")
        && mixedBuild("poly-a", jar.toString, s"$polySrc/G.scala")
        && mixedBuild("poly-b", s"$jar:${mixed.resolve("poly-a")}", s"$polySrc/Use.scala")
        && java.util.Arrays.equals(Files.readAllBytes(mixed.resolve("poly-whole/polyclone/E.class")), Files.readAllBytes(mixed.resolve("poly-b/polyclone/E.class")))
      if polyRuns == List("Driver", "Driver") && polySplit then pass += 1
      else
        fail += 1
        println(s"FAIL owners: a polymorphic clone's forwarder bridges Object.clone: ${polyRuns.mkString(" | ")}, whole and split alike $polySplit")
      // Two traits' clones, an override of `Object`'s and a polymorphic one (tests/support/hidden_owner/mixedclone):
      // the class mixing both in has a forwarder of C's, which competes with `Object`'s
      // (`MixinOps.needsMixinForwarder`), so that `(new E: C).clone()` is C's, over the traits teq built (a split
      // build) as over scalac's. Over teq's own traits the general path forwards G's as well, as master's does
      // (`mixin_members` takes the two for unrelated definitions of one member), so the class differs between
      // the two. A whole build of the two is refused by the JavaScript naming of their alternatives, as master
      // refuses it (the report's note).
      val mixedSrc = "tests/support/hidden_owner/mixedclone"
      val mixedBuilt = mixedBuild("mixed-a", jar.toString, s"$mixedSrc/G.scala")
        && mixedBuild("mixed-b", s"$jar:${mixed.resolve("mixed-a")}", s"$mixedSrc/Use.scala")
        && mixedBuild("mixed-s", s"$jar:$mixedLib", s"$mixedSrc/Use.scala")
      val mixedRuns =
        if mixedBuilt then List(s"${mixed.resolve("mixed-b")}:${mixed.resolve("mixed-a")}", s"${mixed.resolve("mixed-s")}:$mixedLib").map(cp => Sh("java", "-cp", s"$cp:$jar", "mixedcloneuse.Main").mergeErr.timeout(60).check(false).run().out.trim)
        else Nil
      if mixedRuns == List("cloned", "cloned") then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the forwarder of one of two traits' clones: built $mixedBuilt, runs ${mixedRuns.mkString(" | ")}")
      // A trait's private clone (tests/support/hidden_owner/privateclone): no forwarder (`needsMixinForwarder`
      // excludes a private member), from the source as from the products, the class the same bytes whole and
      // split; the program runs, and javac refuses a Java call of the class's clone, `Object`'s protected one,
      // as against scalac's class.
      val privateSrc = "tests/support/hidden_owner/privateclone"
      val privateSplit = mixedBuild("private-whole", jar.toString, s"$privateSrc/G.scala", s"$privateSrc/Use.scala")
        && mixedBuild("private-a", jar.toString, s"$privateSrc/G.scala")
        && mixedBuild("private-b", s"$jar:${mixed.resolve("private-a")}", s"$privateSrc/Use.scala")
        && java.util.Arrays.equals(Files.readAllBytes(mixed.resolve("private-whole/privatecloneuse/E.class")), Files.readAllBytes(mixed.resolve("private-b/privatecloneuse/E.class")))
      val privateRun = if privateSplit then Sh("java", "-cp", s"${mixed.resolve("private-whole")}:$jar", "privatecloneuse.Main").mergeErr.timeout(60).check(false).run().out.trim else ""
      val privateRefused = List(mixed.resolve("private-whole").toString, s"$privateScalac:$privateLib").map(cp =>
        Sh(javac, "--release", "17", "-nowarn", "-cp", s"$cp:$jar", "-d", mixed.resolve("private-client").toString, s"$privateSrc/Client.java").mergeErr.timeout(120).check(false).run())
      if privateSplit && privateRun == "private" && privateRefused.forall(r => !r.ok && r.out.contains("clone() has protected access")) then pass += 1
      else
        fail += 1
        println(s"FAIL owners: a trait's private clone is forwarded: whole and split alike $privateSplit, run $privateRun, javac ${privateRefused.map(r => if r.ok then "accepted" else "refused").mkString(" | ")}")
      // A scalac-built trait's annotated parameters (tests/support/hidden_owner/parammeta): the class mixing it in
      // forwards its `equals`, which competes with `Object`'s, with the parameter's annotation
      // (`MixinOps.mkForwarderSym` copies the parameters' annotations), and not its `clone(String)`, which
      // competes with no class's method, so that reflection reads each parameter's annotation over teq's
      // products as over scalac's. The class is the same bytes whole and split, the trait built by teq.
      val paramSrc = "tests/support/hidden_owner/parammeta"
      val paramReads =
        if mixedBuild("param", s"$jar:$paramLib", s"$paramSrc/Use.scala") then mixedClient("parammeta", paramLib, mixed.resolve("param"), paramScalac) else Nil
      val paramTag = mixed.resolve("param-tag")
      val paramSplit = Sh(javac, "--release", "17", "-nowarn", "-d", paramTag.toString, s"$paramSrc/pm/Tag.java").mergeErr.timeout(120).check(false).run().ok
        && mixedBuild("param-whole", s"$jar:$paramTag", s"$paramSrc/G.scala", s"$paramSrc/Use.scala")
        && mixedBuild("param-a", s"$jar:$paramTag", s"$paramSrc/G.scala")
        && mixedBuild("param-b", s"$jar:$paramTag:${mixed.resolve("param-a")}", s"$paramSrc/Use.scala")
        && java.util.Arrays.equals(Files.readAllBytes(mixed.resolve("param-whole/pmuse/E.class")), Files.readAllBytes(mixed.resolve("param-b/pmuse/E.class")))
      if paramReads == List.fill(2)("[[@pm.Tag(\"arg\")]]\n[[@pm.Tag(\"eq\")]]") && paramSplit then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the parameters' annotations of a trait's methods through the class: ${paramReads.map(_.replace('\n', ',')).mkString(" | ")}, whole and split alike $paramSplit")
      // An ascription in an inline body (owners/inline_abi), built whole and split against the library's
      // first version: the client names the ascribed class (dotty's inliner keeps the `Typed` tree,
      // `Erasure.typedTyped` erases it to its target), the same bytes either way, and runs on the second
      // version, where the member moved to that class.
      val abi = Files.createTempDirectory("teq-inlineabi.")
      Script.atExit(Sh.remove(abi))
      val abiSrc = "tests/classfile/owners/inline_abi"
      def abiBuild(out: String, cp: String, sources: String*): Boolean =
        Sh((List(teq, "compiler", "build") ++ sources.map(f => s"$abiSrc/$f") ++ List("--target", "jvm", "--classpath", cp, "--products", abi.resolve(out).toString))*)
          .stdout(abi.resolve(s"$out.log")).mergeErr.timeout(120).check(false).run().ok
      val abiBuilt = abiBuild("whole", s"$jar:$inlineV1", "Lib.scala", "Main.scala")
        && abiBuild("a", s"$jar:$inlineV1", "Lib.scala")
        && abiBuild("b", s"$jar:$inlineV1:${abi.resolve("a")}", "Main.scala")
      val abiMain = "inlineuse/Main$.class"
      val abiSame = abiBuilt && java.util.Arrays.equals(Files.readAllBytes(abi.resolve("whole").resolve(abiMain)), Files.readAllBytes(abi.resolve("b").resolve(abiMain)))
      val abiRuns =
        if abiBuilt then
          for
            products <- List(abi.resolve("whole").toString, s"${abi.resolve("b")}:${abi.resolve("a")}")
            lib <- List(inlineV1, inlineV2)
          yield Sh("java", "-cp", s"$products:$lib:$jar", "inlineuse.Main").mergeErr.timeout(60).check(false).run().out.trim
        else Nil
      if abiSame && abiRuns == List("List(7, 7)", "List(9, 9)", "List(7, 7)", "List(9, 9)") then pass += 1
      else
        fail += 1
        println(s"FAIL owners: an inline body's ascription: built $abiBuilt, whole and split alike $abiSame, runs ${abiRuns.mkString(" | ")}")
        List("whole", "a", "b").map(o => abi.resolve(s"$o.log")).filter(Files.exists(_)).foreach(f => Files.readAllLines(f).toArray.take(5).foreach(println))
      val declared = Sh("javap", "-p", "-cp", hidden.toString, "hidden.Shown", "hidden.Leaf").timeout(60).check(false).run()
      val bridges = declared.lines.filter(l => l.contains("(") && !l.contains("hidden.Shown(") && !l.contains("hidden.Leaf("))
      if declared.ok && declared.lines.exists(_.contains("hidden.Shown(")) && bridges.isEmpty then pass += 1
      else
        fail += 1
        println("FAIL owners: the public subclasses of the Java fixture declare members javac bridged")
        (declared.lines ++ declared.err.linesIterator).take(10).foreach(println)
      val interpreted = Sh(teq, "interp", "tests/classpath/jvm/hidden_owner.scala", "--classpath", hidden.toString).timeout(120).check(false).run()
      if interpreted.status == 3 && interpreted.out.isEmpty && interpreted.err.contains("has no body for method to run") then pass += 1
      else
        fail += 1
        println(s"FAIL owners: the interpreter's run of the Java fixture's program, exit code ${interpreted.status}")
        (interpreted.out.linesIterator ++ interpreted.err.linesIterator).take(5).foreach(println)
    else println("skip owners: the Java fixture's jars do not build (javac or scala-cli missing)")
  else println("skip owners: no javap, or scala-library not in the coursier cache")
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
