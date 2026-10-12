// lila's translations as its build's resource generator writes them (project/I18n.scala), run by teq as
// the overlay's TeqCommand (overlay/LilaTeq.scala) under `teq interp`: per locale (en-GB, then the
// names of translation/dest/site/*.xml, sorted) the file i18n.<locale>.ser, a java.util.HashMap of the
// keys of every database's file (`site`'s by their name, the others' as `<db>:<name>`) to a string, or
// to a HashMap of a plural's quantities to strings, serialised by java.io.ObjectOutputStream. The
// bytes are the JDK's: the stream format of a HashMap (its class descriptor, its load factor and
// threshold, its table's length and size, then its entries in table order, each string a new handle),
// with the table built as the JDK's HashMap builds it (its hash spread, its doubling resizes, which
// keep a bin's order) and the XML read as the JDK's SAX parser gives scala-xml its text (entities and
// character references decoded, line ends normalised).
//
// teq interp bench/lila/i18n.scala -- <source dir> <dest dir> <output dir> <databases, comma-separated> [<ignored>]
import java.io.{ByteArrayOutputStream, File}
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths}

@main def i18n(args: String*): Unit =
  if args.length < 4 then
    System.err.println("usage: i18n.scala <source dir> <dest dir> <output dir> <databases, comma-separated>")
    sys.exit(2)
  val source = Paths.get(args(0))
  val dest = Paths.get(args(1))
  val output = Paths.get(args(2))
  // The databases; an argument after them (the source directory teq appends to a command) is unread.
  val dbs = args(3).split(',').toList
  val siteFiles = dest.resolve("site").toFile.listFiles.map(_.getName.takeWhile(_ != '.')).sorted.toList
  val locales = "en-GB" :: siteFiles
  Files.createDirectories(output)
  for locale <- locales do
    val map = new JHashMap
    for db <- dbs do
      val file = if locale == "en-GB" then source.resolve(s"$db.xml") else dest.resolve(s"$db/$locale.xml")
      if Files.isRegularFile(file) then
        for e <- Xml.elements(Files.readString(file, UTF_8)) do
          val key = if db == "site" then e.attr("name") else s"$db:${e.attr("name")}"
          e.label match
            case "string" => map.put(key, unescapeQuotes(e.text))
            case "plurals" =>
              val plurals = new JHashMap
              for i <- e.children if i.label == "item" do plurals.put(i.attr("quantity"), unescapeQuotes(i.text))
              map.put(key, plurals)
            case _ => ()
    Files.write(output.resolve(s"i18n.$locale.ser"), Serial.write(map))

def unescapeQuotes(s: String): String = s.replace("\\\"", "\"").replace("\\'", "'")

// An element of the XML subset the translations are written in: its attributes, its text (that of its
// descendants, decoded) and its child elements.
final class Element(val label: String, attrs: List[(String, String)], val text: String, val children: Vector[Element]):
  def attr(name: String): String = attrs.collectFirst { case (n, v) if n == name => v }.getOrElse("")

object Xml:
  // The root element's child elements, the text between them ignored as scala-xml's `child` gives
  // it to project/I18n.scala's match.
  def elements(raw: String): Vector[Element] =
    val text = normaliseLineEnds(if raw.startsWith("﻿") then raw.substring(1) else raw)
    val p = new Parser(text)
    p.skipProlog()
    p.element() match
      case Some(root) => root.children
      case None => Vector.empty

  private def normaliseLineEnds(s: String): String =
    if s.indexOf('\r') < 0 then s else s.replace("\r\n", "\n").replace('\r', '\n')

  def decode(s: String, attribute: Boolean): String =
    val raw = if attribute then s.map(c => if c == '\n' || c == '\t' then ' ' else c) else s
    if raw.indexOf('&') < 0 then raw
    else
      val b = new StringBuilder
      var i = 0
      while i < raw.length do
        val c = raw.charAt(i)
        if c == '&' then
          val end = raw.indexOf(';', i)
          val name = raw.substring(i + 1, end)
          name match
            case "lt" => b.append('<')
            case "gt" => b.append('>')
            case "amp" => b.append('&')
            case "quot" => b.append('"')
            case "apos" => b.append('\'')
            case n if n.startsWith("#x") => appendCodePoint(b, Integer.parseInt(n.substring(2), 16))
            case n if n.startsWith("#") => appendCodePoint(b, Integer.parseInt(n.substring(1)))
            case n => sys.error(s"an entity &$n; the translations do not declare")
          i = end + 1
        else
          b.append(c)
          i += 1
      b.toString

  private def appendCodePoint(b: StringBuilder, cp: Int): Unit =
    if cp < 0x10000 then b.append(cp.toChar)
    else
      b.append((0xd800 + ((cp - 0x10000) >> 10)).toChar)
      b.append((0xdc00 + ((cp - 0x10000) & 0x3ff)).toChar)

final class Parser(s: String):
  private var at = 0

  def skipProlog(): Unit =
    var more = true
    while more do
      skipSpace()
      if s.startsWith("<?", at) then at = s.indexOf("?>", at) + 2
      else if s.startsWith("<!--", at) then at = s.indexOf("-->", at) + 3
      else if s.startsWith("<!DOCTYPE", at) then at = s.indexOf('>', at) + 1
      else more = false

  private def skipSpace(): Unit =
    while at < s.length && Character.isWhitespace(s.charAt(at)) do at += 1

  // The element at `at`, a start tag's `<`.
  def element(): Option[Element] =
    if at >= s.length || s.charAt(at) != '<' then return None
    at += 1
    val nameEnd = nameEndFrom(at)
    val label = s.substring(at, nameEnd)
    at = nameEnd
    var attrs = List.empty[(String, String)]
    skipSpace()
    while s.charAt(at) != '>' && s.charAt(at) != '/' do
      val end = nameEndFrom(at)
      val name = s.substring(at, end)
      at = s.indexOf('=', end) + 1
      skipSpace()
      val quote = s.charAt(at)
      val close = s.indexOf(quote, at + 1)
      attrs = (name, Xml.decode(s.substring(at + 1, close), attribute = true)) :: attrs
      at = close + 1
      skipSpace()
    if s.charAt(at) == '/' then
      at += 2
      Some(Element(label, attrs.reverse, "", Vector.empty))
    else
      at += 1
      val text = new StringBuilder
      val children = Vector.newBuilder[Element]
      var open = true
      while open do
        val lt = s.indexOf('<', at)
        if lt > at then text.append(Xml.decode(s.substring(at, lt), attribute = false))
        at = lt
        if s.startsWith("</", at) then
          at = s.indexOf('>', at) + 1
          open = false
        else if s.startsWith("<!--", at) then at = s.indexOf("-->", at) + 3
        else if s.startsWith("<![CDATA[", at) then
          val end = s.indexOf("]]>", at)
          text.append(s, at + 9, end)
          at = end + 3
        else
          val child = element().get
          text.append(child.text)
          children += child
      Some(Element(label, attrs.reverse, text.toString, children.result()))

  private def nameEndFrom(from: Int): Int =
    var i = from
    while i < s.length && { val c = s.charAt(i); !Character.isWhitespace(c) && c != '>' && c != '/' && c != '=' } do i += 1
    i

// java.util.HashMap's table as the JDK builds it from `new HashMap()` and a sequence of puts: the
// hash spread, the first table of 16, doubling past three quarters full, a resize keeping each half
// of a bin in its order, and a bin of eight in a table under 64 resizing the table. A bin the JDK would
// turn into a tree (eight in a table of 64 or more), whose order follows its red-black tree, is refused.
final class JNode(val hash: Int, val key: String, var value: AnyRef, var next: JNode)

final class JHashMap:
  var table: Array[JNode] = null
  var size = 0
  var threshold = 0

  private def spread(key: String): Int =
    val h = key.hashCode
    h ^ (h >>> 16)

  def put(key: String, value: AnyRef): Unit =
    if table == null then resize()
    val hash = spread(key)
    val i = (table.length - 1) & hash
    var e = table(i)
    if e == null then table(i) = new JNode(hash, key, value, null)
    else
      var bins = 0
      var placed = false
      while !placed do
        if e.hash == hash && e.key == key then
          e.value = value
          return
        if e.next == null then
          e.next = new JNode(hash, key, value, null)
          if bins >= 7 then
            if table.length < 64 then resize()
            else sys.error(s"a bin of the table of ${table.length} holds nine keys, which the JDK makes a tree of")
          placed = true
        else
          e = e.next
          bins += 1
    size += 1
    if size > threshold then resize()

  private def resize(): Unit =
    val old = table
    val oldCap = if old == null then 0 else old.length
    val (newCap, newThr) = if oldCap == 0 then (16, 12) else (oldCap * 2, threshold * 2)
    val fresh = new Array[JNode](newCap)
    var j = 0
    while j < oldCap do
      var e = old(j)
      if e != null then
        if e.next == null then fresh(e.hash & (newCap - 1)) = e
        else
          var loHead: JNode = null
          var loTail: JNode = null
          var hiHead: JNode = null
          var hiTail: JNode = null
          while e != null do
            val next = e.next
            if (e.hash & oldCap) == 0 then
              if loTail == null then loHead = e else loTail.next = e
              loTail = e
            else
              if hiTail == null then hiHead = e else hiTail.next = e
              hiTail = e
            e = next
          if loTail != null then
            loTail.next = null
            fresh(j) = loHead
          if hiTail != null then
            hiTail.next = null
            fresh(j + oldCap) = hiHead
      j += 1
    table = fresh
    threshold = newThr

// java.io.ObjectOutputStream's bytes for a HashMap of strings and HashMaps: the stream header, the
// class descriptor (its handle the first), then the object's fields (`loadFactor`, `threshold`), its
// block of the table's length and size and its entries, each string a handle of its own.
object Serial:
  private val SerialVersionUid = 362498820763181265L

  def write(map: JHashMap): Array[Byte] =
    val w = new Writer
    w.short(0xaced)
    w.short(5)
    w.map(map)
    w.bytes

  final class Writer:
    private val out = new ByteArrayOutputStream
    private var handles = 0
    private var descriptor = -1

    def bytes: Array[Byte] = out.toByteArray
    def byte(b: Int): Unit = out.write(b & 0xff)
    def short(v: Int): Unit = { byte(v >> 8); byte(v) }
    def int(v: Int): Unit = { short(v >>> 16); short(v) }
    def long(v: Long): Unit = { int((v >>> 32).toInt); int(v.toInt) }
    private def utf(s: String): Unit =
      val b = modifiedUtf8(s)
      short(b.length)
      out.write(b, 0, b.length)
    private def handle(): Int = { val h = handles; handles += 1; h }

    def map(m: JHashMap): Unit =
      byte(0x73) // TC_OBJECT
      if descriptor < 0 then
        byte(0x72) // TC_CLASSDESC
        utf("java.util.HashMap")
        long(SerialVersionUid)
        byte(0x03) // SC_WRITE_METHOD | SC_SERIALIZABLE
        short(2)
        byte('F'); utf("loadFactor")
        byte('I'); utf("threshold")
        byte(0x78) // TC_ENDBLOCKDATA
        byte(0x70) // TC_NULL, no superclass
        descriptor = handle()
      else
        byte(0x71) // TC_REFERENCE
        int(0x7e0000 + descriptor)
      handle()
      int(java.lang.Float.floatToIntBits(0.75f))
      int(m.threshold)
      byte(0x77) // TC_BLOCKDATA
      byte(8)
      int(if m.table == null then 16 else m.table.length)
      int(m.size)
      if m.table != null then
        for bin <- m.table do
          var e = bin
          while e != null do
            string(e.key)
            e.value match
              case s: String => string(s)
              case n: JHashMap => map(n)
              case other => sys.error(s"no serialisation of $other")
            e = e.next
      byte(0x78) // TC_ENDBLOCKDATA

    private def string(s: String): Unit =
      val b = modifiedUtf8(s)
      if b.length <= 0xffff then
        byte(0x74) // TC_STRING
        short(b.length)
      else
        byte(0x7c) // TC_LONGSTRING
        long(b.length.toLong)
      out.write(b, 0, b.length)
      handle()

  // The JDK's modified UTF-8: U+0000 as two bytes, a supplementary character as its two surrogates.
  def modifiedUtf8(s: String): Array[Byte] =
    val b = new ByteArrayOutputStream(s.length + 16)
    var i = 0
    while i < s.length do
      val c = s.charAt(i).toInt
      if c >= 1 && c <= 0x7f then b.write(c)
      else if c <= 0x7ff then
        b.write(0xc0 | (c >> 6))
        b.write(0x80 | (c & 0x3f))
      else
        b.write(0xe0 | (c >> 12))
        b.write(0x80 | ((c >> 6) & 0x3f))
        b.write(0x80 | (c & 0x3f))
      i += 1
    b.toByteArray
