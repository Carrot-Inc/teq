//> using platform jvm
// os: unix
// The rest of `java.nio.file.Files` a script calls, against the JDK: a walk that follows links and
// fails on a loop; copy, move and delete with their options and refusals; size and modification
// time; symbolic links; temporary files and directories; directory streams with a glob; lines,
// readers and writers; the options of a write; permissions; `isSameFile`; the names of a `Path`; and
// `java.io.File`. The files are under target/ of the working directory, removed at the end; a
// listing whose order is the system's is sorted.
import java.io.File
import java.nio.charset.StandardCharsets
import java.nio.file.*
import java.nio.file.attribute.{FileTime, PosixFilePermissions}
import scala.jdk.CollectionConverters.*

object Main:
  val dir = System.getProperty("user.dir")
  def shown(text: String): String = if text == null then "null" else text.replace(dir, "<dir>")

  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + shown(String.valueOf(f)))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + shown(String.valueOf(e.getMessage)))

  def all(xs: Any*): String = xs.mkString(" ")

  def tree(p: Path, options: FileVisitOption*): String =
    val s = Files.walk(p, options*)
    try s.iterator.asScala.map(x => if x == p then "." else p.relativize(x).toString).toList.sorted.mkString(",")
    finally s.close()

  def remove(p: Path): Unit =
    if Files.exists(p, LinkOption.NOFOLLOW_LINKS) then
      val s = Files.walk(p)
      val all = try s.iterator.asScala.toList finally s.close()
      all.reverse.foreach(Files.deleteIfExists)

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_walk_copy")
    remove(root)
    Files.createDirectories(root)
    try run(root)
    finally remove(root)

  def run(root: Path): Unit =
    val a = root.resolve("a.txt")
    Files.writeString(a, "alpha\nbeta\r\ngamma")
    // walk, following links, and a loop
    Files.createDirectories(root.resolve("t/u"))
    Files.writeString(root.resolve("t/u/f"), "f")
    Files.createSymbolicLink(root.resolve("t/link-to-u"), Paths.get("u"))
    show("walk")(tree(root.resolve("t")))
    show("walk following")(tree(root.resolve("t"), FileVisitOption.FOLLOW_LINKS))
    Files.createSymbolicLink(root.resolve("t/u/loop"), Paths.get(".."))
    show("walk a loop")(tree(root.resolve("t")))
    // The loop is met under whichever of `u` and `link-to-u` the system lists first.
    show("walk following a loop")(try tree(root.resolve("t"), FileVisitOption.FOLLOW_LINKS) catch case e: java.io.UncheckedIOException => all(e.getCause.getClass.getName, e.getCause.getMessage.endsWith("/loop")))
    // links
    val link = root.resolve("t/link-to-u")
    show("isSymbolicLink")(all(Files.isSymbolicLink(link), Files.isSymbolicLink(a)))
    show("readSymbolicLink")(Files.readSymbolicLink(link))
    show("readSymbolicLink of a file")(Files.readSymbolicLink(a))
    show("link again")(Files.createSymbolicLink(link, Paths.get("u")))
    val dangling = root.resolve("dangling")
    Files.createSymbolicLink(dangling, Paths.get("nowhere"))
    show("dangling exists")(all(Files.exists(dangling), Files.exists(dangling, LinkOption.NOFOLLOW_LINKS), Files.notExists(dangling)))
    show("dangling regular")(all(Files.isRegularFile(dangling), Files.isRegularFile(dangling, LinkOption.NOFOLLOW_LINKS)))
    show("directory through a link")(all(Files.isDirectory(link), Files.isDirectory(link, LinkOption.NOFOLLOW_LINKS)))
    // size and time
    show("size")(Files.size(a))
    show("size of absent")(Files.size(root.resolve("absent")))
    Files.setLastModifiedTime(a, FileTime.fromMillis(1262304000000L))
    show("modified")(Files.getLastModifiedTime(a).toMillis)
    show("modified absent")(Files.getLastModifiedTime(root.resolve("absent")))
    // copy
    val b = root.resolve("b.txt")
    show("copy")(Files.copy(a, b))
    show("copied")(all(Files.readString(b).length, Files.getLastModifiedTime(b).toMillis.equals(1262304000000L)))
    show("copy onto a file")(Files.copy(a, b))
    Files.writeString(b, "changed")
    show("copy replacing")(Files.copy(a, b, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.COPY_ATTRIBUTES))
    show("copied with attributes")(all(Files.readString(b).length, Files.getLastModifiedTime(b).toMillis))
    show("copy a directory")(Files.copy(root.resolve("t"), root.resolve("t2")))
    show("copied directory")(tree(root.resolve("t2")))
    show("copy absent")(Files.copy(root.resolve("absent"), root.resolve("c")))
    show("copy a link")(Files.copy(link, root.resolve("link-copy"), LinkOption.NOFOLLOW_LINKS))
    show("copied link")(all(Files.isSymbolicLink(root.resolve("link-copy")), Files.readSymbolicLink(root.resolve("link-copy"))))
    val sink = new java.io.ByteArrayOutputStream()
    show("copy to a stream")(all(Files.copy(a, sink), sink.size()))
    show("copy from a stream")(Files.copy(new java.io.ByteArrayInputStream("from a stream".getBytes), root.resolve("streamed.txt")))
    show("streamed")(Files.readString(root.resolve("streamed.txt")))
    show("copy from a stream onto a file")(Files.copy(new java.io.ByteArrayInputStream(Array[Byte](1)), root.resolve("streamed.txt")))
    // move
    show("move")(Files.move(b, root.resolve("moved.txt")))
    show("moved")(all(Files.exists(b), Files.exists(root.resolve("moved.txt"))))
    show("move onto a file")(Files.move(root.resolve("moved.txt"), a))
    show("move replacing")(Files.move(root.resolve("moved.txt"), root.resolve("streamed.txt"), StandardCopyOption.REPLACE_EXISTING))
    show("move atomically")(Files.move(root.resolve("streamed.txt"), root.resolve("atomic.txt"), StandardCopyOption.ATOMIC_MOVE))
    show("move a directory")(Files.move(root.resolve("t2"), root.resolve("t3")))
    show("move absent")(Files.move(root.resolve("absent"), root.resolve("x")))
    // delete
    show("delete")(Files.delete(root.resolve("atomic.txt")))
    show("delete absent")(Files.delete(root.resolve("atomic.txt")))
    show("delete a full directory")(Files.delete(root.resolve("t")))
    show("deleteIfExists")(all(Files.deleteIfExists(root.resolve("t3")), Files.deleteIfExists(root.resolve("t3"))))
    // temporary files
    val tmp = Files.createTempDirectory(root, "teq-case-")
    show("temp directory")(all(tmp.getFileName.toString.matches("teq-case-[0-9]+"), Files.isDirectory(tmp)))
    val tmpFile = Files.createTempFile(tmp, "f-", ".txt")
    show("temp file")(all(tmpFile.getFileName.toString.matches("f-[0-9]+\\.txt"), Files.size(tmpFile)))
    show("temp file default suffix")(Files.createTempFile(tmp, "g", null).getFileName.toString.endsWith(".tmp"))
    show("temp permissions")(all(PosixFilePermissions.toString(Files.getPosixFilePermissions(tmp)), PosixFilePermissions.toString(Files.getPosixFilePermissions(tmpFile))))
    val defaultTemp = Files.createTempDirectory("teq-case-")
    show("default temp")(defaultTemp.getParent == Paths.get(System.getProperty("java.io.tmpdir")))
    Files.delete(defaultTemp)
    remove(tmp)
    // directory streams
    val ds = Files.newDirectoryStream(root)
    show("directory stream")(try ds.iterator.asScala.map(_.getFileName.toString).toList.sorted.mkString(",") finally ds.close())
    val globbed = Files.newDirectoryStream(root, "*.{txt,md}")
    show("glob")(try globbed.asScala.map(_.getFileName.toString).toList.sorted.mkString(",") finally globbed.close())
    // lines, readers, writers
    val lines = Files.lines(a)
    show("lines")(try lines.iterator.asScala.mkString("|") finally lines.close())
    val reader = Files.newBufferedReader(a)
    show("reader")(try reader.readLine() + "/" + reader.readLine() finally reader.close())
    val w = Files.newBufferedWriter(root.resolve("w.txt"))
    w.write("one")
    w.newLine()
    w.write("twö")
    w.close()
    show("writer")(Files.readAllLines(root.resolve("w.txt")).asScala.mkString("|"))
    val appended = Files.newBufferedWriter(root.resolve("w.txt"), StandardOpenOption.APPEND)
    appended.write("\nthree")
    appended.close()
    show("appended")(Files.readString(root.resolve("w.txt")).replace("\n", "|"))
    show("write appending")(Files.write(root.resolve("w.txt"), "!".getBytes, StandardOpenOption.APPEND))
    show("written")(Files.readString(root.resolve("w.txt")).length)
    show("create new over a file")(Files.writeString(root.resolve("w.txt"), "x", StandardOpenOption.CREATE_NEW))
    show("write to absent without create")(Files.writeString(root.resolve("absent.txt"), "x", StandardOpenOption.WRITE))
    show("truncate")(Files.writeString(root.resolve("w.txt"), "short", StandardOpenOption.TRUNCATE_EXISTING))
    show("truncated")(Files.readString(root.resolve("w.txt")))
    show("write lines")(Files.write(root.resolve("l.txt"), java.util.List.of("x", "y")))
    show("lines written")(Files.readString(root.resolve("l.txt")).replace("\n", "|"))
    val out = Files.newOutputStream(root.resolve("o.bin"))
    out.write(Array[Byte](1, 2, 3))
    out.close()
    val in = Files.newInputStream(root.resolve("o.bin"))
    show("streams")(try in.readAllBytes().mkString(",") finally in.close())
    show("input of absent")(Files.newInputStream(root.resolve("absent")))
    show("readString charset")(Files.readString(root.resolve("w.txt"), StandardCharsets.UTF_8))
    // permissions
    val script = root.resolve("run.sh")
    Files.writeString(script, "#!/bin/sh\n")
    show("executable")(all(Files.isExecutable(script), Files.isReadable(script), Files.isWritable(script)))
    Files.setPosixFilePermissions(script, PosixFilePermissions.fromString("rwxr-x---"))
    show("permissions")(all(PosixFilePermissions.toString(Files.getPosixFilePermissions(script)), Files.isExecutable(script)))
    show("bad permissions")(PosixFilePermissions.fromString("rwz------"))
    show("readable absent")(Files.isReadable(root.resolve("absent")))
    // isSameFile
    show("same file")(all(Files.isSameFile(a, root.resolve("./a.txt")), Files.isSameFile(root.resolve("t/u"), root.resolve("t/link-to-u")), Files.isSameFile(a, script)))
    show("same file absent")(Files.isSameFile(a, root.resolve("absent")))
    // paths
    val p = Paths.get("target", "files_walk_copy", "t", "u")
    show("Paths.get")(p)
    show("names")(all(p.getNameCount, p.getName(1), p.subpath(1, 3), p.iterator.asScala.mkString("|")))
    show("bad name")(p.getName(4))
    show("endsWith")(all(p.endsWith("t/u"), p.endsWith("u"), p.endsWith("x/u"), p.endsWith("")))
    show("root")(all(p.getRoot, p.toAbsolutePath.getRoot))
    show("resolveSibling")(p.resolveSibling("v"))
    show("real path")(p.toRealPath())
    show("real path of a link")(root.resolve("t/link-to-u").toRealPath())
    show("real path not followed")(root.resolve("t/link-to-u").toRealPath(LinkOption.NOFOLLOW_LINKS))
    show("real path absent")(root.resolve("absent").toRealPath())
    show("toFile")(p.toFile.getPath)
    show("compare")(Paths.get("a").compareTo(Paths.get("b")) < 0)
    // java.io.File
    val f = new File("target/files_walk_copy", "a.txt")
    show("file")(all(f.getPath, f.getName, f.getParent, f.exists, f.isFile, f.isDirectory, f.length))
    show("file absolute")(all(f.getAbsolutePath, f.isAbsolute, new File("/x//y/").getPath))
    show("file list")(new File("target/files_walk_copy/t").list().toList.sorted.mkString(","))
    show("file mkdirs")(all(new File("target/files_walk_copy/m/n").mkdirs(), new File("target/files_walk_copy/m/n").mkdirs()))
    show("file delete")(all(new File("target/files_walk_copy/m/n").delete(), new File("target/files_walk_copy/m/n").exists))
    show("file separators")(all(File.separator, File.pathSeparator, File.separatorChar))
    show("file equality")(new File("a/b") == new File("a//b/") && new File("a").toPath == Paths.get("a"))
