//> using platform jvm
// The open options of `Files.write`, `newOutputStream` and `newBufferedWriter` as the JDK takes them,
// each a flag of its own (`UnixChannelFactory`): none is CREATE, TRUNCATE_EXISTING and WRITE; WRITE
// alone writes over the start of the file and keeps the rest; CREATE makes a missing file without
// emptying one that is there; CREATE_NEW refuses one that is there; APPEND writes at the end;
// READ and APPEND with TRUNCATE_EXISTING refused. `FileOutputStream` truncates or appends.
import java.nio.file.{Files, Path, StandardOpenOption as O, OpenOption}
import java.nio.charset.StandardCharsets.UTF_8

object Main:
  def main(args: Array[String]): Unit =
    val dir = Files.createTempDirectory("open-options")
    val p = dir.resolve("data")
    def run(name: String, existing: Boolean, options: OpenOption*): Unit =
      Files.deleteIfExists(p)
      if existing then Files.writeString(p, "abcdef")
      val r =
        try
          Files.write(p, "XY".getBytes, options*)
          Files.readString(p)
        catch
          case e: IllegalArgumentException => "IllegalArgumentException: " + e.getMessage
          case e: Exception => e.getClass.getName
      println(s"$name on ${if existing then "a file" else "no file"}: $r")
    for existing <- List(true, false) do
      run("none", existing)
      run("WRITE", existing, O.WRITE)
      run("CREATE", existing, O.CREATE)
      run("CREATE_NEW", existing, O.CREATE_NEW)
      run("TRUNCATE_EXISTING", existing, O.TRUNCATE_EXISTING)
      run("APPEND", existing, O.APPEND)
      run("CREATE APPEND", existing, O.CREATE, O.APPEND)
      run("CREATE TRUNCATE_EXISTING", existing, O.CREATE, O.TRUNCATE_EXISTING)
      run("CREATE_NEW TRUNCATE_EXISTING", existing, O.CREATE_NEW, O.TRUNCATE_EXISTING)
      run("APPEND TRUNCATE_EXISTING", existing, O.APPEND, O.TRUNCATE_EXISTING)
      run("READ", existing, O.READ)
      run("READ APPEND", existing, O.READ, O.APPEND)
      run("WRITE APPEND", existing, O.WRITE, O.APPEND)
    Files.writeString(p, "abcdef")
    val w = Files.newBufferedWriter(p, UTF_8, O.WRITE)
    w.write("XY")
    w.close()
    println("newBufferedWriter WRITE: " + Files.readString(p))
    Files.writeString(p, "abcdef", O.WRITE)
    println("writeString WRITE: " + Files.readString(p))
    val o = Files.newOutputStream(p, O.APPEND)
    o.write('!')
    o.close()
    println("newOutputStream APPEND: " + Files.readString(p))
    val a = new java.io.FileOutputStream(p.toFile, true)
    a.write('?')
    a.close()
    println("FileOutputStream appending: " + Files.readString(p))
    val t = new java.io.FileOutputStream(p.toFile)
    t.write('.')
    t.close()
    println("FileOutputStream: " + Files.readString(p))
    Files.delete(p)
    Files.delete(dir)
