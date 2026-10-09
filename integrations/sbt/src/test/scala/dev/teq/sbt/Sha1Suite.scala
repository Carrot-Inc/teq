package dev.teq.sbt

import java.io.File
import java.nio.file.Files

import sbt.io.IO

class Sha1Suite extends munit.FunSuite:
  private val checksum = "8d3484e95d1eb36004dfe79daacf628255736a20"
  private val abc = "a9993e364706816aba3e25717850c26c9cd0d89d"

  test("the checksum a stamp or a repository's file carries"):
    assertEquals(Sha1.in(checksum), Some(checksum))
    assertEquals(Sha1.in(s"$checksum\n"), Some(checksum))
    assertEquals(Sha1.in(s"$checksum  teq-0.1.1-osx-aarch_64.exe\n"), Some(checksum))
    assertEquals(Sha1.in(checksum.toUpperCase), Some(checksum))
    assertEquals(Sha1.in("<html><body><h1>404 Not Found</h1></body></html>"), None)
    assertEquals(Sha1.in(""), None)
    assertEquals(Sha1.in(checksum.dropRight(1)), None)

  test("the stamp beside a copy"):
    val dir = Files.createTempDirectory("teq-sha1").toFile
    val copy = new File(dir, "teq-0.1.1-osx-aarch_64")
    assertEquals(Sha1.stamped(copy), None)
    IO.write(copy, "abc")
    assertEquals(Sha1.compute(copy), abc)
    assertEquals(Sha1.stamped(copy), Some(abc))
    assertEquals(IO.read(Sha1.stamp(copy)), abc)
    IO.write(Sha1.stamp(copy), checksum)
    assertEquals(Sha1.stamped(copy), Some(checksum))
    IO.write(Sha1.stamp(copy), "not a checksum")
    assertEquals(Sha1.stamped(copy), Some(abc))
    assertEquals(Sha1.writeStamp(copy), abc)
    IO.delete(dir)

  test("the digest of a resolved file is its bytes', whatever coursier's checksum beside it says"):
    val dir = Files.createTempDirectory("teq-sha1-digest").toFile
    val file = new File(dir, "teq-0.1.1-osx-aarch_64.exe")
    IO.write(file, "abc")
    IO.write(new File(file.getPath + ".sha1"), s"$checksum  teq-0.1.1-osx-aarch_64.exe\n")
    assertEquals(Sha1.of(file), Some(abc))
    IO.write(file, "abcde")
    assertEquals(Sha1.of(file), Some("03de6c570bfe24bfc328ccd7ca46b76eadaf4334"))
    assertEquals(Sha1.of(new File(dir, "missing.exe")), None)
    IO.delete(dir)
