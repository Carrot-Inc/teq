//> using dep org.scala-native::tools:0.5.12
//> using javaOpt --add-opens=jdk.zipfs/jdk.nio.zipfs=ALL-UNNAMED
// The JVM's answers of tests/native-build/jar-names.expected: each jar of the table below written as a stored zip,
// then read by Scala Native 0.5.12's own build, `NativeLib.findNativeLibs` (every jar of the class path opened as a
// java.util.zip.ZipFile, taken as a library where an entry's name is a native file's) and `unpackNativeCode` (the jar
// copied out through the JDK's zip file system). The table: the cut check's eighteen jars, then the product of a
// varied entry's name (each kind of piece, encoding and character a name's reading turns on), its place (under
// scala-native/, outside it, under /scala-native/), its position among the entries of a library (first, between, last)
// and, where the name is not ASCII, the entry's UTF-8 flag; the same names alone beside a descriptor (whether the jar
// is a library turns on them); files and directories of one name, in either order; entries of one name; comments. A
// line: the case, the jar's SHA-256, what the build made of it (`refused` and the exception; `none`, no library; or
// `files` and each file unpacked under scala-native/ with its SHA-256, in the names' order, after `clash` where the zip
// file system reads an entry as a file and a directory both, keeping one and leaving out the other; after `classpath`
// where the jar is of a shape teq's class path read refuses, the read the link shares (a stub before the jar with the
// jar's own offsets, a zip64 end record whose plain fields are its marks); after `teqread` where that read takes
// another central directory than the JDK finds, or counts the entries' offsets from another place), then `|` and the
// entries (flag, external attributes, name, comment, data, method, extra field, the compressed size, size, offset and
// CRC-32 the central header gives where not the entry's own, disk number, the local header's name where another;
// bytes outside printable ASCII and `\,;|= ` written `\xHH`, numbers in hexadecimal), then `|` and the bytes after
// the central directory's last entry; or, of a jar written byte by byte (`rawCases`: where the directory is found,
// the copy, and a deflate stream reaching each of zlib's refusals beside its neighbour, `zlibFamily`), `| raw |` and
// its bytes in hexadecimal. It also writes tests/native-build/inflate-flips.expected beside it (`flips`). It calls
// Scala Native's own classes, private to its package (hence the package; `unpackNativeCode`, a
// private method, by reflection), so it runs on a JVM, never under teq, from the repository's root:
//   LC_ALL=C.UTF-8 scala-cli run --scala 3.3.6 tests/native-build/jar-names-oracle.scala -- tests/native-build/jar-names.expected <tools>
// <tools> the directory tests/native-build/tool-jars.py wrote (the real tools' jars, judged and written as `tool` lines);
// `-- --judge <jar>...` prints what the build makes of each jar named, a published library's where it lies,
// under a UTF-8 locale, as the JDK writes a name to the file system in the locale's encoding (`sun.jnu.encoding`; under
// the C locale it cannot write a name that is not ASCII, and Scala Native's build fails on it). It works under
// out/jar-names and leaves it there.
package scala.scalanative.build

import java.io.ByteArrayOutputStream
import java.nio.file.{Files, Path, Paths}
import scala.jdk.CollectionConverters._

object JarNamesOracle {
  /// An entry: its name, data, flag, comment and external attributes; the method its headers name (its data stored
  /// whatever it names), its central header's extra field, the compressed size, size, offset and CRC-32 that header
  /// gives where they are not the entry's own (`-1`), its disk number, and the name its local header gives (that of the
  /// central header where empty).
  final case class E(name: Array[Byte], data: Array[Byte], flag: Int = 0, comment: Array[Byte] = Array.emptyByteArray, attr: Int = 0,
      method: Int = 0, extra: Array[Byte] = Array.emptyByteArray, csize: Long = -1, size: Long = -1, offset: Long = -1, crc: Long = -1,
      disk: Int = 0, local: Array[Byte] = Array.emptyByteArray)

  /// Each character a byte, as the table writes names.
  def b(s: String): Array[Byte] = s.getBytes("ISO-8859-1")

  def escaped(bytes: Array[Byte]): String =
    bytes.map { x =>
      val c = x & 0xff
      if (c > 0x20 && c < 0x7f && !"\\,;|=".contains(c.toChar)) c.toChar.toString else f"\\x$c%02X"
    }.mkString

  /// An exception's message on one line of printable ASCII, every other character `\uXXXX`.
  def said(message: String): String =
    String.valueOf(message).flatMap(c => if (c >= 0x20 && c < 0x7f) c.toString else f"\\u${c.toInt}%04X")

  def sha256(bytes: Array[Byte]): String =
    java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))

  /// A stored zip: each entry's local header and data, the central directory and the bytes `trailing` after its last
  /// entry, its end; the DOS date 1980-01-01.
  def jarBytes(es: Seq[E], trailing: Array[Byte] = Array.emptyByteArray): Array[Byte] = {
    val out = new ByteArrayOutputStream
    def u16(v: Int): Unit = { out.write(v & 0xff); out.write((v >> 8) & 0xff) }
    def u32(v: Long): Unit = { u16((v & 0xffff).toInt); u16(((v >> 16) & 0xffff).toInt) }
    val written = es.map { e =>
      val at = out.size
      val crc = new java.util.zip.CRC32
      crc.update(e.data)
      val local = if (e.local.isEmpty) e.name else e.local
      u32(0x04034b50L); u16(20); u16(e.flag); u16(e.method); u16(0); u16(0x21); u32(crc.getValue)
      u32(e.data.length.toLong); u32(e.data.length.toLong); u16(local.length); u16(0)
      out.write(local); out.write(e.data)
      (at, crc.getValue)
    }
    val directory = out.size
    for ((e, (at, crc)) <- es.zip(written)) {
      def or(set: Long, own: Long): Long = if (set >= 0) set else own
      u32(0x02014b50L); u16(0x0314); u16(20); u16(e.flag); u16(e.method); u16(0); u16(0x21); u32(or(e.crc, crc))
      u32(or(e.csize, e.data.length.toLong)); u32(or(e.size, e.data.length.toLong)); u16(e.name.length); u16(e.extra.length); u16(e.comment.length)
      u16(e.disk); u16(0); u32(e.attr.toLong & 0xffffffffL); u32(or(e.offset, at.toLong))
      out.write(e.name); out.write(e.extra); out.write(e.comment)
    }
    out.write(trailing)
    val size = out.size - directory
    u32(0x06054b50L); u16(0); u16(0); u16(es.length); u16(es.length); u32(size.toLong); u32(directory.toLong); u16(0)
    out.toByteArray
  }

  val descriptor: Array[Byte] = b("compile.c.options = -DJK=1\n")

  /// An entry's data: of a name that ends as a source does, a source of a function of its own (`jn<case>_<entry>`), so
  /// that every library of the table links into one program; of the descriptor, the descriptor; else a header line.
  def data(k: Int, i: Int, name: Array[Byte]): Array[Byte] = {
    val n = new String(name, "ISO-8859-1")
    if (n.endsWith("/")) Array.emptyByteArray
    else if (n.endsWith("scala-native.properties")) descriptor
    else if (n.endsWith(".cpp")) b(s"""extern "C" int jn${k}_$i(void) { return $i; }\n""")
    else if (n.endsWith(".c")) b(s"int jn${k}_$i(void) { return $i; }\n")
    else if (n.endsWith(".S")) b(s".globl jn${k}_$i\njn${k}_$i:\n    ret\n    .section .note.GNU-stack,\"\",@progbits\n")
    else b(s"#define JN${k}_$i $i\n")
  }

  val kinds: Seq[(String, String)] = Seq(
    "plain" -> "x.c", "dot" -> "./x.c", "dotdot" -> "../x.c", "dot-inner" -> "a/./x.c", "dotdot-inner" -> "a/../x.c",
    "dot-last" -> "x/.", "dotdot-last" -> "x/..", "doubled" -> "a//x.c", "dir" -> "d.c/", "dir-doubled" -> "e.c//",
    "nul" -> "a\u0000b.c", "long" -> ("L" * 300 + ".c"), "latin1" -> "m\u00e9thode.c", "utf8" -> "m\u00c3\u00a9thode.c",
    "overlong" -> "\u00c0\u00af.c", "surrogate" -> "\u00ed\u00a0\u0080.c", "beyond" -> "\u00f4\u0090\u0080\u0080.c",
    "cut" -> "\u00e2\u0082.c", "backslash" -> "in\\side.c", "newline" -> "a\nb.c", "nel" -> "a\u00c2\u0085b.c",
    "ls" -> "a\u00e2\u0080\u00a8b.c", "hidden" -> ".hidden.c", "dots3" -> ".../x.c", "trailing-dot" -> "t.c.",
    "dot-dir" -> "d./u.c", "empty-stem" -> ".c", "upper" -> "X.C", "cpp" -> "y.cpp", "asm" -> "z.S", "header" -> "h.h",
    "blank" -> "a b.c", "tab" -> "a\tb.c")
  val notAscii: Set[String] = Set("latin1", "utf8", "overlong", "surrogate", "beyond", "cut", "nel", "ls")
  val places: Seq[(String, String)] = Seq("in" -> "scala-native/", "out" -> "include/", "abs" -> "/scala-native/")

  /// The cases: a name and the entries, each entry's data made by `data` unless given.
  def cases: Seq[(String, Seq[(Array[Byte], Int, Array[Byte], Int, Option[Array[Byte]])])] = {
    type Row = (Array[Byte], Int, Array[Byte], Int, Option[Array[Byte]])
    def plain(name: String, flag: Int = 0): Row = (b(name), flag, Array.emptyByteArray, 0, None)
    def bodied(name: String, body: String, attr: Int = 0): Row = (b(name), 0, Array.emptyByteArray, attr, Some(b(body)))
    val desc = plain("scala-native/scala-native.properties")
    val jk = plain("scala-native/jk.c")
    val base = Seq(desc, jk)
    // The cut check's eighteen (owner-shared/reviews/cut-native-build/mkjars.py and mkjars2.py), their entries' data
    // the table's.
    val review: Seq[(String, Seq[Row])] = Seq(
      "review-ok" -> base,
      "review-nul" -> (base :+ plain("scala-native/a\u0000b.c")),
      "review-long" -> (base :+ plain("scala-native/" + "L" * 300 + ".c")),
      "review-clash" -> (base ++ Seq(bodied("scala-native/x", "plain\n"), plain("scala-native/x/y.c"))),
      "review-clash2" -> (base ++ Seq(plain("scala-native/x/y.c"), bodied("scala-native/x", "plain\n"))),
      "review-latin1h" -> (base ++ Seq(plain("scala-native/caf\u00e9.h"), bodied("scala-native/uses.c", "#include \"caf\u00e9.h\"\nint uses_fn(void){return 0;}\n"))),
      "review-latin1only" -> Seq(desc, plain("scala-native/m\u00e9thode.c")),
      "review-dotout" -> (base :+ plain("include/../x.h")),
      "review-abs" -> (base :+ plain("/scala-native/abs.c")),
      "review-bslash" -> (base ++ Seq(plain("scala-native\\bs.c"), plain("scala-native/in\\side.c"))),
      "review-tdot" -> (base ++ Seq(bodied("scala-native/t.c.", "garbage(\n"), plain("scala-native/d./u.c"))),
      "review-case" -> (base ++ Seq(plain("scala-native/Case.c"), plain("scala-native/case.c"))),
      "review-dirfile" -> (base ++ Seq(plain("scala-native/d.c/"), plain("scala-native/d.c"))),
      "review-symlink" -> (base ++ Seq(bodied("scala-native/link.h", "real.h", (0xa000 | 0x1ff) << 16), bodied("scala-native/real.h", "#define REAL 5\n"),
        bodied("scala-native/sl.c", "int sl_fn(void){return 0;}\n"))),
      "review-dup" -> Seq(desc, bodied("scala-native/jk.c", "int jk_first(void) { return 1; }\n"), jk),
      "review-abs2" -> (base :+ plain("/scala-native/abs.c")),
      "review-latin2" -> (base :+ plain("scala-native/m\u00e9thode.c")),
      "review-ctl" -> base)
    val product = for {
      (kind, piece) <- kinds
      (place, prefix) <- places
      flag <- if (notAscii(kind)) Seq(0, 0x800) else Seq(0)
      at <- 0 to 2
    } yield s"$kind-$place-$at${if (flag != 0) "-utf8flag" else ""}" -> base.patch(at, Seq(plain(prefix + piece, flag)), 0)
    val alone = for {
      (kind, piece) <- kinds
      (place, prefix) <- places
    } yield s"$kind-$place-alone" -> Seq(desc, plain(prefix + piece))
    val clashes = Seq(
      "clash-file-then-dir" -> Seq("scala-native/x", "scala-native/x/y.c"), "clash-dir-then-file" -> Seq("scala-native/x/y.c", "scala-native/x"),
      "clash-entry-then-file" -> Seq("scala-native/d.c/", "scala-native/d.c"), "clash-file-then-entry" -> Seq("scala-native/d.c", "scala-native/d.c/"),
      "clash-deep" -> Seq("scala-native/a", "scala-native/a/b/c.c"), "clash-out" -> Seq("include/x", "include/x/y.h"),
      "clash-out-entry" -> Seq("include/d/", "include/d"), "clash-abs" -> Seq("/scala-native/x", "scala-native/x/y.c"))
      .map { case (name, names) => name -> (base ++ names.map(plain(_))) }
    val keys = Seq(
      "key-twice" -> Seq(desc, bodied("scala-native/jk.c", "int jk_one(void) { return 1; }\n"), jk),
      "key-slash" -> Seq(desc, bodied("scala-native/jk.c", "int jk_one(void) { return 1; }\n"), plain("/scala-native/jk.c")),
      "key-doubled" -> Seq(desc, plain("scala-native//jk.c"), bodied("scala-native/jk.c", "int jk_two(void) { return 2; }\n")),
      "key-descriptor" -> Seq(bodied("scala-native/scala-native.properties", "compile.c.options = -DJK=2\n"), jk, plain("/scala-native/scala-native.properties")),
      "key-root" -> Seq(desc, jk, plain("/")),
      "key-empty" -> Seq(desc, jk, plain("")))
    val comments = Seq(
      "comment-latin1" -> Seq(desc, jk, (b("x.class"), 0, b("caf\u00e9"), 0, None)),
      "comment-utf8" -> Seq(desc, jk, (b("x.class"), 0, b("caf\u00c3\u00a9"), 0, None)),
      "comment-latin1-flag" -> Seq(desc, jk, (b("x.class"), 0x800, b("caf\u00e9"), 0, None)))
    val noCode = Seq(
      "nocode-latin1" -> Seq(plain("x/m\u00e9thode.class")),
      "nocode-dot" -> Seq(plain("x/../y.class"), plain("scala-native/x.h")),
      "nocode-clash" -> Seq(plain("x"), plain("x/y.class")))
    review ++ product ++ alone ++ clashes ++ keys ++ comments ++ noCode
  }

  /// The cases of a central directory's other fields, each with its entries (a `null` data the table's own) and the
  /// bytes after the directory's last entry: encryption, methods, a header of 65535 bytes and one over, zip64 blocks
  /// good and bad and zip64 marks without one, other extra blocks, bytes after the last entry, a local header naming
  /// another file, a directory entry that holds data, no entry, and offsets, CRC-32s and sizes the data belies.
  def headerCases: Seq[(String, Seq[E], Array[Byte])] = {
    def e(name: String, data: Array[Byte] = null): E = E(b(name), data)
    val base = Seq(e("scala-native/scala-native.properties"), e("scala-native/jk.c"))
    def le(value: Long, width: Int): Array[Byte] = Array.tabulate(width)(i => ((value >> (8 * i)) & 0xff).toByte)
    def zip64(fields: Long*): Array[Byte] = le(1, 2) ++ le(8L * fields.length, 2) ++ fields.flatMap(le(_, 8))
    val M = 0xffffffffL
    val places = Seq("in" -> "scala-native/x.c", "out" -> "include/x.h")
    // Each case's source a function of its own, so that the libraries of the table link into one program.
    val perPlace = places.flatMap { case (place, name) =>
      def at(label: String): (String, Array[Byte], Long) = {
        val source = b(s"int jn_${label.replace('-', '_')}_$place(void) { return 0; }\n")
        (s"$label-$place", source, source.length.toLong)
      }
      Seq[(String, (Array[Byte], Long) => E)](
        "cen-encrypted" -> ((d, _) => e(name, d).copy(flag = 1)),
        "cen-method12" -> ((d, _) => e(name, d).copy(method = 12)),
        "cen-method99" -> ((d, _) => e(name, d).copy(method = 99)),
        "cen-zip64" -> ((d, n) => e(name, d).copy(csize = M, size = M, extra = zip64(n, n))),
        "cen-zip64-badsize" -> ((d, n) => e(name, d).copy(size = M, extra = zip64(n, n))),
        "cen-zip64-empty" -> ((d, _) => e(name, d).copy(offset = M, extra = le(1, 2) ++ le(0, 2))),
        "cen-zip64-negative" -> ((d, _) => e(name, d).copy(size = M, extra = zip64(-1L))),
        "cen-zip64-unmarked" -> ((d, _) => e(name, d).copy(extra = le(1, 2) ++ le(0, 2))),
        "cen-marks-csize" -> ((d, _) => e(name, d).copy(csize = M)),
        "cen-marks-size" -> ((d, _) => e(name, d).copy(size = M)),
        "cen-marks-offset" -> ((d, _) => e(name, d).copy(offset = M)),
        "cen-marks-disk" -> ((d, _) => e(name, d).copy(disk = 0xffff)),
        "cen-extra-overrun" -> ((d, _) => e(name, d).copy(extra = le(0x5455, 2) ++ le(9, 2) ++ Array[Byte](1, 2))),
        "cen-extra-other" -> ((d, _) => e(name, d).copy(extra = le(0x5455, 2) ++ le(1, 2) ++ Array[Byte](1, 9))),
        "cen-offset" -> ((d, _) => e(name, d).copy(offset = 1)),
        "cen-crc" -> ((d, _) => e(name, d).copy(crc = 0)),
        "cen-size" -> ((d, n) => e(name, d).copy(size = n + 5)),
        "cen-local-name" -> ((d, _) => e(name, d).copy(local = b(name.replace("x.", "other.")))))
        .map { case (label, made) =>
          val (case_, source, n) = at(label)
          (case_, base :+ made(source, n), Array.emptyByteArray)
        }
    }
    val limit = b("c" * (0xffff - 46 - 7))
    perPlace ++ Seq(
      ("cen-comment-limit", base :+ e("x.class").copy(comment = limit), Array.emptyByteArray),
      ("cen-comment-over", base :+ e("x.class").copy(comment = limit :+ 'c'.toByte), Array.emptyByteArray),
      ("cen-trailing46", base, Array.fill[Byte](46)(0)),
      ("cen-trailing5", base, Array.fill[Byte](5)(7)),
      ("cen-trailing5-nocode", Seq(e("x.class")), Array.fill[Byte](5)(7)),
      ("cen-slash-data", base :+ e("scala-native/y.c/", b("int not_a_file;\n")), Array.emptyByteArray),
      ("cen-no-entry", Seq.empty, Array.emptyByteArray))
  }

  /// An entry of a jar written byte by byte (`rawJar`): its name and data, deflated or stored, its flag and external
  /// attributes, its local and central extra fields; the size, the compressed size and the CRC-32 its headers give
  /// where not its own (-1); a data descriptor after it (1, with its signature; 2, without; its local header's sizes and
  /// CRC-32 then 0, its flag's bit 3 set); bytes after its deflate stream within its compressed size; bytes cut from the
  /// end of its compressed data; a deflate stream written by hand (`stream`) in the place of the data's own.
  final case class R(name: String, data: Array[Byte], deflate: Boolean = false, flag: Int = 0, attr: Int = 0,
      localExtra: Array[Byte] = Array.emptyByteArray, cenExtra: Array[Byte] = Array.emptyByteArray, size: Long = -1, csize: Long = -1,
      crc: Long = -1, descriptor: Int = 0, trail: Array[Byte] = Array.emptyByteArray, cut: Int = 0, stream: Array[Byte] = null)

  def deflated(data: Array[Byte]): Array[Byte] = {
    val d = new java.util.zip.Deflater(6, true)
    d.setInput(data); d.finish()
    val out = new ByteArrayOutputStream
    val buf = new Array[Byte](4096)
    while (!d.finished()) { val n = d.deflate(buf); out.write(buf, 0, n) }
    d.end()
    out.toByteArray
  }

  /// A writer of a jar's bytes that says where it is: `put` appends little-endian numbers and bytes.
  final class Out {
    val bytes = new ByteArrayOutputStream
    def at: Int = bytes.size
    def u16(v: Long): Unit = { bytes.write((v & 0xff).toInt); bytes.write(((v >> 8) & 0xff).toInt) }
    def u32(v: Long): Unit = { u16(v & 0xffff); u16((v >> 16) & 0xffff) }
    def u64(v: Long): Unit = { u32(v & 0xffffffffL); u32((v >>> 32) & 0xffffffffL) }
    def put(b: Array[Byte]): Unit = bytes.write(b)
  }

  /// An entry's local header, data and descriptor at the writer's place; what its central header gives: (offset, crc,
  /// csize, size).
  def local(out: Out, e: R, offsetFrom: Int): (Long, Long, Long, Long) = {
    val crc32 = new java.util.zip.CRC32
    crc32.update(e.data)
    val payload = (if (e.stream != null) e.stream else if (e.deflate) deflated(e.data) else e.data) ++ e.trail
    val stored = payload.dropRight(e.cut)
    val crc = if (e.crc >= 0) e.crc else crc32.getValue
    val csize = if (e.csize >= 0) e.csize else stored.length.toLong
    val size = if (e.size >= 0) e.size else e.data.length.toLong
    val offset = (out.at - offsetFrom).toLong
    val name = b(e.name)
    val described = e.descriptor > 0
    out.u32(0x04034b50L); out.u16(20); out.u16(e.flag | (if (described) 8 else 0)); out.u16(if (e.deflate) 8 else 0); out.u16(0); out.u16(0x21)
    out.u32(if (described) 0 else crc); out.u32(if (described) 0 else csize); out.u32(if (described) 0 else size)
    out.u16(name.length); out.u16(e.localExtra.length); out.put(name); out.put(e.localExtra); out.put(stored)
    if (described) { if (e.descriptor == 1) out.u32(0x08074b50L); out.u32(crc); out.u32(csize); out.u32(size) }
    (offset, crc, csize, size)
  }

  def central(out: Out, e: R, placed: (Long, Long, Long, Long)): Unit = {
    val (offset, crc, csize, size) = placed
    val name = b(e.name)
    out.u32(0x02014b50L); out.u16(0x0314); out.u16(20); out.u16(e.flag | (if (e.descriptor > 0) 8 else 0)); out.u16(if (e.deflate) 8 else 0)
    out.u16(0); out.u16(0x21); out.u32(crc); out.u32(csize); out.u32(size); out.u16(name.length); out.u16(e.cenExtra.length); out.u16(0)
    out.u16(0); out.u16(0); out.u32(e.attr.toLong & 0xffffffffL); out.u32(offset); out.put(name); out.put(e.cenExtra)
  }

  def endRecord(out: Out, count: Long, size: Long, offset: Long, comment: Array[Byte]): Unit = {
    out.u32(0x06054b50L); out.u16(0); out.u16(0); out.u16(count); out.u16(count); out.u32(size); out.u32(offset); out.u16(comment.length); out.put(comment)
  }

  def zip64Record(out: Out, count: Long, size: Long, offset: Long): Unit = {
    out.u32(0x06064b50L); out.u64(44); out.u16(45); out.u16(45); out.u32(0); out.u32(0); out.u64(count); out.u64(count); out.u64(size); out.u64(offset)
  }

  def locator(out: Out, at: Long): Unit = { out.u32(0x07064b50L); out.u32(0); out.u64(at); out.u32(1) }

  /// A jar written byte by byte: a stub before it (its offsets the jar's own, or the file's where `absolute`), its
  /// entries, bytes between the directory and its end, a zip64 end record and locator where `zip64` (the plain end
  /// record's fields then its marks where `marks`), the end record's comment, and bytes after it.
  def rawJar(es: Seq[R], stub: Array[Byte] = Array.emptyByteArray, absolute: Boolean = false, gap: Array[Byte] = Array.emptyByteArray,
      zip64: Boolean = false, marks: Boolean = false, comment: Array[Byte] = Array.emptyByteArray, after: Array[Byte] = Array.emptyByteArray): Array[Byte] = {
    val out = new Out
    out.put(stub)
    val from = if (absolute) 0 else stub.length
    val placed = es.map(local(out, _, from))
    val cenAt = out.at
    es.zip(placed).foreach { case (e, g) => central(out, e, g) }
    val cenLen = out.at - cenAt
    out.put(gap)
    if (zip64) {
      val recordAt = out.at
      zip64Record(out, es.length.toLong, cenLen.toLong, (cenAt - from).toLong)
      locator(out, recordAt.toLong)
    }
    if (marks) endRecord(out, 0xffff, 0xffffffffL, 0xffffffffL, comment)
    else endRecord(out, es.length.toLong, cenLen.toLong, (cenAt - from).toLong, comment)
    out.put(after)
    out.bytes.toByteArray
  }

  /// A C source of a function of its own, so that every jar of the table links into one program.
  def src(tag: String): Array[Byte] = b(s"int jn_raw_${tag.replace('-', '_')}(void) { return 0; }\n")

  /// The jars of the table written byte by byte, each with its mark (`classpath`, `teqread` or none): where the JDK
  /// finds the central directory (end records in the comment, in an entry's data, in an extra field; zip64 locators
  /// real and planted; stubs of several lengths; a directory that does not end at its end record; bytes after the
  /// end record); the copy (sizes understated and overstated, streams cut and with bytes after their end, stored data
  /// of a wrong size, CRC-32s the data belies by method and with a long local extra field, data descriptors with and
  /// without their signature); and names: a symbolic link's mode, names differing by case alone and by Unicode
  /// normalisation alone, the multi-release layout.
  def rawCases: Seq[(String, Array[Byte], String)] = {
    val desc = R("scala-native/scala-native.properties", descriptor)
    def jk(tag: String) = R("scala-native/jk.c", src(tag))
    // A second directory in an entry's data, naming scala-native/evil.c, whose local header lies there too; the end
    // record's comment holds a second end record pointing at it, reaching the file's end or two bytes short of it.
    def fakeEnd(tag: String, short: Boolean): Array[Byte] = {
      val out = new Out
      val good = R("scala-native/good.c", src(tag + "-good"))
      val gGiven = local(out, good, 0)
      val padName = b("pad.bin")
      val padAt = out.at
      val inner = new Out
      val evil = R("scala-native/evil.c", src(tag + "-evil"))
      val evilAt = padAt + 30 + padName.length
      val eGiven = local(inner, evil, -evilAt)
      val cen2At = evilAt + inner.at
      central(inner, evil, eGiven)
      val cen2Len = evilAt + inner.at - cen2At
      val pad = R("pad.bin", inner.bytes.toByteArray)
      val pGiven = local(out, pad, 0)
      val cenAt = out.at
      central(out, good, gGiven); central(out, pad, pGiven)
      val cenLen = out.at - cenAt
      val second = new Out
      endRecord(second, 1, cen2Len.toLong, cen2At.toLong, Array.emptyByteArray)
      endRecord(out, 2, cenLen.toLong, cenAt.toLong, second.bytes.toByteArray ++ (if (short) b("xx") else Array.emptyByteArray))
      out.bytes.toByteArray
    }
    // A zip64 locator as the last 20 bytes of the directory (the data of the last entry's central extra block), pointing
    // at a zip64 end record in the end record's comment whose values are the plain record's (or not, where `differ`), a
    // second directory of the same size and count before it naming scala-native/evil.c.
    def plantedLocator(tag: String, differ: Boolean): Array[Byte] = {
      val lead = R("lead.bin", b("l" * 1000))
      val good = R("scala-native/good.c", src(tag + "-good"))
      val note0 = R("note.txt", b("note\n"), cenExtra = Array.fill[Byte](24)(0))
      val out = new Out
      val lGiven = local(out, lead, 0)
      val gGiven = local(out, good, 0)
      val nGiven = local(out, note0, 0)
      val cenAt = out.at
      val cenLen = 3 * 46 + lead.name.length + good.name.length + note0.name.length + 24
      val endAt = cenAt + cenLen
      // The comment: the local headers and data of the second directory's entries, that directory (of the first's size
      // and count, its offsets counted from `locpos2`), the zip64 end record.
      val commentAt = endAt + 22
      val comment = new Out
      val lead2 = R("lead.bin", b("m" * 1000))
      val evil = R("scala-native/evil.c", src(tag + "-evil"))
      val note2 = R("note.txt", b("tone\n"), cenExtra = Array.fill[Byte](24)(0))
      val placed2 = Seq(lead2, evil, note2).map(e => local(comment, e, -commentAt))
      val cen2At = commentAt + comment.at
      val locpos2 = cen2At - cenAt
      Seq(lead2, evil, note2).zip(placed2).foreach { case (e, (abs, crc, csize, size)) => central(comment, e, (abs - locpos2, crc, csize, size)) }
      val recordAt = commentAt + comment.at
      zip64Record(comment, 3, cenLen.toLong + (if (differ) 1 else 0), cenAt.toLong)
      val loc = new Out
      loc.u16(0xcafe); loc.u16(20); locator(loc, recordAt.toLong)
      val note = note0.copy(cenExtra = loc.bytes.toByteArray)
      central(out, lead, lGiven); central(out, good, gGiven); central(out, note, nGiven)
      endRecord(out, 3, cenLen.toLong, cenAt.toLong, comment.bytes.toByteArray)
      out.bytes.toByteArray
    }
    val endInData = R("data.bin", b("x") ++ { val o = new Out; endRecord(o, 9, 9, 9, Array.emptyByteArray); o.bytes.toByteArray } ++ b("y"))
    val endInExtra = R("extra.txt", b("e\n"), cenExtra = { val o = new Out; o.u16(0xcafe); o.u16(22); endRecord(o, 9, 9, 9, Array.emptyByteArray); o.bytes.toByteArray })
    val text = b("compile.c.options = -DJK=1\n" + "# " + "c" * 3000 + "\n")
    val big = b("int jn_raw_big(void) { return 0; }\n" + "/* " + "b" * 20000 + " */\n")
    Seq(
      ("raw-end-in-comment-reaching", fakeEnd("ecr", short = false), ""),
      ("raw-end-in-comment-short", fakeEnd("ecs", short = true), "teqread"),
      ("raw-end-in-data", rawJar(Seq(desc, jk("eid"), endInData)), ""),
      ("raw-end-in-extra", rawJar(Seq(desc, jk("eie"), endInExtra)), ""),
      ("raw-locator-real", rawJar(Seq(desc, jk("lr")), zip64 = true), ""),
      ("raw-locator-planted", plantedLocator("lp", differ = false), "teqread"),
      ("raw-locator-planted-differing", plantedLocator("lpd", differ = true), ""),
      ("raw-stub-1", rawJar(Seq(desc, jk("s1")), stub = b("#")), "classpath"),
      ("raw-stub-7", rawJar(Seq(desc, jk("s7")), stub = b("#!/bin/")), "classpath"),
      ("raw-stub-106", rawJar(Seq(desc, jk("s106")), stub = b("#" * 106)), "classpath"),
      ("raw-stub-7-absolute", rawJar(Seq(desc, jk("s7a")), stub = b("#!/bin/"), absolute = true), ""),
      ("raw-stub-106-absolute", rawJar(Seq(desc, jk("s106a")), stub = b("#" * 106), absolute = true), ""),
      ("raw-gap", rawJar(Seq(desc, jk("gap")), gap = b("gapgap")), "teqread"),
      ("raw-after-end", rawJar(Seq(desc, jk("ae")), after = b("\u0000\u0000\u0000")), ""),
      ("raw-zip64-end", rawJar(Seq(desc, jk("z64")), zip64 = true, marks = true), "classpath"),
      ("raw-size-under", rawJar(Seq(R("scala-native/scala-native.properties", text, deflate = true, size = 10), jk("su"))), ""),
      ("raw-size-over", rawJar(Seq(R("scala-native/scala-native.properties", text, deflate = true, size = 99999), jk("so"))), ""),
      ("raw-stream-cut", rawJar(Seq(desc, R("scala-native/big.c", big, deflate = true, cut = 40))), ""),
      ("raw-stream-trailing", rawJar(Seq(desc, R("scala-native/big.c", big, deflate = true, trail = b("trail")))), ""),
      ("raw-stored-size-wrong", rawJar(Seq(desc, R("scala-native/ssw.c", src("ssw"), size = 3))), ""),
      ("raw-crc-stored", rawJar(Seq(desc, R("scala-native/crs.c", src("crs"), crc = 1))), ""),
      ("raw-crc-deflated", rawJar(Seq(desc, R("scala-native/crd.c", src("crd"), deflate = true, crc = 1))), ""),
      ("raw-crc-stored-extra", rawJar(Seq(desc, R("scala-native/cre.c", src("cre"), crc = 1, localExtra = Array.fill[Byte](200)(0)))), ""),
      ("raw-descriptor-signed", rawJar(Seq(desc, R("scala-native/ds.c", src("ds"), deflate = true, descriptor = 1))), ""),
      ("raw-descriptor-unsigned", rawJar(Seq(desc, R("scala-native/du.c", src("du"), descriptor = 2))), ""),
      ("raw-symlink-mode", rawJar(Seq(desc, jk("sym"), R("scala-native/sym.h", b("target.h"), attr = (0xa000 | 0x1ff) << 16))), ""),
      ("raw-case", rawJar(Seq(desc, R("scala-native/Ab.c", src("case-upper")), R("scala-native/ab.c", src("case-lower")))), ""),
      ("raw-unicode-normalisation", rawJar(Seq(desc, R("scala-native/Ã©.c", src("nfc")), R("scala-native/eÌ\u0081.c", src("nfd")))), ""),
      ("raw-multi-release-alone", rawJar(Seq(R("META-INF/MANIFEST.MF", b("Manifest-Version: 1.0\nMulti-Release: true\n")), R("META-INF/versions/9/scala-native/mr.c", src("mr")))), ""),
      ("raw-multi-release", rawJar(Seq(R("META-INF/MANIFEST.MF", b("Manifest-Version: 1.0\nMulti-Release: true\n")), desc, jk("mr2"), R("META-INF/versions/9/scala-native/mr2.c", src("mr2-versioned")))), ""),
      ("raw-stub-end-in-comment", stubEndInComment("sec"), "teqread")) ++
      zlibFamily.map { case (tag, stream, plain) =>
        (s"raw-zlib-$tag", rawJar(Seq(R(s"scala-native/z_${tag.replace('-', '_')}.c", plain, deflate = true, stream = stream))), "")
      }
  }

  /// A stub before a jar whose offsets are the jar's own, and in its end record's comment a second end record naming
  /// the same directory by its place in the file, two bytes short of the file's end: the JDK's readers take the first
  /// (the second's comment does not reach the end, and ZipFile finds no directory before it) and count the offsets from
  /// the stub's end; teq's class path read takes the last end record in the file, the same directory, its offsets
  /// counted from the file's start.
  def stubEndInComment(tag: String): Array[Byte] = {
    val stub = b("#!/bin/")
    val es = Seq(R("scala-native/jk.c", src(tag)))
    val out = new Out
    out.put(stub)
    val placed = es.map(local(out, _, stub.length))
    val cenAt = out.at
    es.zip(placed).foreach { case (e, g) => central(out, e, g) }
    val cenLen = out.at - cenAt
    val second = new Out
    endRecord(second, 1, cenLen.toLong, cenAt.toLong, Array.emptyByteArray)
    endRecord(out, 1, cenLen.toLong, (cenAt - stub.length).toLong, second.bytes.toByteArray ++ b("xx"))
    out.bytes.toByteArray
  }

  /// A deflate stream written bit by bit: numbers low bits first, Huffman codes high bits first.
  final class Bits {
    private val out = new ByteArrayOutputStream
    private var acc = 0
    private var n = 0
    def put(v: Int, k: Int): Bits = {
      for (i <- 0 until k) { acc |= ((v >> i) & 1) << n; n += 1; if (n == 8) { out.write(acc); acc = 0; n = 0 } }
      this
    }
    def code(c: Int, k: Int): Bits = { for (i <- (0 until k).reverse) put((c >> i) & 1, 1); this }
    def align(): Bits = { if (n > 0) { out.write(acc); acc = 0; n = 0 }; this }
    def raw(bytes: Array[Byte]): Bits = { align(); out.write(bytes); this }
    def bytes: Array[Byte] = { val o = out.toByteArray; if (n > 0) o :+ acc.toByte else o }
  }

  /// Each symbol's canonical code for a set of code lengths (RFC 1951 3.2.2).
  def canonical(lens: Array[Int]): Array[Int] = {
    val count = new Array[Int](16)
    lens.foreach(l => if (l > 0) count(l) += 1)
    val next = new Array[Int](16)
    var code = 0
    for (len <- 1 until 16) { code = (code + count(len - 1)) << 1; next(len) = code }
    lens.map(l => if (l == 0) 0 else { val c = next(l); next(l) += 1; c })
  }

  val clOrder = Array(16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15)
  /// A code length code of 4 bits for each length 0 to 15; one of 4 bits for 0 to 12 and the three repeats.
  val clPlain: Array[Int] = Array.tabulate(19)(i => if (i < 16) 4 else 0)
  val clRepeat: Array[Int] = Array.tabulate(19)(i => if (i <= 12 || i >= 16) 4 else 0)
  /// A complete literal/length code of 265 symbols (every byte of 9 bits; the end of a block and the lengths 3 to 10
  /// of 2 to 9) and a complete distance code of 16 of 4 bits.
  val litPlain: Array[Int] = Array.fill(256)(9) ++ Array(2, 3, 4, 5, 6, 7, 8, 9, 9)
  val distPlain: Array[Int] = Array.fill(16)(4)
  /// The fixed codes (RFC 1951 3.2.6).
  val litFixed: Array[Int] = Array.tabulate(288)(s => if (s < 144) 8 else if (s < 256) 9 else if (s < 280) 7 else 8)

  /// A dynamic block's header and code lengths: the counts (or `nlen` and `ndist` as written), the code length code's
  /// lengths and how many of them are written, the lengths as the code length code's symbols (each with its repeat's
  /// extra bits and their count).
  def dynamicHead(bits: Bits, nlen: Int, ndist: Int, cl: Array[Int], ncode: Int, symbols: Seq[(Int, Int, Int)], last: Boolean = true): Bits = {
    bits.put(if (last) 1 else 0, 1).put(2, 2).put(nlen - 257, 5).put(ndist - 1, 5).put(ncode - 4, 4)
    for (i <- 0 until ncode) bits.put(cl(clOrder(i)), 3)
    val codes = canonical(cl)
    symbols.foreach { case (sym, extra, k) => bits.code(codes(sym), cl(sym)); if (k > 0) bits.put(extra, k) }
    bits
  }
  def asSymbols(lens: Array[Int]): Seq[(Int, Int, Int)] = lens.toSeq.map(l => (l, 0, 0))
  def literals(bits: Bits, lit: Array[Int], text: Array[Byte]): Bits = {
    val codes = canonical(lit)
    text.foreach(c => bits.code(codes(c & 0xff), lit(c & 0xff)))
    bits
  }
  def symbol(bits: Bits, lit: Array[Int], sym: Int): Bits = bits.code(canonical(lit)(sym), lit(sym))
  /// A last dynamic block of the plain codes holding `text`.
  def plainBlock(text: Array[Byte], lit: Array[Int] = litPlain, dist: Array[Int] = distPlain): Array[Byte] = {
    val bits = dynamicHead(new Bits, lit.length, dist.length, clPlain, 19, asSymbols(lit ++ dist))
    symbol(literals(bits, lit, text), lit, 256).bytes
  }
  def fixedBlock(body: Bits => Unit): Array[Byte] = {
    val bits = new Bits
    bits.put(1, 1).put(1, 2)
    body(bits)
    symbol(bits, litFixed, 256).bytes
  }

  /// The streams that reach each of zlib 1.3.1's refusals of a raw deflate stream (the JDK's bundled inflate.c and
  /// inftrees.c), each beside one that just does not: a block type of 3 against a dynamic one; a stored block's length
  /// whose complement is wrong and one whose is right; 287 literal/length symbols or 31 distance symbols against 286
  /// and 30; a code length code incomplete, over-subscribed and empty (its zeros read as lengths 0, so that the end of
  /// the block has no code) against a complete one; a repeat of the previous length first and after a length, a repeat
  /// past the last symbol and one ending at it; no code for the end of the block; a literal/length code incomplete or
  /// over-subscribed, a single code of one bit (zlib takes it) and its missing code read; the same of distance codes,
  /// and an empty distance code where no distance is read; the fixed codes' symbols 286 and 287 against 285, and their
  /// distances 30 and 31 against 0; a distance one byte past what is written against one reaching its start. Each
  /// with the C source the stream inflates to where it inflates.
  def zlibFamily: Seq[(String, Array[Byte], Array[Byte])] = {
    def c(tag: String): Array[Byte] = b(s"int zf_${tag.replace('-', '_')}(void) { return 0; }\n")
    val padding = Array.fill[Byte](64)(0)
    def plain(tag: String) = (tag, plainBlock(c(tag)), c(tag))
    def raw(tag: String, bytes: Array[Byte]) = (tag, bytes ++ padding, c(tag))
    val stored = (len: Int, complement: Int, tag: String) => {
      val text = c(tag)
      val bits = new Bits
      bits.put(1, 1).put(0, 2).align()
      bits.put(len, 16).put(complement, 16)
      (tag, bits.bytes ++ text, text)
    }
    def withLit(tag: String, lit: Array[Int], text: Array[Byte], data: Bits => Unit, dist: Array[Int] = distPlain) = {
      val bits = dynamicHead(new Bits, lit.length, dist.length, clPlain, 19, asSymbols(lit ++ dist))
      data(bits)
      (tag, bits.bytes, text)
    }
    // The repeats: a 9, then 16s copying it to 256 nines; the plain code's other lengths; a 4 and 16s to 16 fours.
    val repeatSymbols: Seq[(Int, Int, Int)] =
      Seq((9, 0, 0)) ++ Seq.fill(42)((16, 3, 2)) ++ Seq((16, 0, 2)) ++ Seq(2, 3, 4, 5, 6, 7, 8, 9, 9).map(l => (l, 0, 0)) ++
        Seq((4, 0, 0), (16, 3, 2), (16, 3, 2), (16, 0, 2))
    def repeats(tag: String, symbols: Seq[(Int, Int, Int)], nlen: Int = 265) = {
      val text = c(tag)
      val lit = litPlain ++ Array.fill(nlen - 265)(0)
      val bits = dynamicHead(new Bits, nlen, 16, clRepeat, 19, symbols)
      symbol(literals(bits, lit, text), lit, 256)
      (tag, bits.bytes ++ padding, text)
    }
    val lit286 = litPlain ++ Array.fill(21)(0)
    val single = Array.fill(257)(0).updated(256, 1)
    val incomplete = Array.fill(257)(0).patch(0, Array.fill(14)(4), 14).updated(256, 4)
    val over = Array.fill(257)(0).patch(0, Array.fill(16)(4), 16).updated(256, 4)
    def match1(bits: Bits, lit: Array[Int], dist: Array[Int], distanceCode: Int): Bits = {
      symbol(bits, lit, 257) // a length of 3
      bits.code(canonical(dist)(distanceCode), dist(distanceCode))
    }
    val far = (tag: String, back: Int) => {
      val text = b("//f\n") ++ c(tag)
      val distance = text.length + back
      // The plain distance code's codes 0 to 15 (RFC 1951 3.2.5: bases 1 to 193, their extra bits).
      val base = Seq(1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193)
      val code = base.lastIndexWhere(_ <= distance)
      val extra = if (code < 4) 0 else code / 2 - 1
      val bits = dynamicHead(new Bits, 265, 16, clPlain, 19, asSymbols(litPlain ++ distPlain))
      literals(bits, litPlain, text)
      symbol(bits, litPlain, 257)
      bits.code(canonical(distPlain)(code), 4).put(distance - base(code), extra)
      literals(bits, litPlain, b("\n"))
      symbol(bits, litPlain, 256)
      (tag, bits.bytes, text ++ b("//f\n"))
    }
    Seq(
      raw("type-3", new Bits().put(1, 1).put(3, 2).bytes),
      plain("type-2"),
      stored(0, 0, "stored-bad"),
      { val t = c("stored-ok"); stored(t.length, t.length ^ 0xffff, "stored-ok") },
      raw("nlen-287", dynamicHead(new Bits, 287, 16, clPlain, 19, asSymbols(litPlain ++ Array.fill(22)(0) ++ distPlain)).bytes),
      ("nlen-286", plainBlock(c("nlen-286"), lit286), c("nlen-286")),
      raw("ndist-31", dynamicHead(new Bits, 265, 31, clPlain, 19, asSymbols(litPlain ++ distPlain ++ Array.fill(15)(0))).bytes),
      ("ndist-30", plainBlock(c("ndist-30"), litPlain, distPlain ++ Array.fill(14)(0)), c("ndist-30")),
      raw("clc-incomplete", dynamicHead(new Bits, 265, 16, clPlain.updated(15, 0), 19, Seq.empty).bytes),
      raw("clc-over", dynamicHead(new Bits, 265, 16, clPlain.updated(16, 4), 19, Seq.empty).bytes),
      raw("clc-empty", dynamicHead(new Bits, 265, 16, Array.fill(19)(0), 19, Seq.empty).bytes),
      plain("clc-complete"),
      repeats("repeat-first", Seq((16, 0, 2))),
      repeats("repeat-after", repeatSymbols),
      repeats("repeat-past", repeatSymbols.dropRight(1) ++ Seq((18, 127, 7)), 265),
      repeats("repeat-to-end", repeatSymbols.take(53) ++ Seq((18, 10, 7)) ++ repeatSymbols.drop(53), 286),
      raw("no-eob", dynamicHead(new Bits, 257, 16, clPlain, 19, asSymbols(Array.fill(256)(8) ++ Array(0) ++ distPlain)).bytes),
      plain("eob"),
      withLit("lit-incomplete", incomplete, c("lit-incomplete"), _ => ()),
      withLit("lit-over", over, c("lit-over"), _ => ()),
      withLit("lit-single", single, Array.emptyByteArray, bits => symbol(bits, single, 256)),
      withLit("lit-single-missing", single, c("lit-single-missing"), bits => bits.put(1, 1)),
      withLit("dist-incomplete", litPlain, c("dist-incomplete"), _ => (), Array(2, 2, 2)),
      withLit("dist-over", litPlain, c("dist-over"), _ => (), Array(1, 1, 1)),
      withLit("dist-single", litPlain, c("dist-single") ++ b("\n\n\n"), bits => {
        literals(bits, litPlain, c("dist-single")); match1(bits, litPlain, Array(1), 0); symbol(bits, litPlain, 256)
      }, Array(1)),
      withLit("dist-single-missing", litPlain, c("dist-single-missing"), bits => {
        literals(bits, litPlain, c("dist-single-missing")); symbol(bits, litPlain, 257); bits.put(1, 1)
      }, Array(1)),
      withLit("dist-empty", litPlain, c("dist-empty"), bits => { literals(bits, litPlain, c("dist-empty")); symbol(bits, litPlain, 256) }, Array(0)),
      raw("fixed-286", fixedBlock(bits => { literals(bits, litFixed, c("fixed-286")); symbol(bits, litFixed, 286) })),
      raw("fixed-287", fixedBlock(bits => { literals(bits, litFixed, c("fixed-287")); symbol(bits, litFixed, 287) })),
      ("fixed-285", fixedBlock(bits => {
        literals(bits, litFixed, c("fixed-285") ++ b("//")); symbol(bits, litFixed, 285); bits.code(0, 5); literals(bits, litFixed, b("\n"))
      }), c("fixed-285") ++ b("/" * 260 + "\n")),
      raw("fixed-dist-30", fixedBlock(bits => { literals(bits, litFixed, c("fixed-dist-30")); symbol(bits, litFixed, 257); bits.code(30, 5) })),
      raw("fixed-dist-31", fixedBlock(bits => { literals(bits, litFixed, c("fixed-dist-31")); symbol(bits, litFixed, 257); bits.code(31, 5) })),
      ("fixed-dist-0", fixedBlock(bits => { literals(bits, litFixed, c("fixed-dist-0")); symbol(bits, litFixed, 257); bits.code(0, 5) }), c("fixed-dist-0") ++ b("\n\n\n")),
      far("far-past", 1),
      far("far-start", 0))
  }

  /// What Scala Native's build made of a jar: its exception, no library, or the files under scala-native/.
  def judged(jar: Path, names: Seq[Array[Byte]], work: Path): String = {
    val config = Config.empty.withBaseDir(work).withClassPath(Seq(jar))
    val libs =
      try NativeLib.findNativeLibs(config)
      catch {
        case e: BuildException if e.getMessage.startsWith("No Scala Native libraries were found") => return "none"
        case e: Exception => return s"refused ZipFile ${e.getClass.getName}: ${said(e.getMessage)}"
      }
    val unpack = NativeLib.getClass.getDeclaredMethods.find(_.getName.endsWith("unpackNativeCode")).get
    unpack.setAccessible(true)
    val dest =
      try unpack.invoke(NativeLib, libs.head).asInstanceOf[Path]
      catch {
        case e: java.lang.reflect.InvocationTargetException =>
          val cause = e.getCause
          return s"refused unzip ${cause.getClass.getName}: ${said(String.valueOf(cause.getMessage).replace(work.toString, "<work>"))}"
      }
    val code = dest.resolve("scala-native")
    val found =
      if (!Files.isDirectory(code)) Seq.empty
      else {
        val s = Files.walk(code)
        try s.iterator.asScala.filter(Files.isRegularFile(_)).toSeq finally s.close()
      }
    val named = found.map(f => (dest.relativize(f).toString.getBytes("UTF-8"), sha256(Files.readAllBytes(f)).take(16))).sortBy(p => new String(p._1, "ISO-8859-1"))
    (if (holdsFileAndDirectory(names)) "clash " else "") + "files" + named.map { case (n, d) => s" ${escaped(n)}=$d" }.mkString
  }

  /// The key and the kind the JDK's zip file system gives an entry's name: its own `IndexNode(byte[] cen, int pos,
  /// int nlen)`, which reads the name after a central directory header's 46 bytes.
  val indexNode: java.lang.reflect.Constructor[_] = {
    val node = Class.forName("jdk.nio.zipfs.ZipFileSystem$IndexNode")
    val made = node.getDeclaredConstructor(classOf[Array[Byte]], Integer.TYPE, Integer.TYPE)
    made.setAccessible(true)
    made
  }
  def keyed(name: Array[Byte]): (String, Boolean) = {
    val node = indexNode.newInstance(Array.fill[Byte](46)(0) ++ name, Int.box(0), Int.box(name.length)).asInstanceOf[AnyRef]
    def field(n: String): AnyRef = {
      val f = node.getClass.getSuperclass.getDeclaredFields.find(_.getName == n).orElse(node.getClass.getDeclaredFields.find(_.getName == n)).get
      f.setAccessible(true)
      f.get(node)
    }
    (new String(field("name").asInstanceOf[Array[Byte]], "ISO-8859-1"), field("isdir").asInstanceOf[Boolean])
  }

  /// Whether the zip file system holds a name as a file and a directory both, by its own keys: entries of one key, one a
  /// directory and one not, or an entry below one that is a file.
  def holdsFileAndDirectory(names: Seq[Array[Byte]]): Boolean = {
    val keys = names.filter(_.nonEmpty).map(keyed)
    val files = keys.filter(!_._2).map(_._1).toSet
    keys.groupBy(_._1).values.exists(_.map(_._2).distinct.size > 1) || keys.exists { case (key, _) => files.exists(f => f != "/" && key.startsWith(f + "/")) }
  }

  def main(args: Array[String]): Unit = {
    if (args.headOption.contains("--judge")) return judgeEach(args.toSeq.drop(1).map(Paths.get(_)))
    if (args.length != 2) sys.error("usage: jar-names-oracle.scala <expected file> <directory of tool-jars.py's jars> | --judge <jar>...")
    val target = Paths.get(args(0))
    val toolJars = Paths.get(args(1)).toAbsolutePath
    val root = Paths.get("out/jar-names").toAbsolutePath
    if (Files.exists(root)) {
      val s = Files.walk(root)
      try s.iterator.asScala.toSeq.reverse.foreach(Files.delete(_)) finally s.close()
    }
    val all: Seq[(String, Seq[E], Array[Byte])] =
      cases.map { case (name, rows) => (name, rows.map { case (n, flag, comment, attr, body) => E(n, body.orNull, flag, comment, attr) }, Array.emptyByteArray) } ++ headerCases
    val lines = all.zipWithIndex.map { case ((name, rows, trailing), k) =>
      val entries = rows.zipWithIndex.map { case (entry, i) => if (entry.data == null) entry.copy(data = data(k, i, entry.name)) else entry }
      val bytes = jarBytes(entries, trailing)
      val jar = Files.createDirectories(root.resolve("jars")).resolve(s"$name.jar")
      Files.write(jar, bytes)
      val verdict = judged(jar, entries.map(_.name), root.resolve("work").resolve(name))
      def field(value: Long): String = if (value < 0) "" else value.toHexString
      val written = entries.map(e => Seq(e.flag.toHexString, e.attr.toHexString, escaped(e.name), escaped(e.comment), escaped(e.data), e.method.toHexString,
        escaped(e.extra), field(e.csize), field(e.size), field(e.offset), field(e.crc), e.disk.toHexString, escaped(e.local)).mkString(",")).mkString(";")
      s"$name ${sha256(bytes)} $verdict | $written | ${escaped(trailing)}"
    }
    val raw = rawCases.map { case (name, bytes, mark) =>
      val jar = Files.createDirectories(root.resolve("jars")).resolve(s"$name.jar")
      Files.write(jar, bytes)
      val names = scala.util.Try { val z = new java.util.zip.ZipFile(jar.toFile); try z.entries.asScala.map(_.getName.getBytes("UTF-8")).toVector finally z.close() }.getOrElse(Vector.empty)
      val verdict = judged(jar, names, root.resolve("work").resolve(name))
      s"$name ${sha256(bytes)} ${if (mark.isEmpty) "" else mark + " "}$verdict | raw | ${java.util.HexFormat.of().formatHex(bytes)}"
    }
    val provenance = Files.readAllLines(toolJars.resolve("provenance.txt")).asScala.toSeq.filter(_.nonEmpty)
    val tools = provenance.map { line =>
      val name = line.takeWhile(_ != ' ')
      val bytes = Files.readAllBytes(toolJars.resolve(s"$name.jar"))
      val jar = Files.createDirectories(root.resolve("jars")).resolve(s"$name.jar")
      Files.write(jar, bytes)
      val names = scala.util.Try { val z = new java.util.zip.ZipFile(jar.toFile); try z.entries.asScala.map(_.getName.getBytes("UTF-8")).toVector finally z.close() }.getOrElse(Vector.empty)
      val verdict = judged(jar, names, root.resolve("work").resolve(name))
      s"$name ${sha256(bytes)} tool ${if (classPathRefuses(bytes)) "classpath " else ""}$verdict | raw | ${java.util.HexFormat.of().formatHex(bytes)}"
    }
    val header = Seq(
      "# What Scala Native 0.5.12's build makes of each jar (refused, none or the files it unpacks under scala-native/), written",
      "# by tests/native-build/jar-names-oracle.scala (which says how to run it again) under:",
      s"# Scala Native 0.5.12's tools for Scala 3, Java ${System.getProperty("java.version")} ${System.getProperty("java.vendor")}",
      "# The jars marked `tool` were written by real tools (tests/native-build/tool-jars.py); each one's tool, version and command:") ++
      provenance.map("#   " + _)
    Files.write(target, (header ++ lines ++ raw ++ tools).mkString("", "\n", "\n").getBytes("UTF-8"))
    println(s"jar-names, tool-written: ${tools.length} jars, ${tools.count(_.contains(" tool refused "))} refused, ${tools.count(_.contains(" tool none |"))} none")
    val every = lines ++ raw
    println(s"jar-names: ${every.length} jars, ${every.count(_.contains(" refused "))} refused, ${every.count(_.contains(" none |"))} none, ${every.count(_.contains(" clash files"))} clash, ${raw.length} written byte by byte")
    flips(target.resolveSibling("inflate-flips.expected"), root)
  }

  /// Whether teq's class path read (src/zip.rs, `Zip::open`) refuses the jar as zip64: the last end record signature in
  /// its last 65,557 bytes carries a zip64 mark (an entry count of 0xFFFF, a directory size or offset of 0xFFFFFFFF).
  def classPathRefuses(jar: Array[Byte]): Boolean = {
    def u16(at: Int): Int = (jar(at) & 0xff) | (jar(at + 1) & 0xff) << 8
    def u32(at: Int): Long = (u16(at) | u16(at + 2).toLong << 16) & 0xffffffffL
    val from = math.max(0, jar.length - 22 - 65535)
    (jar.length - 22 to from by -1).find(i => u32(i) == 0x06054b50L).exists(i => u16(i + 10) == 0xffff || u32(i + 12) == 0xffffffffL || u32(i + 16) == 0xffffffffL)
  }

  /// What Scala Native's build makes of each jar named (a published library's, read where it lies): a line each, the
  /// jar's path, SHA-256 and verdict.
  def judgeEach(jars: Seq[Path]): Unit = {
    val root = Files.createTempDirectory("jar-names-judge")
    for ((jar, k) <- jars.zipWithIndex) {
      val names = scala.util.Try { val z = new java.util.zip.ZipFile(jar.toFile); try z.entries.asScala.map(_.getName.getBytes("UTF-8")).toVector finally z.close() }.getOrElse(Vector.empty)
      println(s"$jar ${sha256(Files.readAllBytes(jar))} ${judged(jar, names, root.resolve(s"w$k"))}")
    }
  }

  /// Valid deflate streams of a C text: the JDK's Deflater at levels 6, 1, 0 (stored), 9 coding Huffman codes alone,
  /// 6 filtered, 6 flushed halfway (two blocks and an empty stored one between); a fixed block with matches and a
  /// dynamic block whose lengths repeat, written by hand.
  def flipBases: Seq[(Array[Byte], Array[Byte])] = {
    val text = b((0 until 12).map(i => s"int flip_$i(int x) { return x * $i + ${i * i}; }\n").mkString)
    def deflater(level: Int, strategy: Int, flushHalfway: Boolean): Array[Byte] = {
      val d = new java.util.zip.Deflater(level, true)
      d.setStrategy(strategy)
      val out = new ByteArrayOutputStream
      val buf = new Array[Byte](65536)
      if (flushHalfway) {
        d.setInput(text, 0, text.length / 2)
        var n = d.deflate(buf, 0, buf.length, java.util.zip.Deflater.SYNC_FLUSH)
        out.write(buf, 0, n)
        d.setInput(text, text.length / 2, text.length - text.length / 2)
      } else d.setInput(text)
      d.finish()
      while (!d.finished()) { val n = d.deflate(buf); out.write(buf, 0, n) }
      d.end()
      out.toByteArray
    }
    import java.util.zip.Deflater.{DEFAULT_STRATEGY, FILTERED, HUFFMAN_ONLY}
    val family = zlibFamily.map { case (tag, stream, plain) => tag -> (stream, plain) }.toMap
    Seq(
      (deflater(6, DEFAULT_STRATEGY, false), text), (deflater(1, DEFAULT_STRATEGY, false), text), (deflater(0, DEFAULT_STRATEGY, false), text),
      (deflater(9, HUFFMAN_ONLY, false), text), (deflater(6, FILTERED, false), text), (deflater(6, DEFAULT_STRATEGY, true), text),
      family("fixed-285"), family("repeat-after"))
  }

  /// Bits flipped in the valid streams (`flipBases`; java.util.Random of a fixed seed: a stream, then one to three
  /// distinct bits of it), each stream the deflated data of `scala-native/flip.h` in a jar, read through the JDK's zip
  /// file system as Scala Native's copy reads an entry (`Files.newInputStream` of its path there): refused with its
  /// exception, or the bytes it gives, by their SHA-256's first 16 hexadecimal digits. Written to `target`, which the
  /// cargo test `copies_every_flipped_stream_as_the_jdk_does` (src/native/jar.rs) reads.
  def flips(target: Path, root: Path): Unit = {
    val seed = 20261122L
    val count = 3000
    val bases = flipBases
    val random = new java.util.Random(seed)
    val dir = Files.createDirectories(root.resolve("flips"))
    val rows = (0 until count).map { k =>
      val base = random.nextInt(bases.length)
      val (stream, plain) = bases(base)
      val bits = 1 + random.nextInt(3)
      val at = scala.collection.mutable.LinkedHashSet.empty[Int]
      while (at.size < bits) at += random.nextInt(stream.length * 8)
      val flipped = stream.clone()
      at.foreach(i => flipped(i / 8) = (flipped(i / 8) ^ (1 << (i % 8))).toByte)
      val jar = dir.resolve(s"flip-$k.jar")
      Files.write(jar, rawJar(Seq(R("scala-native/flip.h", plain, deflate = true, stream = flipped))))
      val fs = java.nio.file.FileSystems.newFileSystem(jar)
      val verdict =
        try {
          val in = Files.newInputStream(fs.getPath("scala-native/flip.h"))
          try s"bytes ${sha256(in.readAllBytes()).take(16)}" finally in.close()
        } catch { case e: java.io.IOException => s"refused ${e.getClass.getName}: ${e.getMessage}" }
        finally fs.close()
      s"flip $k $base ${at.mkString(",")} $verdict"
    }
    val header = Seq(
      "# What the JDK's zip file system reads of a deflated entry whose stream has bits flipped, written by",
      "# tests/native-build/jar-names-oracle.scala (`flips`) under:",
      s"# Java ${System.getProperty("java.version")} ${System.getProperty("java.vendor")}; then the seed, the valid streams in hexadecimal,",
      "# and for each flipped stream: its number, its valid stream's, the bits flipped (bit i is bit i % 8 of byte i / 8), the verdict.",
      s"seed $seed") ++ bases.zipWithIndex.map { case ((stream, _), i) => s"base $i ${java.util.HexFormat.of().formatHex(stream)}" }
    Files.write(target, (header ++ rows).mkString("", "\n", "\n").getBytes("UTF-8"))
    val refused = rows.count(_.contains(" refused "))
    println(s"inflate flips: $count streams of ${bases.length} valid ones, the JDK refused $refused and read ${count - refused}")
  }
}
