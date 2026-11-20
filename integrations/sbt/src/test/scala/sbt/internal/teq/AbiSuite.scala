package sbt.internal.teq

import java.io.File
import java.nio.file.Files

import sbt.io.IO
import sbt.util.Logger

/** The number of teq's products (`TeqCompile.Abi`): a class directory of another number emptied with its analysis
  * before zinc's run, one of the binary's own left as it is, an upstream directory of another number refused before
  * a batch, and the binary's number read from `teq --version --verbose`. */
class AbiSuite extends munit.FunSuite {
  private def directory(base: File, name: String, manifest: String): File = {
    val classes = new File(base, name)
    IO.write(new File(classes, "p/A.class"), "class")
    IO.write(new File(classes, TeqCompile.manifestName), manifest)
    classes
  }

  test("a class directory of another number is emptied with its analysis; one of the binary's, or of 0 without the field, stays") {
    val base = Files.createTempDirectory("teq-abi").toFile
    val analysis = new File(base, "analysis.zip")
    val other = directory(base, "other", """{"identity":"0","teq":"0.1.12","abi":1,"products":[]}""")
    val same = directory(base, "same", """{"identity":"0","teq":"0.1.12","abi":1,"products":[]}""")
    val none = directory(base, "none", """{"identity":"0","teq":"0.1.11","products":[]}""")
    IO.write(analysis, "zinc")
    assert(TeqCompile.Abi.check(other, analysis, 0, Logger.Null))
    assert(!other.exists && !analysis.exists)
    IO.write(analysis, "zinc")
    assert(!TeqCompile.Abi.check(same, analysis, 1, Logger.Null))
    assert(!TeqCompile.Abi.check(none, analysis, 0, Logger.Null))
    assert(new File(same, "p/A.class").isFile && new File(none, "p/A.class").isFile && analysis.isFile)
    assert(TeqCompile.Abi.check(none, analysis, 1, Logger.Null))
    assert(!none.exists && !analysis.exists)
    IO.delete(base)
  }

  test("an upstream directory of another number is refused, naming its manifest, the writer and both numbers") {
    val base = Files.createTempDirectory("teq-abi-upstream").toFile
    val classes = directory(base, "own", """{"identity":"0","teq":"0.1.11","products":[]}""")
    val upstream = directory(base, "upstream", """{"identity":"0","teq":"0.1.11","products":[]}""")
    TeqCompile.Abi.refuseUpstream(Seq(classes, upstream), classes, 0)
    val refused = intercept[sbt.MessageOnlyException](TeqCompile.Abi.refuseUpstream(Seq(classes, upstream), classes, 1))
    assertEquals(refused.getMessage, s"teq: ${new File(upstream, TeqCompile.manifestName)} holds products of ABI 0 (written by teq 0.1.11), this teq reads ABI 1: compile the project of $upstream first")
    IO.delete(base)
  }

  test("the binary's number from --version --verbose; none printed (a teq up to 0.1.11) is 0") {
    assertEquals(TeqCompile.Abi.ofVersion(Seq("teq 0.1.12 abc", "products ABI 3")), 3)
    assertEquals(TeqCompile.Abi.ofVersion(Seq("teq 0.1.11 736ca15c1d")), 0)
  }
}
