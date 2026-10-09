// The file system of the interpreter: what a macro reads at compile time, as under scalac (a class
// list next to the sources, say), and what a program under `teq interp` reads and writes (a generator
// of sources), through the builtins below (src/interp/files.rs), which throw the JDK's exceptions; a
// macro writes none, its expansions having no order, and a folded constant has none. JavaScript has
// no files.
package java.nio.file:

  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileExists(path: String): Boolean
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileIsDirectory(path: String): Boolean
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileText(path: String): String
  // The lines as `BufferedReader.readLine` ends them.
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileLines(path: String): Array[String]
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileBytes(path: String): Array[Byte]
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileModified(path: String): Long
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileWriteBytes(path: String, bytes: Array[Byte]): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileWrite(path: String, text: String): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileCreateDirectory(path: String): Unit
  // Nothing when the file is there (a link followed), its exception otherwise.
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileAccess(path: String): Unit
  // Whether the file is a directory, a link followed or not; its exception when it cannot be read.
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileDirectory(path: String, follow: Boolean): Boolean
  // The names of a directory's entries, in the order the system lists them.
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileEntries(path: String): Array[String]
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileDelete(path: String): Boolean
  // The path against the process's working directory (`user.dir`).
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def pathAbsolute(path: String): String
  // `normalize`, `relativize` and `startsWith` of the paths' texts, the JDK's `UnixPath` and
  // `WindowsPath` rules (src/interp/files.rs).
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def pathNormalize(path: String): String
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def pathRelativize(base: String, other: String): String
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def pathStartsWith(path: String, other: String): Boolean
  @js("null")
  def absentPath: Path
  // Whether paths are Windows' (the interpreter on Windows): the JDK's WindowsPath beside its
  // UnixPath, the rules src/interp/files.rs states for the natives: `\` and `/` both separate, a
  // root is a drive's (`C:\`, `C:`), a share's (`\\server\share\`) or `\`, a separator at the end
  // counts for nothing.
  @js("false")
  def windowsPaths: Boolean

  def pathSeparator(c: Char): Boolean = c == '/' || (c == '\\' && windowsPaths)

  def pathRoot(text: String): Int =
    def sep(i: Int): Boolean = i < text.length && pathSeparator(text.charAt(i))
    def after(from: Int): Int =
      var i = from
      while i < text.length && !sep(i) do i += 1
      i
    if !windowsPaths then (if sep(0) then 1 else 0)
    else if sep(0) && sep(1) then
      // The host and the share each past a run of separators, as the JDK's parser skips them.
      def name(from: Int): Int =
        var i = from
        while i < text.length && sep(i) do i += 1
        i
      val shareEnd = after(name(after(name(2))))
      if shareEnd < text.length then shareEnd + 1 else text.length
    else if text.length >= 2 && text.charAt(1) == ':' && ((text.charAt(0) >= 'A' && text.charAt(0) <= 'Z') || (text.charAt(0) >= 'a' && text.charAt(0) <= 'z')) then
      if sep(2) then 3 else 2
    else if sep(0) then 1
    else 0

  def pathTrimmed(text: String): String =
    val root = pathRoot(text)
    var end = text.length
    while end > root && pathSeparator(text.charAt(end - 1)) do end -= 1
    text.substring(0, end)

  // A path as the JDK's parsers write it: a Unix path with a run of `/` as one and none at the end
  // past the root (UnixPath); a Windows path (WindowsPathParser) with `\` for `/`, a share's root as
  // `\\server\share\`, a run of separators past the root as one and none at the end.
  def pathNormalized(text: String): String =
    if !windowsPaths then
      val out = new StringBuilder
      var i = 0
      while i < text.length do
        val c = text.charAt(i)
        if c != '/' || out.isEmpty || out.charAt(out.length - 1) != '/' then out.append(c)
        i += 1
      if out.length > 1 && out.charAt(out.length - 1) == '/' then out.setLength(out.length - 1)
      out.toString
    else
      val t = text.replace('/', '\\')
      val root = pathRoot(t)
      val out = new StringBuilder
      def segments(from: Int, until: Int): Unit =
        var i = from
        var sep = false
        var first = true
        while i < until do
          val c = t.charAt(i)
          if c == '\\' then sep = true
          else
            if sep && !first then out.append('\\')
            sep = false
            first = false
            out.append(c)
          i += 1
      if root >= 2 && t.charAt(0) == '\\' && t.charAt(1) == '\\' then
        out.append("\\\\")
        segments(2, root)
        out.append('\\')
      else out.append(t.substring(0, root))
      segments(root, t.length)
      out.toString

  def pathLastSeparator(text: String): Int =
    val root = pathRoot(text)
    var i = text.length - 1
    while i >= root && !pathSeparator(text.charAt(i)) do i -= 1
    if i >= root then i else -1

  final case class Path(text: String):
    def getParent: Path =
      val t = pathTrimmed(text)
      val root = pathRoot(t)
      val cut = pathLastSeparator(t)
      if cut >= 0 then new Path(t.substring(0, cut))
      else if root > 0 && t.length > root then new Path(t.substring(0, root))
      else absentPath
    def getFileName: Path =
      val t = pathTrimmed(text)
      val root = pathRoot(t)
      val cut = pathLastSeparator(t)
      if cut >= 0 then new Path(t.substring(cut + 1))
      else if root > 0 && root == t.length then absentPath
      else new Path(t.substring(root))
    def resolve(operand: String): Path =
      def ends(s: String): Boolean = s.nonEmpty && pathSeparator(s.charAt(s.length - 1))
      val base = pathNormalized(text)
      val other = pathNormalized(operand)
      if other.isEmpty then new Path(base)
      else if !windowsPaths then
        if pathRoot(other) > 0 || base.isEmpty then new Path(other)
        else if ends(base) then new Path(base + other)
        else new Path(base + "/" + other)
      else if new Path(other).isAbsolute then new Path(other)
      else
        val root = base.substring(0, pathRoot(base))
        if pathRoot(other) == 0 then
          val bare = base.isEmpty || (base.length == 2 && base.charAt(1) == ':')
          if ends(base) || bare then new Path(base + other) else new Path(base + "\\" + other)
        else if pathSeparator(other.charAt(0)) then
          if ends(root) then new Path(root + other.substring(1)) else new Path(root + other)
        else if !ends(root) || !root.substring(0, root.length - 1).equalsIgnoreCase(other.substring(0, 2)) then new Path(other)
        else if ends(base) then new Path(base + other.substring(2))
        else new Path(base + "\\" + other.substring(2))
    def resolve(other: Path): Path = resolve(other.toString)
    def isAbsolute: Boolean =
      val root = pathRoot(text)
      if !windowsPaths then root == 1
      else (root == 3 && text.charAt(1) == ':') || (root >= 2 && pathSeparator(text.charAt(0)) && pathSeparator(text.charAt(1)))
    // A relative path against the process's working directory, whether or not the file is there.
    def toAbsolutePath: Path = if isAbsolute then this else new Path(pathAbsolute(text))
    def normalize: Path = new Path(pathNormalize(text))
    def relativize(other: Path): Path = new Path(pathRelativize(text, other.toString))
    def startsWith(other: Path): Boolean = pathStartsWith(text, other.toString)
    def startsWith(other: String): Boolean = pathStartsWith(text, Path.of(other).toString)
    override def toString: String = text

  // On Windows a path is written as WindowsPath's parser writes it (pathNormalized).
  object Path:
    def of(first: String): Path = new Path(pathNormalized(first))

  object Paths:
    def get(first: String): Path = Path.of(first)

  // The members of the JDK's `Files` a generator of sources calls, with the JDK's answers for a path
  // that is not there, a directory and bytes that are not UTF-8: `NoSuchFileException`,
  // `IOException` and `MalformedInputException` from the readers, which read and write UTF-8.
  object Files:
    def exists(path: Path): Boolean = fileExists(path.toString)
    def isDirectory(path: Path): Boolean = fileIsDirectory(path.toString)
    def isRegularFile(path: Path): Boolean = fileExists(path.toString) && !fileIsDirectory(path.toString)
    def readString(path: Path): String = fileText(path.toString)
    def readAllLines(path: Path): java.util.List[String] = new java.util.ArrayList(fileLines(path.toString))
    def getLastModifiedTime(path: Path): attribute.FileTime = attribute.FileTime.fromMillis(fileModified(path.toString))
    def readAllBytes(path: Path): Array[Byte] = fileBytes(path.toString)
    // The text as UTF-8, the file created or truncated; an absent parent is not created.
    def writeString(path: Path, csq: CharSequence): Path =
      if csq == null then throw new NullPointerException()
      fileWrite(path.toString, csq.toString)
      path
    def write(path: Path, bytes: Array[Byte]): Path =
      if bytes == null then throw new NullPointerException()
      fileWriteBytes(path.toString, bytes)
      path
    def createDirectory(dir: Path): Path =
      fileCreateDirectory(dir.toString)
      dir
    // The JDK's: the directory made, or there; else, from the nearest ancestor of the absolute path
    // that is there, each name below it made in turn. A file in the place of the directory is its
    // `FileAlreadyExistsException`; the path is answered as given.
    def createDirectories(dir: Path): Path =
      val made =
        try
          createAndCheckIsDirectory(dir)
          true
        catch
          case x: FileAlreadyExistsException => throw x
          case _: java.io.IOException => false
      if !made then
        val absolute = dir.toAbsolutePath
        var parent = absolute.getParent
        var found = false
        while parent != null && !found do
          try
            fileAccess(parent.toString)
            found = true
          catch case _: NoSuchFileException => parent = parent.getParent
        if parent == null then throw new FileSystemException(absolute.toString, null, "Unable to determine if root directory exists")
        var child = parent
        val rest = parent.relativize(absolute).toString
        val names = if windowsPaths then rest.split("\\\\") else rest.split("/")
        var i = 0
        while i < names.length do
          child = child.resolve(names(i))
          createAndCheckIsDirectory(child)
          i += 1
      dir
    private def createAndCheckIsDirectory(dir: Path): Unit =
      try fileCreateDirectory(dir.toString)
      catch case x: FileAlreadyExistsException => if !isDirectory(dir) then throw x
    // A link itself, an empty directory or a file; false when there is none, a
    // `DirectoryNotEmptyException` for a directory with entries.
    def deleteIfExists(path: Path): Boolean = fileDelete(path.toString)
    // The entries of the directory, listed when the stream is made; the caller closes it.
    def list(dir: Path): java.util.stream.Stream[Path] =
      val entries = new DirectoryEntries(dir, fileEntries(dir.toString))
      java.util.stream.Pipes.iterated(entries).onClose(() => entries.close())
    // The JDK's `Files.walk` without options: the start, then depth first each directory's entries
    // in the order the system lists them, a directory listed when the walk reaches it; a link is an
    // entry, not followed. The caller closes the stream; the iterator fails once it is closed.
    def walk(start: Path): java.util.stream.Stream[Path] = walk(start, Int.MaxValue)
    def walk(start: Path, maxDepth: Int): java.util.stream.Stream[Path] =
      if maxDepth < 0 then throw new IllegalArgumentException("'maxDepth' is negative")
      val tree = new FileTree(start, maxDepth)
      java.util.stream.Pipes.iterated(tree).onClose(() => tree.close())

  // A directory stream's entries, none left once it is closed.
  private final class DirectoryEntries(dir: Path, names: Array[String]) extends java.util.Iterator[Path]:
    private var i = 0
    def hasNext: Boolean = i < names.length
    def next(): Path =
      if i >= names.length then throw new java.util.NoSuchElementException()
      i += 1
      dir.resolve(names(i - 1))
    def close(): Unit = i = names.length

  // The JDK's FileTreeWalker under FileTreeIterator: each step visits the next entry, a directory
  // opened (its entries listed) as it is visited unless it is at `maxDepth`; the start's failure is
  // the constructor's (`walk`'s own), a later one an `UncheckedIOException` at the step that meets it.
  private final class FileTree(start: Path, maxDepth: Int) extends java.util.Iterator[Path]:
    private var open = true
    private var dirs: List[Path] = Nil
    private var entries: List[Array[String]] = Nil
    private var at: List[Int] = Nil
    private var ahead: Path = visit(start)

    private def visit(entry: Path): Path =
      if fileDirectory(entry.toString, false) && dirs.length < maxDepth then
        val names = fileEntries(entry.toString)
        dirs = entry :: dirs
        entries = names :: entries
        at = 0 :: at
      entry

    private def advance(): Unit =
      while ahead == null && dirs.nonEmpty do
        val i = at.head
        if i >= entries.head.length then
          dirs = dirs.tail
          entries = entries.tail
          at = at.tail
        else
          at = (i + 1) :: at.tail
          val entry = dirs.head.resolve(entries.head(i))
          ahead =
            try visit(entry)
            catch case e: java.io.IOException => throw new java.io.UncheckedIOException(e)

    def hasNext: Boolean =
      if !open then throw new IllegalStateException()
      advance()
      ahead != null
    def next(): Path =
      if !open then throw new IllegalStateException()
      advance()
      if ahead == null then throw new java.util.NoSuchElementException()
      val out = ahead
      ahead = null
      out
    def close(): Unit =
      open = false
      dirs = Nil
      entries = Nil
      at = Nil

package java.nio.file.attribute:

  final case class FileTime(millis: Long):
    def toMillis: Long = millis
    def compareTo(other: FileTime): Int = if millis < other.millis then -1 else if millis > other.millis then 1 else 0
    override def toString: String = millis.toString

  object FileTime:
    def fromMillis(value: Long): FileTime = new FileTime(value)
