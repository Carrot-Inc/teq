// Indigo's font generator (indigo-plugin's generators/FontGen.scala) for bench/indigo/overlay/generate.scala, its
// layouts and its FontInfo source as the plugin writes them, with the plugin's AWT helper (`FontAWTHelper`), which
// needs a JVM, replaced by a reader of the TrueType file in Scala: the metrics as FreeType gives them to AWT
// (`hhea`'s ascender, descender and line gap and `hdmx`'s or `hmtx`'s advances at the size; FontDesignMetrics's
// rounding), the glyphs' outlines from `glyf` filled with the non-zero rule at the pixels' centres (the plugin's
// `noAntiAliasing`; an anti-aliased sheet is drawn the same way), and the sheet written as a PNG of stored deflate
// blocks into `imageOut`, the build's assets, as the plugin writes it (teq.mill.json declares it among the
// generator's outputs).
package indigoplugin.generators

import indigoplugin.FontOptions
import indigoplugin.FontLayout
import indigoplugin.IndigoGenerators
import scala.annotation.tailrec

object FontGen {
  private val CharBatchSize = 2048

  def generate(
      moduleName: String,
      fullyQualifiedPackage: String,
      fontFilePath: os.Path,
      fontOptions: FontOptions,
      imageOut: os.Path
  ): IndigoGenerators.SourceParams => Seq[os.Path] = params => {

    // Some director sanity checking...
    if (!os.exists(imageOut)) {
      throw new Exception(
        s"The supplied path to the output directory for the font-sheet image does not exist: ${imageOut.toString}"
      )
    } else if (!os.isDir(imageOut)) {
      throw new Exception(
        s"The supplied path to the output directory for the font-sheet image is not a directory: ${imageOut.toString}"
      )
    } else {
      //
    }

    val wd = params.destination / Generators.OutputDirName
    os.makeDir.all(wd)
    val file = wd / s"$moduleName.scala"

    val helper = TrueTypeHelper.makeHelper(fontFilePath, fontOptions.fontSize)

    val initialCharDetails =
      fontOptions.charSet.toCharacterCodes.map { c =>
        val (w, h, a) = helper.getCharBounds(c.toChar)
        CharDetail(c.toChar, c, 0, 0, w, h, a)
      }.toList

    val filteredCharDetails = initialCharDetails.filter(cd => filterUnsupportedChars(cd.char, cd.code))

    val charDetails =
      fontOptions.layout match {
        case FontLayout.Normal(maxCharactersPerLine) =>
          val (cellWidth, cellHeight) =
            filteredCharDetails.foldLeft((0, 0)) { case ((w, h), c) =>
              (if (c.width > w) c.width else w, if (c.height > h) c.height else h)
            }
          normalLayout(filteredCharDetails, maxCharactersPerLine, cellWidth, cellHeight)
        case FontLayout.Monospace(maxCharactersPerLine, cellWidth, cellHeight) =>
          monospaceLayout(filteredCharDetails, helper.getMaxAscent, maxCharactersPerLine, cellWidth, cellHeight)
        case FontLayout.IndexedGrid(maxCharactersPerLine, cellWidth, cellHeight) =>
          indexedGridLayout(filteredCharDetails, helper.getMaxAscent, maxCharactersPerLine, cellWidth, cellHeight)
      }

    val default =
      charDetails
        .find(_.char == fontOptions.charSet.default)
        .getOrElse(throw new Exception(s"Couldn't find default character '${fontOptions.charSet.default.toString}'"))

    val (sheetWidth, sheetHeight) =
      fontOptions.layout match {
        case FontLayout.Normal(_) => findFontSheetBounds(charDetails)
        case l @ FontLayout.Monospace(_, _, _) => (l.fontSheetWidth, l.fontSheetHeight(charDetails.length))
        case l @ FontLayout.IndexedGrid(_, _, _) => (l.fontSheetWidth, l.fontSheetHeight(charDetails.map(_.code)))
      }

    os.write.over(file, genFontInfo(moduleName, fullyQualifiedPackage, fontOptions.fontKey, default, charDetails))

    // Write out font image
    val outImageFileName = s"$moduleName.png"

    helper.drawFontSheet(imageOut / outImageFileName, charDetails, sheetWidth, sheetHeight, fontOptions)

    Seq(file)
  }

  def sanitiseName(name: String, ext: String): String = {
    val noExt = if (ext.nonEmpty && name.endsWith(ext)) name.dropRight(ext.length) else name
    noExt.replaceAll("[^A-Za-z0-9]", "-").split("-").map(_.capitalize).mkString
  }

  def normalLayout(unplacedChars: List[CharDetail], maxCharsPerLine: Int, cellWidth: Int, cellHeight: Int): List[CharDetail] = {
    @tailrec
    def rec(remaining: List[CharDetail], lineCount: Int, charCount: Int, nextX: Int, acc: List[CharDetail]): List[CharDetail] =
      remaining match {
        case Nil => acc.reverse
        case c :: cs if charCount == maxCharsPerLine => rec(c :: cs, lineCount + 1, 0, 0, acc)
        case c :: cs =>
          val newC = c.copy(x = nextX, y = lineCount * cellHeight)
          rec(cs, lineCount, charCount + 1, nextX + cellWidth, newC :: acc)
      }
    rec(unplacedChars, 0, 0, 0, Nil)
  }

  def monospaceLayout(unplacedChars: List[CharDetail], maxAscent: Int, maxCharsPerLine: Int, cellWidth: Int, cellHeight: Int): List[CharDetail] = {
    @tailrec
    def rec(remaining: List[CharDetail], lineCount: Int, charCount: Int, nextX: Int, acc: List[CharDetail]): List[CharDetail] =
      remaining match {
        case Nil => acc.reverse
        case c :: cs if charCount == maxCharsPerLine => rec(c :: cs, lineCount + 1, 0, 0, acc)
        case c :: cs =>
          val x = nextX + ((cellWidth - c.width) / 2)
          val y = lineCount * cellHeight - (cellHeight - maxAscent)
          rec(cs, lineCount, charCount + 1, nextX + cellWidth, c.copy(x = x, y = y) :: acc)
      }
    rec(unplacedChars, 0, 0, 0, Nil)
  }

  def indexedGridLayout(unplacedChars: List[CharDetail], maxAscent: Int, maxCharsPerLine: Int, cellWidth: Int, cellHeight: Int): List[CharDetail] =
    unplacedChars.map { c =>
      val x = (c.code % maxCharsPerLine) * cellWidth
      val y = ((c.code / maxCharsPerLine) * cellHeight) - (cellHeight - maxAscent)
      c.copy(x = x, y = y)
    }

  def findFontSheetBounds(charDetails: List[CharDetail]): (Int, Int) =
    charDetails.foldLeft((0, 0)) { case ((w, h), c) =>
      (if (c.x + c.width > w) c.x + c.width else w, if (c.y + c.height > h) c.y + c.height else h)
    }

  def genFontInfo(moduleName: String, fullyQualifiedPackage: String, name: String, default: CharDetail, chars: List[CharDetail]): String = {
    val charBatches = chars
      .sliding(CharBatchSize, CharBatchSize)
      .zipWithIndex
      .map { case (batch, index) =>
        val chars = batch.map(cd => "    " + toFontChar(cd) + ",").mkString("\n").dropRight(1)
        s"""private object ${moduleName}CharBatch$index {
           |  val batch = Batch(
           |${chars}
           |  )
           |}
           |""".stripMargin
      }
      .mkString("\n")
    val charBatchAdditions = (0 until (chars.length / CharBatchSize) + 1)
      .map(index => s"      .addChars(${moduleName}CharBatch${index}.batch)")
      .mkString("\n")
    val dx = default.x.toString
    val dy = default.y.toString
    val dw = default.width.toString
    val dh = default.height.toString
    s"""package $fullyQualifiedPackage
    |
    |import indigo.*
    |
    |// DO NOT EDIT: Generated by Indigo.
    |object $moduleName {
    |
    |  val fontKey: FontKey = FontKey("$name")
    |
    |  val fontInfo: FontInfo =
    |    FontInfo(
    |      fontKey,
    |      FontChar("${default.char.toString()}", $dx, $dy, $dw, $dh)
    |    ).isCaseSensitive
    |${charBatchAdditions}
    |}
    |
    |$charBatches
    |
    |""".stripMargin
  }

  def toFontChar(charDetail: CharDetail): String =
    s"""FontChar("${escapeChar(charDetail.char)}", ${charDetail.x}, ${charDetail.y}, ${charDetail.width}, ${charDetail.height})"""

  def filterUnsupportedChars(c: Char, code: Int): Boolean =
    c match {
      case '\n' | '\t' | '\b' | '\r' | '\f' => false
      case _ if code == 0 || code == 26 => false
      case _ => true
    }

  def escapeChar(c: Char): String =
    c match {
      case '\\' => "\\\\"
      case '\"' => "\\\""
      case '\'' => "\\'"
      case _    => c.toString
    }
}

final case class CharDetail(char: Char, code: Int, x: Int, y: Int, width: Int, height: Int, ascent: Int)

/** A TrueType font's metrics and outlines, read from its tables. */
final class TrueType(bytes: Array[Byte]) {
  private def u8(o: Int): Int = bytes(o) & 0xff
  private def u16(o: Int): Int = (u8(o) << 8) | u8(o + 1)
  private def i16(o: Int): Int = (u16(o) << 16) >> 16
  private def i8(o: Int): Int = bytes(o).toInt
  private def u32(o: Int): Int = (u16(o) << 16) | u16(o + 2)
  private def f2dot14(o: Int): Double = i16(o) / 16384.0

  private val tables: Map[String, Int] =
    (0 until u16(4)).map { i =>
      val record = 12 + 16 * i
      new String(bytes, record, 4, "ISO-8859-1") -> u32(record + 8)
    }.toMap
  private def table(name: String): Int = tables.getOrElse(name, throw new Exception(s"the font has no $name table"))

  val unitsPerEm: Int = u16(table("head") + 18)
  private val longLoca: Boolean = i16(table("head") + 50) == 1
  val ascender: Int = i16(table("hhea") + 4)
  val descender: Int = i16(table("hhea") + 6)
  val lineGap: Int = i16(table("hhea") + 8)
  private val hMetrics: Int = u16(table("hhea") + 34)

  def advance(glyph: Int): Int = u16(table("hmtx") + 4 * math.min(glyph, hMetrics - 1))

  // The advance at a size in pixels from `hdmx`, the font's record of the advances its instructions give there:
  // what AWT's text without anti-aliasing measures, FreeType's hinted load (ttgload.c's `compute_glyph_metrics`
  // reads the table, or runs the instructions for a font of fixed pitch, which this reader does not); none without
  // the table or a record of the size.
  def deviceAdvance(glyph: Int, ppem: Int): Option[Int] =
    tables.get("hdmx").flatMap { base =>
      val size = u32(base + 4)
      (0 until i16(base + 2)).map(r => base + 8 + r * size).find(p => u8(p) == ppem).map(p => u8(p + 2 + glyph))
    }

  // The glyph of a character: the cmap's Windows Unicode, Unicode or Macintosh Roman subtable, of format 4, 12, 6 or 0.
  private val cmap: Int = {
    val base = table("cmap")
    val subtables = (0 until u16(base + 2)).map(i => (u16(base + 4 + 8 * i), u16(base + 6 + 8 * i), base + u32(base + 8 + 8 * i)))
    val preferred = Seq((3, 10), (3, 1), (0, 4), (0, 3), (0, 1), (0, 0), (1, 0))
    preferred.flatMap(p => subtables.find(s => (s._1, s._2) == p)).headOption.map(_._3)
      .getOrElse(throw new Exception("the font has no cmap subtable for Unicode"))
  }

  def glyphIndex(code: Int): Int = u16(cmap) match {
    case 0 => if (code < 256) u8(cmap + 6 + code) else 0
    case 6 =>
      val first = u16(cmap + 6)
      val count = u16(cmap + 8)
      if (code >= first && code < first + count) u16(cmap + 10 + 2 * (code - first)) else 0
    case 4 =>
      val segments = u16(cmap + 6) / 2
      val ends = cmap + 14
      val starts = ends + 2 * segments + 2
      val deltas = starts + 2 * segments
      val offsets = deltas + 2 * segments
      (0 until segments).find(i => u16(ends + 2 * i) >= code) match {
        case Some(i) if u16(starts + 2 * i) <= code =>
          val delta = u16(deltas + 2 * i)
          val offset = u16(offsets + 2 * i)
          if (offset == 0) (code + delta) & 0xffff
          else {
            val g = u16(offsets + 2 * i + offset + 2 * (code - u16(starts + 2 * i)))
            if (g == 0) 0 else (g + delta) & 0xffff
          }
        case _ => 0
      }
    case 12 =>
      val groups = u32(cmap + 12)
      (0 until groups).map(i => cmap + 16 + 12 * i).find(g => u32(g) <= code && code <= u32(g + 4))
        .map(g => u32(g + 8) + code - u32(g)).getOrElse(0)
    case other => throw new Exception(s"cmap format $other is not read")
  }

  private def glyphRange(glyph: Int): (Int, Int) = {
    val loca = table("loca")
    if (longLoca) (u32(loca + 4 * glyph), u32(loca + 4 * glyph + 4))
    else (2 * u16(loca + 2 * glyph), 2 * u16(loca + 2 * glyph + 2))
  }

  /** The glyph's contours in font units, each point with whether it is on the curve. */
  def contours(glyph: Int, depth: Int = 0): Vector[Vector[(Double, Double, Boolean)]] = {
    val (start, end) = glyphRange(glyph)
    if (start == end || depth > 8) return Vector.empty
    val at = table("glyf") + start
    val count = i16(at)
    if (count >= 0) simple(at, count) else composite(at, depth)
  }

  private def simple(at: Int, count: Int): Vector[Vector[(Double, Double, Boolean)]] = {
    val ends = (0 until count).map(i => u16(at + 10 + 2 * i))
    val points = if (ends.isEmpty) 0 else ends.last + 1
    var pos = at + 10 + 2 * count
    pos += 2 + u16(pos)
    val flags = new Array[Int](points)
    var i = 0
    while (i < points) {
      val f = u8(pos); pos += 1
      flags(i) = f; i += 1
      if ((f & 8) != 0) {
        var repeat = u8(pos); pos += 1
        while (repeat > 0) { flags(i) = f; i += 1; repeat -= 1 }
      }
    }
    val xs = new Array[Int](points)
    var x = 0
    for (k <- 0 until points) {
      val f = flags(k)
      if ((f & 2) != 0) { val d = u8(pos); pos += 1; x += (if ((f & 16) != 0) d else -d) }
      else if ((f & 16) == 0) { x += i16(pos); pos += 2 }
      xs(k) = x
    }
    val ys = new Array[Int](points)
    var y = 0
    for (k <- 0 until points) {
      val f = flags(k)
      if ((f & 4) != 0) { val d = u8(pos); pos += 1; y += (if ((f & 32) != 0) d else -d) }
      else if ((f & 32) == 0) { y += i16(pos); pos += 2 }
      ys(k) = y
    }
    var first = 0
    ends.toVector.map { last =>
      val contour = (first to last).map(k => (xs(k).toDouble, ys(k).toDouble, (flags(k) & 1) != 0)).toVector
      first = last + 1
      contour
    }
  }

  private def composite(at: Int, depth: Int): Vector[Vector[(Double, Double, Boolean)]] = {
    var pos = at + 10
    var more = true
    val out = Vector.newBuilder[Vector[(Double, Double, Boolean)]]
    while (more) {
      val flags = u16(pos)
      val component = u16(pos + 2)
      pos += 4
      val (dx, dy) =
        if ((flags & 1) != 0) { val r = (i16(pos), i16(pos + 2)); pos += 4; r }
        else { val r = (i8(pos), i8(pos + 1)); pos += 2; r }
      var (a, b, c, d) = (1.0, 0.0, 0.0, 1.0)
      if ((flags & 8) != 0) { a = f2dot14(pos); d = a; pos += 2 }
      else if ((flags & 0x40) != 0) { a = f2dot14(pos); d = f2dot14(pos + 2); pos += 4 }
      else if ((flags & 0x80) != 0) { a = f2dot14(pos); b = f2dot14(pos + 2); c = f2dot14(pos + 4); d = f2dot14(pos + 6); pos += 8 }
      // Arguments that are point numbers rather than offsets place nothing here: the component stays where it is.
      val (ox, oy) = if ((flags & 2) != 0) (dx.toDouble, dy.toDouble) else (0.0, 0.0)
      for (contour <- contours(component, depth + 1))
        out += contour.map { case (x, y, on) => (a * x + c * y + ox, b * x + d * y + oy, on) }
      more = (flags & 0x20) != 0
    }
    out.result()
  }
}

final case class TrueTypeHelper(font: TrueType, size: Int) {
  private val scale: Double = size.toDouble / font.unitsPerEm
  // FontDesignMetrics's integers over FreeType's metrics: the ascent and descent rounded up past 0.05.
  private val ascentF: Double = font.ascender * scale
  private val descentF: Double = -font.descender * scale
  private val leadingF: Double = font.lineGap * scale
  private val ascent: Int = (0.95 + ascentF).toInt
  private val height: Int = ascent + (0.95 + descentF + leadingF).toInt

  // The hinted advance: `hdmx`'s at the size, else the scaled advance rounded (the phantom points' grid fitting).
  def getCharBounds(char: Char): (Int, Int, Int) =
    val glyph = font.glyphIndex(char.toInt)
    (font.deviceAdvance(glyph, size).getOrElse(math.round(font.advance(glyph) * scale).toInt), height, ascent)

  def getMaxAscent: Int = ascent

  /** The sheet as AWT draws it: each character's glyph at its place, its baseline `ascent` below its top. */
  def drawFontSheet(out: os.Path, charDetails: List[CharDetail], sheetWidth: Int, sheetHeight: Int, fontOptions: FontOptions): Unit = {
    val w = math.max(1, sheetWidth)
    val h = math.max(1, sheetHeight)
    val pixels = new Array[Byte](w * h * 4)
    val (r, g, b) = ((fontOptions.color.r * 255).toInt, (fontOptions.color.g * 255).toInt, (fontOptions.color.b * 255).toInt)
    for (c <- charDetails) {
      val edges = outlineEdges(font.contours(font.glyphIndex(c.code)), c.x.toDouble, (c.y + c.ascent).toDouble)
      fill(edges, w, h) { (x, y) =>
        val i = (y * w + x) * 4
        pixels(i) = r.toByte; pixels(i + 1) = g.toByte; pixels(i + 2) = b.toByte; pixels(i + 3) = 255.toByte
      }
    }
    os.write.over(out, Png.rgba(w, h, pixels))
  }

  // The contours as line segments in the sheet's pixels: quadratic arcs between on-curve points, an implied
  // on-curve point between two off-curve ones, each arc cut into eight segments.
  private def outlineEdges(contours: Vector[Vector[(Double, Double, Boolean)]], originX: Double, baseline: Double): Vector[(Double, Double, Double, Double)] = {
    val edges = Vector.newBuilder[(Double, Double, Double, Double)]
    def px(x: Double): Double = originX + x * scale
    def py(y: Double): Double = baseline - y * scale
    for (contour <- contours if contour.nonEmpty) {
      val n = contour.length
      val startIndex = contour.indexWhere(_._3)
      val (sx, sy) =
        if (startIndex >= 0) (contour(startIndex)._1, contour(startIndex)._2)
        else ((contour(0)._1 + contour(n - 1)._1) / 2, (contour(0)._2 + contour(n - 1)._2) / 2)
      val from = if (startIndex >= 0) startIndex else 0
      var (cx, cy) = (sx, sy)
      var control: Option[(Double, Double)] = None
      def line(x: Double, y: Double): Unit = { edges += ((px(cx), py(cy), px(x), py(y))); cx = x; cy = y }
      def curve(qx: Double, qy: Double, x: Double, y: Double): Unit = {
        val (x0, y0) = (cx, cy)
        for (k <- 1 to 8) {
          val t = k / 8.0
          val mt = 1 - t
          line(mt * mt * x0 + 2 * mt * t * qx + t * t * x, mt * mt * y0 + 2 * mt * t * qy + t * t * y)
        }
      }
      // From the point after the start round to it; a contour of off-curve points alone starts between its last and
      // first, each of its points a control.
      val sequence = if (startIndex >= 0) (1 to n).map(k => contour((from + k) % n)) else contour
      for ((x, y, on) <- sequence) {
        (on, control) match {
          case (true, None) => line(x, y)
          case (true, Some((qx, qy))) => curve(qx, qy, x, y); control = None
          case (false, None) => control = Some((x, y))
          case (false, Some((qx, qy))) =>
            val (mx, my) = ((qx + x) / 2, (qy + y) / 2)
            curve(qx, qy, mx, my)
            control = Some((x, y))
        }
      }
      control match {
        case Some((qx, qy)) => curve(qx, qy, sx, sy)
        case None => if (cx != sx || cy != sy) line(sx, sy)
      }
    }
    edges.result()
  }

  // The non-zero rule at each pixel's centre, row by row.
  private def fill(edges: Vector[(Double, Double, Double, Double)], w: Int, h: Int)(set: (Int, Int) => Unit): Unit =
    for (row <- 0 until h) {
      val yc = row + 0.5
      val crossings = edges.flatMap { case (x0, y0, x1, y1) =>
        if (y0 <= yc && yc < y1) Some((x0 + (yc - y0) * (x1 - x0) / (y1 - y0), 1))
        else if (y1 <= yc && yc < y0) Some((x0 + (yc - y0) * (x1 - x0) / (y1 - y0), -1))
        else None
      }.sortBy(_._1)
      var winding = 0
      var k = 0
      while (k < crossings.length) {
        val (x, dir) = crossings(k)
        val before = winding
        winding += dir
        if (before == 0 && winding != 0) {
          // The span to the crossing where the winding comes back to zero.
          var j = k + 1
          var wnd = winding
          while (j < crossings.length && wnd != 0) { wnd += crossings(j)._2; if (wnd != 0) j += 1 }
          val end = if (j < crossings.length) crossings(j)._1 else w.toDouble
          var col = math.max(0, math.ceil(x - 0.5).toInt)
          while (col < w && col + 0.5 < end) { set(col, row); col += 1 }
          winding = 0
          k = j + 1
        } else k += 1
      }
    }
}

object TrueTypeHelper {
  def makeHelper(fontFile: os.Path, fontSize: Int): TrueTypeHelper = TrueTypeHelper(new TrueType(os.read.bytes(fontFile)), fontSize)
}

/** A PNG of 8-bit RGBA, its zlib stream of stored deflate blocks. */
object Png {
  private lazy val crcTable: Array[Int] = Array.tabulate(256) { n =>
    var c = n
    for (_ <- 0 until 8) c = if ((c & 1) != 0) 0xedb88320 ^ (c >>> 1) else c >>> 1
    c
  }
  private def crc(bytes: Array[Byte], from: Int, until: Int): Int = {
    var c = 0xffffffff
    var i = from
    while (i < until) { c = crcTable((c ^ bytes(i)) & 0xff) ^ (c >>> 8); i += 1 }
    c ^ 0xffffffff
  }
  private def int32(v: Int): Array[Byte] = Array((v >>> 24).toByte, (v >>> 16).toByte, (v >>> 8).toByte, v.toByte)
  private def chunk(kind: String, data: Array[Byte]): Array[Byte] = {
    val typed = kind.getBytes("ISO-8859-1") ++ data
    int32(data.length) ++ typed ++ int32(crc(typed, 0, typed.length))
  }

  def rgba(width: Int, height: Int, pixels: Array[Byte]): Array[Byte] = {
    val stride = width * 4
    val raw = new Array[Byte](height * (stride + 1))
    for (y <- 0 until height) System.arraycopy(pixels, y * stride, raw, y * (stride + 1) + 1, stride)
    val blocks = raw.grouped(65535).toVector
    val zlib = Array[Byte](0x78, 0x01) ++ blocks.zipWithIndex.flatMap { case (block, i) =>
      val last: Byte = if (i == blocks.length - 1) 1 else 0
      Array(last, block.length.toByte, (block.length >>> 8).toByte, (~block.length).toByte, ((~block.length) >>> 8).toByte) ++ block
    } ++ int32(adler32(raw))
    val header = int32(width) ++ int32(height) ++ Array[Byte](8, 6, 0, 0, 0)
    Array[Byte](0x89.toByte, 'P', 'N', 'G', 0x0d, 0x0a, 0x1a, 0x0a) ++ chunk("IHDR", header) ++ chunk("IDAT", zlib) ++ chunk("IEND", Array.empty[Byte])
  }

  private def adler32(bytes: Array[Byte]): Int = {
    var a = 1
    var b = 0
    for (x <- bytes) { a = (a + (x & 0xff)) % 65521; b = (b + a) % 65521 }
    (b << 16) | a
  }
}

/** The plugin's Aseprite embedding reads its JSON through circe, which no module of the pin asks for. */
object EmbedAseprite {
  def generate(moduleName: String, fullyQualifiedPackage: String, filePath: os.Path): IndigoGenerators.SourceParams => Seq[os.Path] =
    _ => throw new UnsupportedOperationException(s"embedAseprite is not run under teq interp ($filePath)")
}
