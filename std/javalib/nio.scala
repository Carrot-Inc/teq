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
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileKind(path: String, follow: Boolean): Int
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileSize(path: String): Long
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileSetModified(path: String, millis: Long): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileCopy(source: String, target: String, replace: Boolean, attributes: Boolean, follow: Boolean): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileMove(source: String, target: String, replace: Boolean, atomic: Boolean): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileReadLink(path: String): String
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileCreateLink(link: String, target: String): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileCreateHardLink(link: String, existing: String): Unit
  // A new directory (no suffix) or file under `dir`, its path.
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileTemp(dir: String, prefix: String, suffix: String): String
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileAccessible(path: String, mode: String): Boolean
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def filePermissions(path: String): Int
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileSetPermissions(path: String, mode: Int): Unit
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileKey(path: String, follow: Boolean): String
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def fileRealPath(path: String, follow: Boolean): String
  @js("$fail(\"UnsupportedOperationException\", \"the file system is not available on JavaScript\")")
  def pathNames(path: String): Array[String]
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

  final case class Path(text: String) extends Comparable[Path], java.lang.Iterable[Path]:
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
    def getRoot: Path =
      val root = pathRoot(text)
      if root == 0 then absentPath else new Path(text.substring(0, root))
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
    def resolveSibling(other: String): Path =
      val parent = getParent
      if parent == null then Path.of(other) else parent.resolve(other)
    def resolveSibling(other: Path): Path = resolveSibling(other.toString)
    def isAbsolute: Boolean =
      val root = pathRoot(text)
      if !windowsPaths then root == 1
      else (root == 3 && text.charAt(1) == ':') || (root >= 2 && pathSeparator(text.charAt(0)) && pathSeparator(text.charAt(1)))
    // A relative path against the process's working directory, whether or not the file is there.
    def toAbsolutePath: Path = if isAbsolute then this else new Path(pathAbsolute(text))
    // Absolute, its links resolved (the last one but with NOFOLLOW_LINKS), as the system spells it.
    def toRealPath(options: LinkOption*): Path = new Path(fileRealPath(text, !options.contains(LinkOption.NOFOLLOW_LINKS)))
    def normalize: Path = new Path(pathNormalize(text))
    def relativize(other: Path): Path = new Path(pathRelativize(text, other.toString))
    def startsWith(other: Path): Boolean = pathStartsWith(text, other.toString)
    def startsWith(other: String): Boolean = pathStartsWith(text, Path.of(other).toString)
    // The names past the root: the empty path has one, a root alone none.
    def getNameCount: Int = pathNames(text).length
    def getName(index: Int): Path =
      val names = pathNames(text)
      if index < 0 || index >= names.length then throw new IllegalArgumentException()
      new Path(names(index))
    def subpath(beginIndex: Int, endIndex: Int): Path =
      val names = pathNames(text)
      if beginIndex < 0 || beginIndex >= names.length || endIndex > names.length || beginIndex >= endIndex then throw new IllegalArgumentException()
      new Path(names.slice(beginIndex, endIndex).mkString(if windowsPaths then "\\" else "/"))
    // The JDK's: an absolute path ends with only itself; a relative one with the path's last names.
    def endsWith(other: Path): Boolean =
      val o = other.toString
      def same(a: String, b: String): Boolean = if windowsPaths then a.equalsIgnoreCase(b) else a == b
      if other.isAbsolute then same(pathTrimmed(text), pathTrimmed(o))
      else if o.isEmpty then text.isEmpty
      else
        val mine = pathNames(text)
        val theirs = pathNames(o)
        theirs.length <= mine.length && (if text.isEmpty then false else theirs.indices.forall(i => same(theirs(i), mine(mine.length - theirs.length + i))))
    def endsWith(other: String): Boolean = endsWith(Path.of(other))
    def iterator(): java.util.Iterator[Path] =
      val list = new java.util.ArrayList[Path]()
      pathNames(text).foreach(n => list.add(new Path(n)))
      list.iterator()
    def toFile: java.io.File = new java.io.File(text)
    // On Windows, `WindowsPath`'s order, equality and hash: the texts' units compared, two that
    // differ upper-cased (`Character.toUpperCase`), then the lengths; the hash of the units
    // upper-cased. On Unix the texts as they are, `UnixPath`'s for every ASCII path.
    def compareTo(other: Path): Int =
      val that = other.toString
      if !windowsPaths then text.compareTo(that)
      else
        val n = Math.min(text.length, that.length)
        var i = 0
        var r = 0
        while r == 0 && i < n do
          val (c1, c2) = (text.charAt(i), that.charAt(i))
          if c1 != c2 then r = Character.toUpperCase(c1) - Character.toUpperCase(c2)
          i += 1
        if r != 0 then r else text.length - that.length
    override def equals(that: Any): Boolean = that match
      case p: Path => if windowsPaths then compareTo(p) == 0 else text == p.toString
      case _ => false
    override def hashCode: Int =
      if !windowsPaths then text.hashCode
      else
        var h = 0
        var i = 0
        while i < text.length do
          h = 31 * h + Character.toUpperCase(text.charAt(i))
          i += 1
        h
    override def toString: String = text

  // On Windows a path is written as WindowsPath's parser writes it (pathNormalized).
  object Path:
    def of(first: String): Path = new Path(pathNormalized(first))
    // The names joined by the separator, the empty ones left out.
    def of(first: String, more: String*): Path =
      var text = first
      more.foreach { m =>
        if m.nonEmpty then text = if text.isEmpty then m else text + (if windowsPaths then "\\" else "/") + m
      }
      new Path(pathNormalized(text))

  object Paths:
    def get(first: String): Path = Path.of(first)
    def get(first: String, more: String*): Path = Path.of(first, more*)

  trait OpenOption
  trait CopyOption

  enum StandardOpenOption extends OpenOption:
    case READ, WRITE, APPEND, TRUNCATE_EXISTING, CREATE, CREATE_NEW, DELETE_ON_CLOSE, SPARSE, SYNC, DSYNC

  enum StandardCopyOption extends CopyOption:
    case REPLACE_EXISTING, COPY_ATTRIBUTES, ATOMIC_MOVE

  enum LinkOption extends OpenOption, CopyOption:
    case NOFOLLOW_LINKS

  enum FileVisitOption:
    case FOLLOW_LINKS

  // A directory's entries as `Files.newDirectoryStream` gives them, read once.
  trait DirectoryStream[T] extends java.lang.Iterable[T], java.io.Closeable

  private final class Entries(dir: Path, names: Array[String], accept: Path => Boolean) extends DirectoryStream[Path]:
    private var taken = false
    def iterator(): java.util.Iterator[Path] =
      if taken then throw new IllegalStateException("Iterator already obtained")
      taken = true
      val list = new java.util.ArrayList[Path]()
      names.foreach { n =>
        val p = dir.resolve(n)
        if accept(p) then list.add(p)
      }
      list.iterator()
    def close(): Unit = taken = true

  // A glob's pattern as the JDK's `Globs.toRegexPattern` makes it: `*` any run within a name,
  // `**` any run, `?` one character, `[...]` a class, `{a,b}` alternatives, `\\` an escape.
  private[file] def globRegex(glob: String): String =
    val sb = new StringBuilder("^")
    var i = 0
    var inGroup = false
    while i < glob.length do
      val c = glob.charAt(i)
      i += 1
      c match
        case '\\' if i < glob.length =>
          sb.append(java.util.regex.Pattern.quote(glob.charAt(i).toString))
          i += 1
        case '*' if i < glob.length && glob.charAt(i) == '*' =>
          sb.append(".*")
          i += 1
        case '*' => sb.append(if windowsPaths then "[^\\\\]*" else "[^/]*")
        case '?' => sb.append(if windowsPaths then "[^\\\\]" else "[^/]")
        case '[' =>
          sb.append('[')
          if i < glob.length && glob.charAt(i) == '!' then
            sb.append('^')
            i += 1
          while i < glob.length && glob.charAt(i) != ']' do
            val d = glob.charAt(i)
            if d == '\\' || d == '[' || d == '&' then sb.append('\\')
            sb.append(d)
            i += 1
          sb.append(']')
          i += 1
        case '{' =>
          sb.append("(?:(?:")
          inGroup = true
        case '}' if inGroup =>
          sb.append("))")
          inGroup = false
        case ',' if inGroup => sb.append(")|(?:")
        case _ => sb.append(java.util.regex.Pattern.quote(c.toString))
    sb.append('$')
    sb.toString

  // The members of the JDK's `Files` the repository's scripts call, with the JDK's answers for a
  // path that is not there, a directory and bytes that are not UTF-8: `NoSuchFileException`,
  // `IOException` and `MalformedInputException` from the readers, which read and write UTF-8. A
  // member a program's TASTy maps onto the JDK's with its Java varargs passed empty keeps the form
  // without them beside the one with options (src/tasty/write/std_inverse.txt).
  object Files:
    private def follows(options: Seq[Any]): Boolean = !options.contains(LinkOption.NOFOLLOW_LINKS)
    def exists(path: Path): Boolean = fileExists(path.toString)
    def exists(path: Path, options: LinkOption*): Boolean = fileKind(path.toString, follows(options)) >= 0
    def notExists(path: Path, options: LinkOption*): Boolean = !exists(path, options*)
    def isDirectory(path: Path): Boolean = fileIsDirectory(path.toString)
    def isDirectory(path: Path, options: LinkOption*): Boolean = fileKind(path.toString, follows(options)) == 1
    def isRegularFile(path: Path): Boolean = fileKind(path.toString, true) == 0
    def isRegularFile(path: Path, options: LinkOption*): Boolean = fileKind(path.toString, follows(options)) == 0
    def isSymbolicLink(path: Path): Boolean = fileKind(path.toString, false) == 2
    def isReadable(path: Path): Boolean = fileAccessible(path.toString, "r")
    def isWritable(path: Path): Boolean = fileAccessible(path.toString, "w")
    def isExecutable(path: Path): Boolean = fileAccessible(path.toString, "x")
    def readString(path: Path): String = fileText(path.toString)
    def readString(path: Path, cs: java.nio.charset.Charset): String =
      if cs.name() == "UTF-8" then fileText(path.toString) else new String(fileBytes(path.toString), cs)
    def readAllLines(path: Path): java.util.List[String] = new java.util.ArrayList(fileLines(path.toString))
    // The file's lines, read when the stream is made; the caller closes it.
    def lines(path: Path): java.util.stream.Stream[String] = new java.util.ArrayList(fileLines(path.toString)).stream()
    def getLastModifiedTime(path: Path): attribute.FileTime = attribute.FileTime.fromMillis(fileModified(path.toString))
    def getLastModifiedTime(path: Path, options: LinkOption*): attribute.FileTime = attribute.FileTime.fromMillis(fileModified(path.toString))
    def setLastModifiedTime(path: Path, time: attribute.FileTime): Path =
      if time == null then throw new NullPointerException()
      fileSetModified(path.toString, time.toMillis)
      path
    def readAllBytes(path: Path): Array[Byte] = fileBytes(path.toString)
    def size(path: Path): Long = fileSize(path.toString)
    // The options of a write as `Files.newOutputStream` takes them (the JDK's provider): none is
    // CREATE, TRUNCATE_EXISTING and WRITE, else those given and WRITE; READ refused, then APPEND
    // with TRUNCATE_EXISTING (`UnixChannelFactory`); each the flag `streamOpenWrite` reads: CREATE 1,
    // CREATE_NEW 2, APPEND 4, TRUNCATE_EXISTING 8.
    private def openFlags(options: Seq[OpenOption]): Int =
      if options.isEmpty then 1 | 8
      else
        if options.contains(null) then throw new NullPointerException()
        if options.contains(StandardOpenOption.READ) then throw new IllegalArgumentException("READ not allowed")
        val append = options.contains(StandardOpenOption.APPEND)
        val truncate = options.contains(StandardOpenOption.TRUNCATE_EXISTING)
        if append && truncate then throw new IllegalArgumentException("APPEND + TRUNCATE_EXISTING not allowed")
        (if options.contains(StandardOpenOption.CREATE) then 1 else 0) | (if options.contains(StandardOpenOption.CREATE_NEW) then 2 else 0) |
          (if append then 4 else 0) | (if truncate then 8 else 0)
    // The text as UTF-8, the file created or truncated; an absent parent is not created.
    def writeString(path: Path, csq: CharSequence): Path = writeString(path, csq, Seq.empty[OpenOption]*)
    def writeString(path: Path, csq: CharSequence, options: OpenOption*): Path =
      if csq == null then throw new NullPointerException()
      if options.isEmpty then fileWrite(path.toString, csq.toString)
      else
        val out = newOutputStream(path, options*)
        try out.write(csq.toString.getBytes(java.nio.charset.StandardCharsets.UTF_8))
        finally out.close()
      path
    def write(path: Path, bytes: Array[Byte]): Path = write(path, bytes, Seq.empty[OpenOption]*)
    def write(path: Path, bytes: Array[Byte], options: OpenOption*): Path =
      if bytes == null then throw new NullPointerException()
      if options.isEmpty then fileWriteBytes(path.toString, bytes)
      else
        val out = newOutputStream(path, options*)
        try out.write(bytes)
        finally out.close()
      path
    // The lines, each ended by the system's line separator.
    def write(path: Path, lines: java.lang.Iterable[? <: CharSequence], options: OpenOption*): Path =
      val sb = new java.lang.StringBuilder()
      val it = lines.iterator()
      while it.hasNext do sb.append(it.next()).append(System.lineSeparator())
      writeString(path, sb.toString, options*)
    def newInputStream(path: Path, options: OpenOption*): java.io.InputStream = new java.io.NativeInputStream(java.io.streamOpenRead(path.toString, false))
    def newOutputStream(path: Path, options: OpenOption*): java.io.OutputStream =
      new java.io.NativeOutputStream(java.io.streamOpenWrite(path.toString, openFlags(options), false))
    // A reader whose decoder reports malformed input (`MalformedInputException`), as the JDK's
    // `newBufferedReader` makes it, where `InputStreamReader` replaces it.
    def newBufferedReader(path: Path): java.io.BufferedReader = newBufferedReader(path, java.nio.charset.StandardCharsets.UTF_8)
    def newBufferedReader(path: Path, cs: java.nio.charset.Charset): java.io.BufferedReader =
      new java.io.BufferedReader(new java.io.InputStreamReader(newInputStream(path), cs).reporting())
    def newBufferedWriter(path: Path, options: OpenOption*): java.io.BufferedWriter =
      new java.io.BufferedWriter(new java.io.OutputStreamWriter(newOutputStream(path, options*)))
    def newBufferedWriter(path: Path, cs: java.nio.charset.Charset, options: OpenOption*): java.io.BufferedWriter =
      new java.io.BufferedWriter(new java.io.OutputStreamWriter(newOutputStream(path, options*), cs))
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
    // `prefix` and a random number under the directory (the temporary directory, `java.io.tmpdir`,
    // when none is given), for the program alone.
    def createTempDirectory(prefix: String): Path = createTempDirectory(Path.of(System.getProperty("java.io.tmpdir")), prefix)
    def createTempDirectory(dir: Path, prefix: String): Path = Path.of(fileTemp(dir.toString, if prefix == null then "" else prefix, null))
    def createTempFile(prefix: String, suffix: String): Path = createTempFile(Path.of(System.getProperty("java.io.tmpdir")), prefix, suffix)
    def createTempFile(dir: Path, prefix: String, suffix: String): Path =
      Path.of(fileTemp(dir.toString, if prefix == null then "" else prefix, if suffix == null then ".tmp" else suffix))
    // A link itself, an empty directory or a file; `NoSuchFileException` when there is none, a
    // `DirectoryNotEmptyException` for a directory with entries.
    def delete(path: Path): Unit = if !fileDelete(path.toString) then throw new NoSuchFileException(path.toString)
    def deleteIfExists(path: Path): Boolean = fileDelete(path.toString)
    // One file, or an empty directory for a directory, as the JDK copies it.
    def copy(source: Path, target: Path, options: CopyOption*): Path =
      fileCopy(source.toString, target.toString, options.contains(StandardCopyOption.REPLACE_EXISTING), options.contains(StandardCopyOption.COPY_ATTRIBUTES), !options.contains(LinkOption.NOFOLLOW_LINKS))
      target
    // What the stream holds, into a new file (or one replaced with REPLACE_EXISTING): its length.
    def copy(in: java.io.InputStream, target: Path, options: CopyOption*): Long =
      val replace = options.contains(StandardCopyOption.REPLACE_EXISTING)
      if !replace && exists(target, LinkOption.NOFOLLOW_LINKS) then throw new FileAlreadyExistsException(target.toString)
      if replace then deleteIfExists(target)
      val out = new java.io.NativeOutputStream(java.io.streamOpenWrite(target.toString, 2, false))
      try in.transferTo(out)
      finally out.close()
    def copy(source: Path, out: java.io.OutputStream): Long =
      val in = newInputStream(source)
      try in.transferTo(out)
      finally in.close()
    def move(source: Path, target: Path, options: CopyOption*): Path =
      fileMove(source.toString, target.toString, options.contains(StandardCopyOption.REPLACE_EXISTING), options.contains(StandardCopyOption.ATOMIC_MOVE))
      target
    def readSymbolicLink(link: Path): Path = Path.of(fileReadLink(link.toString))
    def createSymbolicLink(link: Path, target: Path, attrs: attribute.FileAttribute[?]*): Path =
      fileCreateLink(link.toString, target.toString)
      link
    def createLink(link: Path, existing: Path): Path =
      fileCreateHardLink(link.toString, existing.toString)
      link
    // Two paths that are equal, or name one file (both followed to it).
    def isSameFile(path: Path, path2: Path): Boolean =
      path == path2 || fileKey(path.toString, true) == fileKey(path2.toString, true)
    def getPosixFilePermissions(path: Path, options: LinkOption*): java.util.Set[attribute.PosixFilePermission] =
      attribute.PosixFilePermissions.fromMode(filePermissions(path.toString))
    def setPosixFilePermissions(path: Path, perms: java.util.Set[attribute.PosixFilePermission]): Path =
      fileSetPermissions(path.toString, attribute.PosixFilePermissions.toMode(perms))
      path
    def newDirectoryStream(dir: Path): DirectoryStream[Path] = new Entries(dir, fileEntries(dir.toString), _ => true)
    // The entries whose names match the glob.
    def newDirectoryStream(dir: Path, glob: String): DirectoryStream[Path] =
      val pattern = java.util.regex.Pattern.compile(globRegex(glob))
      new Entries(dir, fileEntries(dir.toString), p => pattern.matcher(p.getFileName.toString).matches())
    // The entries of the directory, listed when the stream is made; the caller closes it.
    def list(dir: Path): java.util.stream.Stream[Path] =
      val entries = new DirectoryEntries(dir, fileEntries(dir.toString))
      java.util.stream.Pipes.iterated(entries).onClose(() => entries.close())
    // The JDK's `Files.walk`: the start, then depth first each directory's entries in the order the
    // system lists them, a directory listed when the walk reaches it; a link is an entry, not
    // followed, unless FOLLOW_LINKS, with which a link back to a directory the walk is in fails
    // with `FileSystemLoopException` in an `UncheckedIOException`. The caller closes the stream;
    // the iterator fails once it is closed.
    def walk(start: Path): java.util.stream.Stream[Path] = walk(start, Int.MaxValue, Seq.empty[FileVisitOption]*)
    def walk(start: Path, options: FileVisitOption*): java.util.stream.Stream[Path] = walk(start, Int.MaxValue, options*)
    def walk(start: Path, maxDepth: Int): java.util.stream.Stream[Path] = walk(start, maxDepth, Seq.empty[FileVisitOption]*)
    def walk(start: Path, maxDepth: Int, options: FileVisitOption*): java.util.stream.Stream[Path] =
      if maxDepth < 0 then throw new IllegalArgumentException("'maxDepth' is negative")
      val tree = new FileTree(start, maxDepth, options.contains(FileVisitOption.FOLLOW_LINKS))
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
  private final class FileTree(start: Path, maxDepth: Int, follow: Boolean) extends java.util.Iterator[Path]:
    private var open = true
    private var dirs: List[Path] = Nil
    private var keys: List[String] = Nil
    private var entries: List[Array[String]] = Nil
    private var at: List[Int] = Nil
    private var ahead: Path = visit(start)

    private def visit(entry: Path): Path =
      // A link that names nothing is an entry of its own when followed too.
      val directory =
        if follow then
          try fileDirectory(entry.toString, true)
          catch case _: NoSuchFileException => false
        else fileDirectory(entry.toString, false)
      if directory && dirs.length < maxDepth then
        val key = if follow then fileKey(entry.toString, true) else null
        if follow && keys.contains(key) then throw new FileSystemLoopException(entry.toString)
        val names = fileEntries(entry.toString)
        dirs = entry :: dirs
        keys = key :: keys
        entries = names :: entries
        at = 0 :: at
      entry

    private def advance(): Unit =
      while ahead == null && dirs.nonEmpty do
        val i = at.head
        if i >= entries.head.length then
          dirs = dirs.tail
          keys = keys.tail
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
      keys = Nil
      entries = Nil
      at = Nil

package java.nio.file.attribute:

  // The JDK's `FileTime`: a value with its unit, converted when asked (`to`, `toMillis`, saturated as
  // `TimeUnit` saturates), so that a time keeps the precision it was given (a zip entry's
  // microseconds); two times of one unit compared by their values, of two units by the instants
  // they stand for (`toInstant`'s epoch second and nanosecond, held at `Instant`'s bounds, then their
  // days and the nanoseconds past them). Its text is `toString`'s ISO instant. A time from an
  // `Instant` is not here.
  final class FileTime private (private val value: Long, private val unit: java.util.concurrent.TimeUnit) extends Comparable[FileTime]:
    def to(unit: java.util.concurrent.TimeUnit): Long =
      if unit == null then throw new NullPointerException("unit")
      unit.convert(value, this.unit)
    def toMillis: Long = unit.toMillis(value)

    // `toInstant`'s epoch second and nanosecond.
    private def instant: Array[Long] =
      def scale(d: Long, m: Long): Long = if d > Long.MaxValue / m then Long.MaxValue else if d < -(Long.MaxValue / m) then Long.MinValue else d * m
      val (secs, nanos) = unit.ordinal() match
        case 6 => (scale(value, 86400L), 0L)
        case 5 => (scale(value, 3600L), 0L)
        case 4 => (scale(value, 60L), 0L)
        case 3 => (value, 0L)
        case 2 => (Math.floorDiv(value, 1000L), Math.floorMod(value, 1000L) * 1000000L)
        case 1 => (Math.floorDiv(value, 1000000L), Math.floorMod(value, 1000000L) * 1000L)
        case _ => (Math.floorDiv(value, 1000000000L), Math.floorMod(value, 1000000000L))
      if secs <= FileTime.MinSecond then Array(FileTime.MinSecond, 0L)
      else if secs >= FileTime.MaxSecond then Array(FileTime.MaxSecond, 999999999L)
      else Array(secs, nanos)

    private def toDays: Long = unit.toDays(value)
    private def excessNanos(days: Long): Long = unit.toNanos(value - unit.convert(days, java.util.concurrent.TimeUnit.DAYS))

    def compareTo(other: FileTime): Int =
      if unit == other.unit then java.lang.Long.compare(value, other.value)
      else
        val (a, b) = (instant, other.instant)
        val bySecond = java.lang.Long.compare(a(0), b(0))
        val cmp = if bySecond != 0 then bySecond else java.lang.Long.compare(a(1), b(1))
        if cmp != 0 || (a(0) != FileTime.MaxSecond && a(0) != FileTime.MinSecond) then cmp
        else
          val (days, daysOther) = (toDays, other.toDays)
          if days == daysOther then java.lang.Long.compare(excessNanos(days), other.excessNanos(daysOther))
          else java.lang.Long.compare(days, daysOther)
    override def equals(that: Any): Boolean = that match
      case other: FileTime => compareTo(other) == 0
      case _ => false
    // `Instant.hashCode` of the instant.
    override def hashCode: Int =
      val i = instant
      (i(0) ^ (i(0) >>> 32)).toInt + 51 * i(1).toInt

    // The instant in ISO 8601 at UTC, its year past 9999 or before 0000 in full and its fraction
    // without trailing zeros, as the JDK writes it.
    override def toString: String =
      val (secs, nanos) =
        if unit.compareTo(java.util.concurrent.TimeUnit.SECONDS) >= 0 then (unit.toSeconds(value), 0L)
        else
          val i = instant
          (i(0), i(1))
      val per10000 = FileTime.SecondsPer10000Years
      val (hi, lo) =
        if secs >= -FileTime.Seconds0000To1970 then
          val zero = secs - per10000 + FileTime.Seconds0000To1970
          (Math.floorDiv(zero, per10000) + 1, Math.floorMod(zero, per10000))
        else
          val zero = secs + FileTime.Seconds0000To1970
          (zero / per10000, zero % per10000)
      // `LocalDateTime.ofEpochSecond` at UTC of what is left, its civil date from the epoch day.
      val epochSecond = lo - FileTime.Seconds0000To1970
      val day = Math.floorDiv(epochSecond, 86400L)
      val secOfDay = Math.floorMod(epochSecond, 86400L).toInt
      val z = day + 719468L
      val era = Math.floorDiv(z, 146097L)
      val doe = z - era * 146097L
      val yoe = (doe - doe / 1460L + doe / 36524L - doe / 146096L) / 365L
      val doy = doe - (365L * yoe + yoe / 4L - yoe / 100L)
      val mp = (5L * doy + 2L) / 153L
      val dayOfMonth = (doy - (153L * mp + 2L) / 5L + 1L).toInt
      val month = (if mp < 10L then mp + 3L else mp - 9L).toInt
      val civilYear = (yoe + era * 400L + (if month <= 2 then 1L else 0L)).toInt
      var year = civilYear + hi.toInt * 10000
      if year <= 0 then year = year - 1
      val sb = new java.lang.StringBuilder(64)
      def append(width: Int, digits: Int): Unit =
        var (w, d) = (width, digits)
        while w > 0 do
          sb.append((d / w + '0').toChar)
          d = d % w
          w /= 10
      if year < 0 then sb.append('-')
      year = Math.abs(year)
      if year < 10000 then append(1000, year) else sb.append(String.valueOf(year))
      sb.append('-')
      append(10, month)
      sb.append('-')
      append(10, dayOfMonth)
      sb.append('T')
      append(10, secOfDay / 3600)
      sb.append(':')
      append(10, secOfDay / 60 % 60)
      sb.append(':')
      append(10, secOfDay % 60)
      var fraction = nanos.toInt
      if fraction != 0 then
        sb.append('.')
        var w = 100000000
        while fraction % 10 == 0 do
          fraction /= 10
          w /= 10
        append(w, fraction)
      sb.append('Z')
      sb.toString

  object FileTime:
    private[attribute] final val MinSecond = -31557014167219200L
    private[attribute] final val MaxSecond = 31556889864403199L
    private[attribute] final val SecondsPer10000Years = 146097L * 25L * 86400L
    private[attribute] final val Seconds0000To1970 = ((146097L * 5L) - (30L * 365L + 7L)) * 86400L
    def from(value: Long, unit: java.util.concurrent.TimeUnit): FileTime =
      if unit == null then throw new NullPointerException("unit")
      new FileTime(value, unit)
    def fromMillis(value: Long): FileTime = new FileTime(value, java.util.concurrent.TimeUnit.MILLISECONDS)

  trait FileAttribute[T]:
    def name(): String
    def value(): T

  enum PosixFilePermission:
    case OWNER_READ, OWNER_WRITE, OWNER_EXECUTE, GROUP_READ, GROUP_WRITE, GROUP_EXECUTE, OTHERS_READ, OTHERS_WRITE, OTHERS_EXECUTE

  // `rwxr-x---` and the sets of permissions, in the JDK's order of the bits.
  object PosixFilePermissions:
    private val letters = "rwxrwxrwx"
    def toString(perms: java.util.Set[PosixFilePermission]): String =
      val sb = new java.lang.StringBuilder()
      PosixFilePermission.values.foreach(p => sb.append(if perms.contains(p) then letters.charAt(p.ordinal) else '-'))
      sb.toString
    def fromString(perms: String): java.util.Set[PosixFilePermission] =
      if perms.length != 9 then throw new IllegalArgumentException("Invalid mode")
      val set = new java.util.HashSet[PosixFilePermission]()
      PosixFilePermission.values.foreach { p =>
        val c = perms.charAt(p.ordinal)
        if c == letters.charAt(p.ordinal) then set.add(p)
        else if c != '-' then throw new IllegalArgumentException("Invalid mode")
      }
      set
    private[file] def fromMode(mode: Int): java.util.Set[PosixFilePermission] =
      val set = new java.util.HashSet[PosixFilePermission]()
      PosixFilePermission.values.foreach(p => if (mode & (256 >> p.ordinal)) != 0 then set.add(p))
      set
    private[file] def toMode(perms: java.util.Set[PosixFilePermission]): Int =
      var mode = 0
      PosixFilePermission.values.foreach(p => if perms.contains(p) then mode |= 256 >> p.ordinal)
      mode
