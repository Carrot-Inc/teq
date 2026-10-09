//> using platform jvm
// The readers and writers of `java.nio.file.Files` on each kind of path, against the JDK: a file of
// non-ASCII text, a path that is not there, a directory, bytes that are not UTF-8 (the length of
// the malformed input as each reader states it), relative and absolute; the exceptions are the
// JDK's, which a `catch` of `IOException` takes. Its files are under target/ of the working
// directory, removed at the end.
import java.io.IOException
import java.nio.file.{Files, Path, Paths}
import scala.jdk.CollectionConverters.*

object Main:
  val dir = System.getProperty("user.dir")
  def shown(text: String): String = if text == null then "null" else text.replace(dir, "<dir>")

  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + shown(String.valueOf(f)))
    catch
      case e: IOException => println(name + ": IOException " + e.getClass.getName + ": " + shown(e.getMessage))
      case e: Exception => println(name + ": " + e.getClass.getName + ": " + shown(e.getMessage))

  def lines(l: java.util.List[String]): String = l.asScala.map(s => "<" + s + ">").mkString("[", ",", "]")
  def bytes(b: Array[Byte]): String = b.map(_ & 255).mkString(" ")

  def remove(p: Path): Unit =
    if Files.exists(p) then
      val s = Files.walk(p)
      val all = try s.iterator.asScala.toList finally s.close()
      all.reverse.foreach(Files.deleteIfExists)

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_read_write")
    remove(root)
    Files.createDirectories(root)
    try run(root)
    finally remove(root)

  def run(root: Path): Unit =
    val text = root.resolve("text.txt")
    val absent = root.resolve("absent.txt")
    val malformed = root.resolve("malformed.txt")
    Files.writeString(text, "héllo wörld ☃ 😀\n\n")
    Files.write(malformed, Array(0x41, 0xC3, 0x28).map(_.toByte))
    for (kind, p) <- List("relative" -> text, "absolute" -> text.toAbsolutePath) do
      show("readString " + kind)(Files.readString(p))
      show("readAllLines " + kind)(lines(Files.readAllLines(p)))
      show("readAllBytes " + kind)(bytes(Files.readAllBytes(p)))
    for (kind, p) <- List("absent" -> absent, "absent absolute" -> absent.toAbsolutePath, "directory" -> root, "directory absolute" -> root.toAbsolutePath) do
      show("readString " + kind)(Files.readString(p))
      show("readAllLines " + kind)(Files.readAllLines(p))
      show("readAllBytes " + kind)(Files.readAllBytes(p))
      show("getLastModifiedTime " + kind)(Files.getLastModifiedTime(p).toMillis > 0)
    show("readString malformed")(Files.readString(malformed))
    show("readAllLines malformed")(Files.readAllLines(malformed))
    show("readAllBytes malformed")(bytes(Files.readAllBytes(malformed)))
    // Each reader's length of the first malformed input: a sequence cut short, a surrogate, a
    // code point past U+10FFFF, a lead byte without its continuation.
    for (name, b) <- List("cut" -> Seq(0x41, 0xE2, 0x82), "surrogate" -> Seq(0xED, 0xA0, 0x80), "above" -> Seq(0xF4, 0x90, 0x80, 0x80), "lead" -> Seq(0xE2, 0x82, 0x28), "four" -> Seq(0xF0, 0x9F, 0x98, 0x28)) do
      Files.write(malformed, b.map(_.toByte).toArray)
      show("readString " + name)(Files.readString(malformed))
      show("readAllLines " + name)(Files.readAllLines(malformed))
    try Files.readString(malformed)
    catch case e: java.nio.charset.CharacterCodingException => println("caught as CharacterCodingException: " + e.getMessage)
    // The lines as `BufferedReader.readLine` ends them: at \n, \r and \r\n, none started by the
    // last terminator.
    for (name, t) <- List("trailing" -> "héllo\n\n", "cr" -> "a\rb\r", "crlf" -> "a\r\n\r\nb", "none" -> "x", "empty" -> "", "blank" -> "\n", "crcrlf" -> "\r\r\n") do
      Files.writeString(text, t)
      show("lines " + name)(lines(Files.readAllLines(text)))
    // writeString: UTF-8, the file created or truncated, the path answered; a parent that is not
    // there is not made.
    val written = root.resolve("written.txt")
    show("writeString new")(Files.writeString(written, "a longer first text") eq written)
    show("writeString truncates")(bytes(Files.readAllBytes(Files.writeString(written, "é\n"))))
    show("writeString supplementary")(bytes(Files.readAllBytes(Files.writeString(written, "😀"))))
    show("writeString builder")(Files.readString(Files.writeString(written, new java.lang.StringBuilder("built"))))
    // A surrogate that pairs with none is unmappable, and the file is left as it was.
    show("writeString lone surrogate")(Files.writeString(written, "x\uD800y"))
    show("writeString lone low surrogate")(Files.writeString(root.resolve("never.txt"), "\uDC00"))
    show("left as it was")(Files.readString(written) + ", " + Files.exists(root.resolve("never.txt")))
    // A null text or array is refused before the file is opened, and the file keeps its contents.
    show("writeString null")(Files.writeString(written, null))
    show("write null")(Files.write(written, null.asInstanceOf[Array[Byte]]))
    show("kept")(Files.readString(written))
    show("UncheckedIOException without its cause")(new java.io.UncheckedIOException(null.asInstanceOf[IOException]))
    show("writeString absolute")(Files.readString(Files.writeString(written.toAbsolutePath, "absolute")))
    show("writeString absent parent")(Files.writeString(root.resolve("absent/child.txt"), "x"))
    show("writeString absent parent absolute")(Files.writeString(root.resolve("absent/child.txt").toAbsolutePath, "x"))
    show("writeString directory")(Files.writeString(root, "x"))
    show("writeString under a file")(Files.writeString(written.resolve("x"), "x"))
    show("write bytes")(bytes(Files.readAllBytes(Files.write(written, Array[Byte](0, -1, 10)))))
    show("write absent parent")(Files.write(root.resolve("absent/child.bin"), Array[Byte](1)))
    try Files.readString(absent)
    catch case e: java.nio.file.FileSystemException => println("file " + e.getFile + ", other " + e.getOtherFile + ", reason " + e.getReason)
