//> using platform jvm
// os: unix
// `Files.walkFileTree` with a visitor: every event of a tree, a subtree skipped, the walk ended, a
// depth bound, a start that is not there, a loop met through a followed link; a file's basic
// attributes; a file's byte channel, its position, size and truncation, and streams over it
// (`Channels`); and a buffered output stream. The files are under target/ of the working directory,
// removed at the end; events whose order is the system's listing are sorted.
import java.io.{BufferedOutputStream, IOException}
import java.nio.channels.Channels
import java.nio.file.*
import java.nio.file.attribute.{BasicFileAttributes, FileTime}

object Main:
  val dir = System.getProperty("user.dir")
  def shown(text: String): String = if text == null then "null" else text.replace(dir, "<dir>")

  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + shown(String.valueOf(f)))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + shown(String.valueOf(e.getMessage)))

  // A visitor that records each event, answering `answer` for each.
  class Recorder(root: Path, answer: (String, Path) => FileVisitResult) extends FileVisitor[Path]:
    val events = scala.collection.mutable.ArrayBuffer.empty[String]
    private def name(p: Path): String = if p == root then "." else root.relativize(p).toString
    private def note(kind: String, p: Path): FileVisitResult =
      events += kind + " " + name(p)
      answer(kind, p)
    def preVisitDirectory(d: Path, attrs: BasicFileAttributes): FileVisitResult = note("pre", d)
    def visitFile(f: Path, attrs: BasicFileAttributes): FileVisitResult =
      note(if attrs.isDirectory then "dir" else if attrs.isSymbolicLink then "link" else "file", f)
    def visitFileFailed(f: Path, e: IOException): FileVisitResult = note("failed " + e.getClass.getSimpleName, f)
    def postVisitDirectory(d: Path, e: IOException): FileVisitResult = note("post", d)
    def sorted: String = events.sorted.mkString(", ")
    def before(a: String, b: String): Boolean = events.indexOf(a) >= 0 && events.indexOf(a) < events.indexOf(b)

  def options[A](xs: A*): java.util.Set[A] =
    val set = new java.util.HashSet[A]()
    xs.foreach(set.add)
    set

  def remove(p: Path): Unit =
    if Files.exists(p, LinkOption.NOFOLLOW_LINKS) then
      Files.walkFileTree(p, new SimpleFileVisitor[Path]:
        override def visitFile(f: Path, attrs: BasicFileAttributes): FileVisitResult =
          Files.delete(f)
          FileVisitResult.CONTINUE
        override def postVisitDirectory(d: Path, e: IOException): FileVisitResult =
          Files.delete(d)
          FileVisitResult.CONTINUE
      )

  def main(args: Array[String]): Unit =
    val root = Paths.get("target/files_tree_visitor")
    remove(root)
    Files.createDirectories(root)
    try run(root)
    finally remove(root)
    show("removed")(Files.exists(root))

  def run(root: Path): Unit =
    val t = root.resolve("t")
    Files.createDirectories(t.resolve("a/b"))
    Files.writeString(t.resolve("a/b/deep.txt"), "deep")
    Files.writeString(t.resolve("a/f.txt"), "f")
    Files.writeString(t.resolve("top.txt"), "top")
    val all = new Recorder(t, (_, _) => FileVisitResult.CONTINUE)
    show("walk")(Files.walkFileTree(t, all) == t)
    show("events")(all.sorted)
    show("order")(List(all.before("pre .", "pre a"), all.before("pre a", "pre a/b"), all.before("file a/b/deep.txt", "post a/b"), all.before("post a/b", "post a"), all.before("post a", "post .")).mkString(" "))
    val skip = new Recorder(t, (kind, p) => if kind == "pre" && p.endsWith("a") then FileVisitResult.SKIP_SUBTREE else FileVisitResult.CONTINUE)
    Files.walkFileTree(t, skip)
    show("skip subtree")(skip.sorted)
    val siblings = new Recorder(t, (kind, p) => if kind == "pre" && p.endsWith("b") then FileVisitResult.SKIP_SIBLINGS else FileVisitResult.CONTINUE)
    Files.walkFileTree(t.resolve("a"), siblings)
    show("skip siblings")(siblings.events.filter(e => !e.startsWith("file a/f") && !e.contains("f.txt")).mkString(", "))
    val end = new Recorder(t, (kind, _) => if kind == "file" then FileVisitResult.TERMINATE else FileVisitResult.CONTINUE)
    Files.walkFileTree(t, end)
    show("terminate")(end.events.count(_.startsWith("file")) + " " + end.events.count(_.startsWith("post")))
    val shallow = new Recorder(t, (_, _) => FileVisitResult.CONTINUE)
    Files.walkFileTree(t, new java.util.HashSet[FileVisitOption](), 1, shallow)
    show("depth 1")(shallow.sorted)
    val none = new Recorder(t, (_, _) => FileVisitResult.CONTINUE)
    Files.walkFileTree(t, new java.util.HashSet[FileVisitOption](), 0, none)
    show("depth 0")(none.sorted)
    show("negative depth")(Files.walkFileTree(t, new java.util.HashSet[FileVisitOption](), -1, none))
    val absent = new Recorder(root, (_, _) => FileVisitResult.CONTINUE)
    Files.walkFileTree(root.resolve("absent"), absent)
    show("absent")(absent.sorted)
    show("absent, simple")(Files.walkFileTree(root.resolve("absent"), new SimpleFileVisitor[Path] { override def toString = "simple" }))
    // links: not followed, a link; followed, its target; a link to an ancestor, a loop
    Files.createSymbolicLink(t.resolve("a/up"), Paths.get(".."))
    val links = new Recorder(t, (_, _) => FileVisitResult.CONTINUE)
    Files.walkFileTree(t.resolve("a"), links)
    show("link")(links.events.filter(_.contains("up")).mkString(", "))
    val loop = new Recorder(t, (_, _) => FileVisitResult.CONTINUE)
    Files.walkFileTree(t.resolve("a"), options(FileVisitOption.FOLLOW_LINKS), Int.MaxValue, loop)
    show("loop")(loop.events.filter(_.contains("up")).sorted.mkString(", "))
    Files.delete(t.resolve("a/up"))
    // attributes
    val top = t.resolve("top.txt")
    Files.setLastModifiedTime(top, FileTime.fromMillis(1577934245000L))
    val attrs = Files.readAttributes(top, classOf[BasicFileAttributes])
    show("attributes")(List(attrs.size, attrs.isRegularFile, attrs.isDirectory, attrs.isSymbolicLink, attrs.isOther, attrs.lastModifiedTime).mkString(" "))
    show("directory attributes")(Files.readAttributes(t, classOf[BasicFileAttributes]).isDirectory)
    Files.createSymbolicLink(t.resolve("link"), Paths.get("top.txt"))
    show("link attributes")(s"${Files.readAttributes(t.resolve("link"), classOf[BasicFileAttributes], LinkOption.NOFOLLOW_LINKS).isSymbolicLink} ${Files.readAttributes(t.resolve("link"), classOf[BasicFileAttributes]).isRegularFile}")
    show("absent attributes")(Files.readAttributes(root.resolve("absent"), classOf[BasicFileAttributes]))
    // streams over a byte channel
    val c = root.resolve("c.txt")
    val out = Channels.newOutputStream(Files.newByteChannel(c, options[OpenOption](StandardOpenOption.CREATE_NEW, StandardOpenOption.WRITE)))
    out.write("hello\n".getBytes)
    out.write('!')
    show("out range")(out.write(new Array[Byte](2), 1, 2))
    out.close()
    show("channel written")(Files.readString(c))
    val in = Channels.newInputStream(Files.newByteChannel(c))
    show("available")(in.available)
    show("read")(in.read())
    val buf = new Array[Byte](10)
    show("read some")(in.read(buf, 0, 10) + " " + new String(buf, 0, 6))
    show("read at the end")(in.read() + " " + in.read(buf, 0, 10) + " " + in.read(buf, 0, 0))
    in.close()
    // the channel's position, size and truncation
    val sc = Files.newByteChannel(c, options[OpenOption](StandardOpenOption.WRITE))
    sc.position(2)
    sc.write(java.nio.ByteBuffer.wrap("XY".getBytes))
    show("channel position")(s"${sc.position} ${sc.size}")
    sc.truncate(3)
    show("truncated")(s"${sc.position} ${sc.size}")
    sc.close()
    show("channel rewritten")(Files.readString(c))
    show("closed channel")(sc.position)
    show("read-only channel")(Files.newByteChannel(c).write(java.nio.ByteBuffer.wrap("Z".getBytes)))
    // a buffered stream
    val b = root.resolve("b.txt")
    val buffered = new BufferedOutputStream(Files.newOutputStream(b), 4)
    buffered.write("ab".getBytes)
    show("buffered")(Files.size(b))
    buffered.write("cde".getBytes)
    show("buffer full")(Files.readString(b))
    buffered.write("fghij".getBytes)
    show("past the buffer")(Files.readString(b))
    buffered.write('k')
    buffered.flush()
    show("flushed")(Files.readString(b))
    buffered.close()
    show("no buffer")(new BufferedOutputStream(Files.newOutputStream(b), 0))
