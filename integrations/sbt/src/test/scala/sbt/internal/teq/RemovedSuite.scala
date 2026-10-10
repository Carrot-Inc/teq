package sbt.internal.teq

import java.io.File
import java.nio.file.Files

import sbt.io.IO

/** The sources a configuration's batch names `--removed` (`TeqCompile.removed`) when sbt's cache has
  * brought the class directory, its manifest among its files, from another checkout of the build. */
class RemovedSuite extends munit.FunSuite {
  test("a manifest another checkout wrote names this checkout's sources: only one the configuration lacks is removed, as this checkout's file") {
    val base = Files.createTempDirectory("teq-removed").toFile.getCanonicalFile
    val other = new File(base, "other")
    val here = new File(base, "here")
    for (dir <- Seq(other, here); name <- Seq("A", "B")) IO.write(new File(dir, s"src/$name.scala"), "")
    val classes = new File(here, "target/classes")
    val entries = Seq("A", "B", "Gone").map(name => s"""{"source":"${new File(other, s"src/$name.scala").getPath}"}""")
    IO.write(new File(classes, TeqCompile.manifestName), s"""{"root":"${other.getPath}","products":[${entries.mkString(",")}]}""")
    val sources = Seq("A", "B").map(name => new File(here, s"src/$name.scala"))
    assertEquals(TeqCompile.removed(classes, here, sources), Seq(new File(here, "src/Gone.scala").getPath))
    assertEquals(TeqCompile.removed(classes, other, sources), Seq("A", "B", "Gone").map(name => new File(other, s"src/$name.scala").getPath))
    IO.delete(base)
  }

  test("a source through a link and `..` is the file the filesystem names, in the manifest's checkout and in another") {
    val base = Files.createTempDirectory("teq-removed-link").toFile.getCanonicalFile
    val external = new File(base, "external")
    IO.write(new File(external, "B.scala"), "")
    IO.createDirectory(new File(external, "sub"))
    for (checkout <- Seq("old", "new")) {
      IO.createDirectory(new File(base, checkout))
      Files.createSymbolicLink(new File(base, s"$checkout/link").toPath, java.nio.file.Paths.get("../external/sub"))
    }
    val (old, here) = (new File(base, "old"), new File(base, "new"))
    val classes = new File(here, "target/classes")
    IO.write(new File(classes, TeqCompile.manifestName), s"""{"root":"${old.getPath}","products":[{"source":"link/../B.scala"}]}""")
    val real = new File(external, "B.scala")
    assertEquals(TeqCompile.removed(classes, old, Seq(real)), Nil)
    assertEquals(TeqCompile.removed(classes, here, Seq(real)), Nil)
    assertEquals(TeqCompile.removed(classes, here, Nil), Seq(real.getPath))
    IO.delete(base)
  }

  test("a manifest of a format this plugin does not read is refused, naming the teq that wrote it, before anything is removed") {
    val base = Files.createTempDirectory("teq-removed-format").toFile.getCanonicalFile
    val classes = new File(base, "target/classes")
    IO.write(new File(base, "src/A.scala"), "")
    val entry = s"""{"source":"${new File(base, "src/A.scala").getPath}"}"""
    for (format <- Seq("", "\"format\":1,")) {
      IO.write(new File(classes, TeqCompile.manifestName), s"""{$format"teq":"0.1.7","root":"${base.getPath}","products":[$entry]}""")
      assertEquals(TeqCompile.removed(classes, base, Nil), Seq(new File(base, "src/A.scala").getPath))
    }
    IO.write(new File(classes, TeqCompile.manifestName), s"""{"format":2,"teq":"0.2.0","root":"${base.getPath}","products":[{"source":"A.scala","classFiles":[]}]}""")
    val refused = intercept[Exception](TeqCompile.removed(classes, base, Nil))
    assert(refused.getMessage.contains("format 2, which teq 0.2.0 wrote"), refused.getMessage)
    IO.delete(base)
  }
}
