package dev.teq.sbt

import java.io.File
import java.nio.charset.StandardCharsets
import java.nio.file.Files

import sbt.io.IO

class ArgsFileSuite extends munit.FunSuite:
  private def utf8(file: File) = new String(Files.readAllBytes(file.toPath), StandardCharsets.UTF_8)

  test("a command's arguments go to a file named by them, one per line in UTF-8, and the command names the file"):
    val base = Files.createTempDirectory("teq-args").toFile
    val place = ArgsFile.of(base, "test", "batch")
    assertEquals(place.dir, new File(base, "target/teq"))
    val args = Seq("compiler", "build", "C:\\Users\\Zoë\\a dir\\Main.scala", "--classpath", "/a b/ünï.jar:/c.jar", "@not-a-file", "")
    val command = ArgsFile.command("/bin/teq" +: args, place)
    val file = ArgsFile.file(place, args)
    assert(file.getName.matches("test-batch-[0-9a-f]{16}\\.args"), file.getName)
    assertEquals(command, Seq("/bin/teq", "@" + file.getAbsolutePath))
    assertEquals(utf8(file), args.map(_ + "\n").mkString)
    assertEquals(Option(place.dir.list).toSeq.flatten, Seq(file.getName))
    IO.delete(base)

  test("a file that holds the arguments already is left as it is but for its time"):
    val base = Files.createTempDirectory("teq-args").toFile
    val place = ArgsFile.of(base, "compile", "build")
    val file = ArgsFile.write(place, Seq("compiler", "build", "a"))
    val identity = Files.readAttributes(file.toPath, classOf[java.nio.file.attribute.BasicFileAttributes]).fileKey
    file.setLastModified(1000000000000L)
    assertEquals(ArgsFile.write(place, Seq("compiler", "build", "a")), file)
    assertEquals(Files.readAttributes(file.toPath, classOf[java.nio.file.attribute.BasicFileAttributes]).fileKey, identity)
    assert(file.lastModified > 1000000000000L)
    IO.delete(base)

  // A race: two builds of one base into two outputs, started at once, each
  // reading its own arguments (a fixed name had the one that wrote last serve both).
  test("two commands of one base and kind with other arguments, written at once, have files of their own"):
    val base = Files.createTempDirectory("teq-args").toFile
    val place = ArgsFile.of(base, "compile", "build")
    val (one, two) = (Seq("compiler", "build", "src", "-o", "out/one.js"), Seq("compiler", "build", "src", "-o", "out/two.js"))
    val pool = java.util.concurrent.Executors.newFixedThreadPool(8)
    val writes = (1 to 64).map(i => pool.submit(() => ArgsFile.command("teq" +: (if i % 2 == 0 then one else two), place)))
    val commands = writes.map(_.get()).distinct
    pool.shutdown()
    assertEquals(commands.size, 2)
    val (fileOne, fileTwo) = (ArgsFile.file(place, one), ArgsFile.file(place, two))
    assert(fileOne != fileTwo)
    assertEquals(utf8(fileOne), one.map(_ + "\n").mkString)
    assertEquals(utf8(fileTwo), two.map(_ + "\n").mkString)
    assertEquals(Option(place.dir.list).toSeq.flatten.sorted, Seq(fileOne.getName, fileTwo.getName).sorted)
    IO.delete(base)

  test("two links into directories of one name have files of their own"):
    val base = Files.createTempDirectory("teq-args").toFile
    val (a, b) = (new File(base, "a/served"), new File(base, "b/served"))
    val args = Seq("compiler", "watch", "src", "--split", "served")
    val (pa, pb) = (ArgsFile.into(base, "compile", "link", a), ArgsFile.into(base, "compile", "link", b))
    assert(pa != pb && ArgsFile.file(pa, args) != ArgsFile.file(pb, args))
    assertEquals(ArgsFile.into(base, "compile", "link", new File(base, "a/../a/served")), pa)
    assert(ArgsFile.file(pa, args).getName.matches("compile-link-served-[0-9a-f]{8}-[0-9a-f]{16}\\.args"), ArgsFile.file(pa, args).getName)
    IO.delete(base)

  // A directory's name cut at a code point within the byte budget: an emoji name,
  // whose `take(64)` split a surrogate pair, and a long ASCII one.
  test("a link's directory name is cut between code points, its file's name within the budget"):
    val base = Files.createTempDirectory("teq-args").toFile
    // A JVM whose file names are ASCII (a Linux locale that is not UTF-8) cannot name the emoji's
    // file at all, nor its directory: the write is checked where it can.
    val names = java.nio.charset.Charset.forName(System.getProperty("sun.jnu.encoding", "UTF-8")).newEncoder
    for name <- Seq("a" + "😀" * 40, "s" * 230) do
      val place = ArgsFile.into(base, "compile", "link", new File(base, name))
      val args = Seq("compiler", "watch", "--split", name)
      val chosen = ArgsFile.file(place, args).getName
      assertEquals(new String(chosen.getBytes(StandardCharsets.UTF_8), StandardCharsets.UTF_8), chosen)
      assert(chosen.getBytes(StandardCharsets.UTF_8).length <= ArgsFile.NameBytes, chosen)
      assert(chosen.startsWith("compile-link-" + name.take(1)), chosen)
      if names.canEncode(chosen) then assertEquals(utf8(ArgsFile.write(place, args)), s"compiler\nwatch\n--split\n$name\n")
    assertEquals(ArgsFile.truncated("a" + "😀" * 40, 64), "a" + "😀" * 15)
    assertEquals(ArgsFile.truncated("é" * 3, 5), "éé")
    IO.delete(base)

  // The files' accumulation: a write removes its stem's files older than an hour,
  // never the one it writes, nor a fresh sibling, another stem's or a name of an earlier version.
  test("a write removes its stem's files older than an hour, a fresh sibling and the one written kept"):
    val base = Files.createTempDirectory("teq-args").toFile
    val place = ArgsFile.of(base, "compile", "batch")
    val hours = (n: Int) => System.currentTimeMillis - n * 60L * 60 * 1000
    val old = ArgsFile.write(place, Seq("compiler", "build", "Old.scala"))
    val fresh = ArgsFile.write(place, Seq("compiler", "build", "Fresh.scala"))
    val other = ArgsFile.write(ArgsFile.of(base, "test", "batch"), Seq("compiler", "build", "Other.scala"))
    val legacy = new File(place.dir, "compile-batch.args")
    val partial = new File(place.dir, old.getName.stripSuffix(".args").dropRight(2) + "00.args.4711.tmp")
    IO.write(legacy, "compiler\n")
    IO.write(partial, "compiler\n")
    for f <- Seq(old, other, legacy, partial) do f.setLastModified(hours(2))
    val written = ArgsFile.write(place, Seq("compiler", "build", "New.scala"))
    assert(!old.exists, "an old file of the stem stays")
    assert(!partial.exists, "an old temporary file of the stem stays")
    assert(fresh.exists && other.exists && legacy.exists && written.exists)
    // A file written again with the same arguments is fresh for the next sweep, however old it was.
    fresh.setLastModified(hours(3))
    assertEquals(ArgsFile.write(place, Seq("compiler", "build", "Fresh.scala")), fresh)
    assert(fresh.exists && fresh.lastModified > hours(1))
    ArgsFile.write(place, Seq("compiler", "build", "Newer.scala"))
    assert(fresh.exists && written.exists)
    IO.write(new File(place.dir, "teq.lock"), "teq: 0.1.5\n")
    assertEquals(ArgsFile.files(base).map(_.getName).sorted, place.dir.list.toSeq.filter(_.endsWith(".args")).sorted)
    assert(!ArgsFile.files(base).exists(_.getName == "teq.lock"), "clean would take the lock")
    IO.delete(base)

  test("an argument with a line end is refused, no file written"):
    val base = Files.createTempDirectory("teq-args").toFile
    val place = ArgsFile.of(base, "compile", "batch")
    for bad <- Seq("two\nlines", "a\r") do
      val e = intercept[sbt.MessageOnlyException](ArgsFile.write(place, Seq("compiler", "watch", bad)))
      assert(e.getMessage.contains("holds a line end"), e.getMessage)
    assert(!place.dir.exists || place.dir.list.isEmpty)
    IO.delete(base)
