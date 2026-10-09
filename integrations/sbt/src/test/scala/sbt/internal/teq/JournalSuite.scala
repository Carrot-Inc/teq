package sbt.internal.teq

import java.io.File
import java.nio.file.Files

import sbt.io.IO

/** The rollback of a run (`TeqCompile.Journal`) over a class directory sbt's cache restored as links
  * into its store, one of whose blobs the store has lost. */
class JournalSuite extends munit.FunSuite:
  test("a link whose blob is gone is left as it is: the run begins, and a failed one puts back the rest"):
    val base = Files.createTempDirectory("teq-journal").toFile
    val classes = new File(base, "classes")
    val store = new File(base, "cas")
    IO.write(new File(classes, "r/R.class"), "runtime")
    IO.write(new File(store, "kept"), "kept")
    IO.createDirectory(new File(classes, "p"))
    Files.createSymbolicLink(new File(classes, "p/Kept.class").toPath, new File(store, "kept").toPath)
    val gone = new File(classes, "p/Gone.class").toPath
    Files.createSymbolicLink(gone, new File(store, "gone").toPath)
    val journal = new TeqCompile.Journal(classes)
    journal.begin()
    IO.write(new File(classes, "r/R.class"), "replaced")
    IO.write(new File(classes, "p/Added.class"), "added")
    journal.complete(success = false)
    assertEquals(IO.read(new File(classes, "r/R.class")), "runtime")
    assertEquals(IO.read(new File(classes, "p/Kept.class")), "kept")
    assert(!new File(classes, "p/Added.class").exists)
    assert(Files.isSymbolicLink(gone) && !Files.exists(gone))
    assert(!new File(classes.getPath + ".teq-run").exists)
    IO.delete(base)
