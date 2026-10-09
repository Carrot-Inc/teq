//> using platform jvm
// os: unix
//> using file ../../tools/script
// A large output stays on disk: a result's texts are a bounded head and tail of its spool file
// (`Sh.TextBound`), cut at whole UTF-8 sequences, around a line saying how much was left out; the
// tail of stderr in a failure's message is read from the file's end (`Sh.TailBound`); `outAll`
// reads the whole. The child writes an 8 MiB line to stderr and 1.5 MB of short lines to stdout.
object Main:
  def main(args: Array[String]): Unit = Script.run {
    val r = Sh("sh", "-c", "head -c 8388608 /dev/zero | tr '\\0' x >&2; yes 'aéb' | head -c 1500001; exit 1").check(false).run()
    println("status: " + r.status)
    println("stderr file: " + java.nio.file.Files.size(r.errFile))
    println("stderr text: " + r.err.length)
    println("stderr tail: " + r.errTail.length)
    println("failure's message: " + new Sh.Failed(r).getMessage.length)
    println("stdout file: " + java.nio.file.Files.size(r.outFile))
    println("stdout text: " + r.out.length + " " + r.out.linesIterator.count(_.startsWith("[")))
    println("left out: " + r.out.linesIterator.find(_.startsWith("[")).getOrElse(""))
    println("whole: " + r.outAll().length)
    println("lines kept whole: " + r.lines.forall(l => l == "aéb" || l.startsWith("[") || "aéb".endsWith(l) || "aéb".startsWith(l)))
  }
