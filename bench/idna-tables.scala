//> using file ../tools/script
// Writes src/native/idn2/idna_tables.h, the tables of the native target's IDNA library (src/native/idn2/idna.c), from
// the Unicode Character Database. The library answers as GNU libidn2 2.3.8 does, and that release pairs the IDNA data
// of Unicode 15.1.0 (UTS #46's mapping table, and RFC 5892's derived property computed from that version's
// characters) with the character properties of Unicode 16.0.0 (general category, combining class, decompositions,
// bidi class, joining type). Every code point 16.0.0 added is disallowed by the 15.1.0 mapping table, so the pairing
// shows only where 16.0.0 corrected a property of an older character. Run from the repository's root:
//
//   ./teq interp bench/idna-tables.scala -- [--ucd <dir>] [--check]
//
// The data files are pinned below by URL and SHA-256 and read from <dir>/<version>/<file>, out/idna-tables by default;
// a file not there is fetched into it with curl, and every file, found or fetched, is held to its digest first.
// --check writes nothing and fails (status 1) where the tree's header is not the bytes the data gives.
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths, StandardCopyOption}
import scala.collection.mutable

final case class Source(version: String, name: String, url: String, sha256: String)

val idnaVersion = "15.1.0"
val propsVersion = "16.0.0"
val sources = Vector(
  Source("15.1.0", "UnicodeData.txt", "https://www.unicode.org/Public/15.1.0/ucd/UnicodeData.txt",
    "2fc713e6a31a87c4850a37fe2caffa4218180fadb5de86b43a143ddb4581fb86"),
  Source("15.1.0", "DerivedNormalizationProps.txt", "https://www.unicode.org/Public/15.1.0/ucd/DerivedNormalizationProps.txt",
    "8875dccee2bc1a7c1fe568a3b502a9e78c9e0495afd96b6568b4294d0ed1f7e1"),
  Source("15.1.0", "PropList.txt", "https://www.unicode.org/Public/15.1.0/ucd/PropList.txt",
    "05672956317b6296bc2ec3d6cef1f6452b57ff4f2efc6dc55b0a19277d5fcfd1"),
  Source("15.1.0", "DerivedCoreProperties.txt", "https://www.unicode.org/Public/15.1.0/ucd/DerivedCoreProperties.txt",
    "f55d0db69123431a7317868725b1fcbf1eab6b265d756d1bd7f0f6d9f9ee108b"),
  Source("15.1.0", "HangulSyllableType.txt", "https://www.unicode.org/Public/15.1.0/ucd/HangulSyllableType.txt",
    "8539c7fdb3e8cd9efcc04a420061fa56a74762a3cb60fc9962c26093c8944427"),
  Source("15.1.0", "CaseFolding.txt", "https://www.unicode.org/Public/15.1.0/ucd/CaseFolding.txt",
    "4e55acfdc32825a22e87670e9056a3bf94ad7c5400065778e9e10f8314372bcf"),
  Source("15.1.0", "IdnaMappingTable.txt", "https://www.unicode.org/Public/idna/15.1.0/IdnaMappingTable.txt",
    "402cbd285f1f952fcd0834b63541d54f69d3d8f1b8f8599bf71a1a14935f82c4"),
  Source("16.0.0", "UnicodeData.txt", "https://www.unicode.org/Public/16.0.0/ucd/UnicodeData.txt",
    "ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f"),
  Source("16.0.0", "DerivedNormalizationProps.txt", "https://www.unicode.org/Public/16.0.0/ucd/DerivedNormalizationProps.txt",
    "4d4c03892dea9146d674b686e495df2d55a28d071ac474041d73518f887abddc"),
  Source("16.0.0", "DerivedBidiClass.txt", "https://www.unicode.org/Public/16.0.0/ucd/extracted/DerivedBidiClass.txt",
    "71ed943a49c58568d8d92e80ecc2ba2f06e62aee9c8ebb0e6e8bd2c3ed8b180e"),
  Source("16.0.0", "DerivedJoiningType.txt", "https://www.unicode.org/Public/16.0.0/ucd/extracted/DerivedJoiningType.txt",
    "6bd08b97da66b70ccfdab105a352de2984e02625239ec5695422c99b33d854f0"),
)
val header = Paths.get("src/native/idn2/idna_tables.h")
val codePoints = 0x110000

// The fields of a record's bits, in idna.c's order; a bidi class outside `bidiClasses` is 11.
val statuses = Vector("valid", "ignored", "mapped", "deviation", "disallowed", "disallowed_STD3_valid", "disallowed_STD3_mapped")
val bidiClasses = Vector("L", "R", "AL", "AN", "EN", "ES", "CS", "ET", "ON", "BN", "NSM")
val bidiLong = Map("Left_To_Right" -> "L", "Right_To_Left" -> "R", "Arabic_Letter" -> "AL", "European_Terminator" -> "ET",
  "Boundary_Neutral" -> "BN")
val joiningTypes = Vector("U", "C", "D", "L", "R", "T")
val pvalid = 0
val contextj = 1
val contexto = 2
val disallowed = 3
val unassigned = 4
// Cn first: a code point UnicodeData.txt does not name is unassigned.
val categories = Vector("Cn", "Lu", "Ll", "Lt", "Lm", "Lo", "Mn", "Mc", "Me", "Nd", "Nl", "No", "Pc", "Pd", "Ps", "Pe",
  "Pi", "Pf", "Po", "Sm", "Sc", "Sk", "So", "Zs", "Zl", "Zp", "Cc", "Cf", "Cs", "Co")
val letterDigitMark = categories.map(Set("Ll", "Lu", "Lo", "Nd", "Lm", "Mn", "Mc").contains).toArray
val marks = categories.map(Set("Mn", "Mc", "Me").contains).toArray

def sha256(bytes: Array[Byte]): String =
  java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))

/// The lines of a data file under `ucd`, fetched there when absent; a file of another digest ends the run.
def lines(ucd: Path, version: String, name: String): Array[String] =
  val source = sources.find(s => s.version == version && s.name == name).get
  val file = ucd.resolve(version).resolve(name)
  def verified(path: Path): Array[Byte] =
    val bytes = Files.readAllBytes(path)
    val found = sha256(bytes)
    if found != source.sha256 then Log.die(s"$path has SHA-256 $found where the script pins ${source.sha256} (${source.url})")
    bytes
  if !Files.isRegularFile(file) then
    Files.createDirectories(file.getParent)
    val part = file.resolveSibling(name + ".part")
    val r = Sh("curl", "-fsSL", "--max-time", "300", "-o", part.toString, source.url).timeout(320).check(false).run()
    if !r.ok then Log.die(s"curl ${source.url}: exit ${r.status}\n${r.errTail}")
    verified(part)
    Files.move(part, file, StandardCopyOption.REPLACE_EXISTING)
  new String(verified(file), UTF_8).split("\n")

/// Each record of a data file: its first and last code point and its other fields, comments and blank lines left out.
def records(lines: Array[String])(each: (Int, Int, Array[String]) => Unit): Unit =
  for line <- lines do
    val hash = line.indexOf('#')
    val body = (if hash < 0 then line else line.substring(0, hash)).trim
    if body.nonEmpty then
      val f = body.split(";", -1).map(_.trim)
      val dots = f(0).indexOf("..")
      val first = Integer.parseInt(if dots < 0 then f(0) else f(0).substring(0, dots), 16)
      val last = if dots < 0 then first else Integer.parseInt(f(0).substring(dots + 2), 16)
      each(first, last, f.drop(1))

def codes(text: String): Array[Int] = text.split(' ').filter(_.nonEmpty).map(Integer.parseInt(_, 16))

def die(why: String): Nothing = Log.die(s"idna-tables: $why")

/// What one version's UnicodeData.txt and DerivedNormalizationProps.txt give: general category (an index of
/// `categories`), combining class, the one-step decompositions, and the canonical pairs that compose.
class Characters(ucd: Path, version: String):
  val category = new Array[Byte](codePoints)
  val combining = new Array[Int](codePoints)
  val canonical = new Array[Array[Int]](codePoints)
  val compatibility = new Array[Array[Int]](codePoints)
  /// The code points with a canonical decomposition, ascending.
  val decomposed = mutable.ArrayBuffer.empty[Int]
  /// (first << 21 | second) -> the composite.
  val pairs = mutable.HashMap.empty[Long, Int]

  locally {
    var first = 0
    for line <- lines(ucd, version, "UnicodeData.txt") do
      val f = line.split(";", -1)
      val cp = Integer.parseInt(f(0), 16)
      if f(1).endsWith(", First>") then first = cp
      else
        val from = if f(1).endsWith(", Last>") then first else cp
        val gc = categories.indexOf(f(2))
        if gc < 0 then die(s"$version: U+${f(0)} has the general category ${f(2)}")
        val ccc = f(3).toInt
        var c = from
        while c <= cp do
          category(c) = gc.toByte
          combining(c) = ccc
          c += 1
        if f(5).nonEmpty then
          if f(5).startsWith("<") then compatibility(cp) = codes(f(5).substring(f(5).indexOf('>') + 1))
          else
            canonical(cp) = codes(f(5))
            decomposed += cp
    val excluded = new Array[Boolean](codePoints)
    records(lines(ucd, version, "DerivedNormalizationProps.txt")) { (a, b, f) =>
      if f(0) == "Full_Composition_Exclusion" then
        var c = a
        while c <= b do
          excluded(c) = true
          c += 1
    }
    for c <- decomposed do
      val d = canonical(c)
      if d.length == 2 && !excluded(c) then pairs(d(0).toLong << 21 | d(1)) = c
  }

  // ---- normalization, as UAX #15 defines it

  val sBase = 0xAC00
  val lBase = 0x1100
  val vBase = 0x1161
  val tBase = 0x11A7
  val lCount = 19
  val vCount = 21
  val tCount = 28
  val sCount = lCount * vCount * tCount

  def decompose(s: Seq[Int], compat: Boolean): mutable.ArrayBuffer[Int] =
    val out = mutable.ArrayBuffer.empty[Int]
    def put(c: Int): Unit =
      if sBase <= c && c < sBase + sCount then
        val i = c - sBase
        out += lBase + i / (vCount * tCount)
        out += vBase + (i % (vCount * tCount)) / tCount
        if i % tCount != 0 then out += tBase + i % tCount
      else if canonical(c) != null then canonical(c).foreach(put)
      else if compat && compatibility(c) != null then compatibility(c).foreach(put)
      else out += c
    s.foreach(put)
    // canonical order: a stable sort of each run of non-starters by combining class
    var i = 0
    while i < out.length do
      if combining(out(i)) == 0 then i += 1
      else
        var j = i
        while j < out.length && combining(out(j)) != 0 do j += 1
        var k = i + 1
        while k < j do
          val c = out(k)
          var m = k
          while m > i && combining(out(m - 1)) > combining(c) do
            out(m) = out(m - 1)
            m -= 1
          out(m) = c
          k += 1
        i = j
    out

  def compose(s: Seq[Int]): mutable.ArrayBuffer[Int] =
    val out = mutable.ArrayBuffer.empty[Int]
    var starter = -1
    var lastClass = -1
    for c <- s do
      val k = combining(c)
      var composed = -1
      if starter >= 0 && !(lastClass != -1 && lastClass >= k) then
        val a = out(starter)
        if lBase <= a && a < lBase + lCount && vBase <= c && c < vBase + vCount then
          composed = sBase + ((a - lBase) * vCount + (c - vBase)) * tCount
        else if sBase <= a && a < sBase + sCount && (a - sBase) % tCount == 0 && tBase < c && c < tBase + tCount then
          composed = a + (c - tBase)
        else composed = pairs.getOrElse(a.toLong << 21 | c, -1)
      if composed >= 0 then out(starter) = composed
      else
        if k == 0 then
          starter = out.length
          lastClass = -1
        else lastClass = k
        out += c
    out

  def nfkc(s: Seq[Int]): mutable.ArrayBuffer[Int] = compose(decompose(s, compat = true))

/// What UTS #46 and RFC 5892 are computed from: the IDNA version's characters, properties, Hangul syllable types,
/// case folding and the mapping table.
final class Idna(ucd: Path) extends Characters(ucd, idnaVersion):
  val noncharacter = new Array[Boolean](codePoints)
  val joinControl = new Array[Boolean](codePoints)
  val defaultIgnorable = new Array[Boolean](codePoints)
  val whiteSpace = new Array[Boolean](codePoints)
  val jamo = new Array[Boolean](codePoints)
  val folding = new Array[Array[Int]](codePoints)
  /// An index of `statuses`.
  val status = new Array[Byte](codePoints)
  val mapping = new Array[Array[Int]](codePoints)

  locally {
    for name <- Vector("PropList.txt", "DerivedCoreProperties.txt") do
      records(lines(ucd, idnaVersion, name)) { (a, b, f) =>
        val set = f(0) match
          case "Noncharacter_Code_Point" => noncharacter
          case "Join_Control" => joinControl
          case "Default_Ignorable_Code_Point" => defaultIgnorable
          case "White_Space" => whiteSpace
          case _ => null
        if set != null then
          var c = a
          while c <= b do
            set(c) = true
            c += 1
      }
    records(lines(ucd, idnaVersion, "HangulSyllableType.txt")) { (a, b, f) =>
      val leadVowelTrail = f(0) == "L" || f(0) == "V" || f(0) == "T"
      var c = a
      while c <= b do
        jamo(c) = leadVowelTrail
        c += 1
    }
    records(lines(ucd, idnaVersion, "CaseFolding.txt")) { (a, _, f) =>
      if f(0) == "C" || f(0) == "F" then folding(a) = codes(f(1))
    }
    val assigned = new Array[Boolean](codePoints)
    var covered = 0
    records(lines(ucd, idnaVersion, "IdnaMappingTable.txt")) { (a, b, f) =>
      val st = statuses.indexOf(f(0))
      if st < 0 then die(s"IdnaMappingTable.txt has the status ${f(0)}")
      val to = if f(0) == "mapped" || f(0) == "disallowed_STD3_mapped" then codes(f(1)) else null
      var c = a
      while c <= b do
        if !assigned(c) then covered += 1
        assigned(c) = true
        status(c) = st.toByte
        if to != null then mapping(c) = to
        c += 1
    }
    if covered != codePoints then die(s"IdnaMappingTable.txt gives a status to ${codePoints - covered} code points fewer than all")
  }

  def casefold(s: Seq[Int]): mutable.ArrayBuffer[Int] =
    val out = mutable.ArrayBuffer.empty[Int]
    for c <- s do
      if folding(c) != null then out ++= folding(c) else out += c
    out

  /// Whether NFKC(casefold(NFKC(c))) is c itself: a code point no decomposition, folding or syllable touches is.
  def stable(c: Int): Boolean =
    if canonical(c) == null && compatibility(c) == null && folding(c) == null && !(sBase <= c && c < sBase + sCount) then true
    else
      val again = nfkc(casefold(nfkc(Seq(c))))
      again.length == 1 && again(0) == c

  /// RFC 5892, section 3: the derived property.
  val exceptions = mutable.HashMap.empty[Int, Int]
  for c <- Vector(0x00DF, 0x03C2, 0x06FD, 0x06FE, 0x0F0B, 0x3007) do exceptions(c) = pvalid
  for c <- Vector(0x00B7, 0x0375, 0x05F3, 0x05F4, 0x30FB) ++ (0x0660 until 0x066A) ++ (0x06F0 until 0x06FA) do exceptions(c) = contexto
  for c <- Vector(0x0640, 0x07FA, 0x302E, 0x302F, 0x303B) ++ (0x3031 until 0x3036) do exceptions(c) = disallowed
  val exceptionsBelow = 0x3100

  def derived(c: Int): Int =
    if c < exceptionsBelow && exceptions.contains(c) then exceptions(c)
    else if category(c) == 0 && !noncharacter(c) then unassigned
    else if c == 0x2D || 0x30 <= c && c <= 0x39 || 0x61 <= c && c <= 0x7A then pvalid
    else if joinControl(c) then contextj
    else if !stable(c) then disallowed
    else if defaultIgnorable(c) || whiteSpace(c) || noncharacter(c) then disallowed
    else if 0x20D0 <= c && c <= 0x20FF || 0x1D100 <= c && c <= 0x1D1FF || 0x1D200 <= c && c <= 0x1D24F then disallowed
    else if jamo(c) then disallowed
    else if letterDigitMark(category(c)) then pvalid
    else disallowed

/// The property version's characters with their bidi classes (an index of `bidiClasses`, else 11) and joining
/// types (an index of `joiningTypes`).
final class Properties(ucd: Path) extends Characters(ucd, propsVersion):
  val bidi = new Array[Byte](codePoints)
  val joining = new Array[Byte](codePoints)

  locally {
    def bidiIndex(name: String): Byte =
      val i = bidiClasses.indexOf(name)
      (if i < 0 then 11 else i).toByte
    def fill(array: Array[Byte], a: Int, b: Int, value: Byte): Unit =
      var c = a
      while c <= b do
        array(c) = value
        c += 1
    val bidiLines = lines(ucd, propsVersion, "DerivedBidiClass.txt")
    // the @missing lines give the defaults of the ranges the records leave out
    val missing = "# @missing:"
    records(bidiLines.filter(_.startsWith(missing)).map(_.substring(missing.length))) { (a, b, f) =>
      fill(bidi, a, b, bidiIndex(bidiLong.getOrElse(f(0), die(s"DerivedBidiClass.txt's @missing names ${f(0)}"))))
    }
    records(bidiLines) { (a, b, f) => fill(bidi, a, b, bidiIndex(f(0))) }
    records(lines(ucd, propsVersion, "DerivedJoiningType.txt")) { (a, b, f) =>
      val j = joiningTypes.indexOf(f(0))
      if j < 0 then die(s"DerivedJoiningType.txt has the joining type ${f(0)}")
      fill(joining, a, b, j.toByte)
    }
  }

/// The runs of equal values: each run's first index and its value.
def runs(values: Array[Int], count: Int): Vector[(Int, Int)] =
  val out = Vector.newBuilder[(Int, Int)]
  var previous = -1
  var c = 0
  while c < count do
    val v = values(c)
    if c == 0 || v != previous then out += ((c, v))
    previous = v
    c += 1
  out.result()

def hex(value: Int, width: Int): String =
  val digits = Integer.toHexString(value)
  "0x" + "0" * (width - digits.length).max(0) + digits

@main def idnaTables(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/idna-tables.scala [--ucd <dir>] [--check]")
  val ucd = Paths.get(args.option("ucd").getOrElse("out/idna-tables")).toAbsolutePath.normalize
  val check = args.flag("check")
  args.exactly(0)
  if !Files.isDirectory(header.getParent) then die("run it from the repository's root")

  val idna = new Idna(ucd)
  val props = new Properties(ucd)

  // status (3 bits), RFC 5892 class (3), bidi class (4), joining type (3), mark (1), virama (1)
  val record = new Array[Int](codePoints)
  var c = 0
  while c < codePoints do
    val derived = if 0xD800 <= c && c <= 0xDFFF then disallowed else idna.derived(c)
    val mark = if marks(props.category(c)) then 1 else 0
    val virama = if props.combining(c) == 9 then 1 else 0
    record(c) = idna.status(c) | derived << 3 | props.bidi(c) << 6 | props.joining(c) << 10 | mark << 13 | virama << 14
    c += 1

  val w = mutable.ArrayBuffer.empty[String]
  def rows(values: IndexedSeq[String], perLine: Int): Unit =
    for i <- 0 until values.length by perLine do w += "  " + values.slice(i, i + perLine).mkString(", ") + ","
  w += "/* Generated by bench/idna-tables.scala from the Unicode Character Database. Do not edit. */"
  w += s"/* IDNA mapping and RFC 5892 classes: Unicode $idnaVersion. Character properties: Unicode $propsVersion. */"
  w += "/* Unicode data files: Copyright (c) Unicode, Inc., under the Unicode License V3, whose text is in NOTICE. */"
  w += ""

  // 1. properties: runs of equal records, each run a start code point and an index of a distinct record
  val recordRuns = runs(record, codePoints)
  val seen = new Array[Boolean](1 << 15)
  for (_, v) <- recordRuns do seen(v) = true
  val distinct = (0 until seen.length).filter(seen(_))
  if distinct.length > 2048 then die(s"${distinct.length} distinct records, more than 11 bits index")
  val indexOf = new Array[Int](1 << 15)
  for (v, i) <- distinct.zipWithIndex do indexOf(v) = i
  w += s"#define PROP_RUNS ${recordRuns.length}"
  w += "/* (first code point << 11) | index into prop_values */"
  w += "static const uint32_t prop_runs[PROP_RUNS] = {"
  rows(recordRuns.map(run => hex(run._1 << 11 | indexOf(run._2), 8)), 8)
  w += "};"
  w += "/* status (3 bits), RFC 5892 class (3), bidi class (4), joining type (3), mark (1), virama (1) */"
  w += s"static const uint16_t prop_values[${distinct.length}] = {"
  rows(distinct.map(hex(_, 4)), 12)
  w += "};"
  w += ""

  // 2. mappings: sorted code points; a value is one code point, or a slice of map_pool
  val keys = (0 until codePoints).filter(idna.mapping(_) != null)
  val pool = mutable.ArrayBuffer.empty[Int]
  val pooled = mutable.HashMap.empty[String, Int]
  var longest = 1
  val values = for key <- keys yield
    val m = idna.mapping(key)
    longest = longest.max(m.length)
    if m.length == 1 then m(0)
    else
      val at = pooled.getOrElseUpdate(m.mkString(" "), { val at = pool.length; pool ++= m; at })
      if at >= (1 << 16) || m.length >= 64 then die(s"the mapping of U+${Integer.toHexString(key)} does not fit a map_vals entry")
      0x80000000 | m.length << 16 | at
  w += s"#define MAP_COUNT ${keys.length}"
  w += s"#define MAP_MAX $longest"
  w += "static const uint32_t map_keys[MAP_COUNT] = {"
  rows(keys.map(hex(_, 5)), 10)
  w += "};"
  w += "/* one code point, or 0x80000000 | length << 16 | offset into map_pool */"
  w += "static const uint32_t map_vals[MAP_COUNT] = {"
  rows(values.map(hex(_, 8)), 8)
  w += "};"
  w += s"static const uint32_t map_pool[${pool.length}] = {"
  rows(pool.map(hex(_, 5)).toVector, 10)
  w += "};"
  w += ""

  // 3. canonical decompositions (one step: a pair, or a singleton with second == 0)
  val decomposed = props.decomposed.sorted.toVector
  w += s"#define DECOMP_COUNT ${decomposed.length}"
  w += "static const uint32_t decomp_keys[DECOMP_COUNT] = {"
  rows(decomposed.map(hex(_, 5)), 10)
  w += "};"
  w += "static const uint32_t decomp_first[DECOMP_COUNT] = {"
  rows(decomposed.map(c => hex(props.canonical(c)(0), 5)), 10)
  w += "};"
  w += "static const uint32_t decomp_second[DECOMP_COUNT] = {"
  rows(decomposed.map(c => hex(if props.canonical(c).length > 1 then props.canonical(c)(1) else 0, 5)), 10)
  w += "};"
  val deepest = decomposed.map(c => props.decompose(Seq(c), compat = false).length).max
  w += s"#define DECOMP_MAX ${deepest.max(3)}"
  // compositions: indices into the decomposition arrays, ordered by (first, second)
  val decompIndex = decomposed.zipWithIndex.toMap
  val compositions = props.pairs.toVector.sortBy(_._1).map(pair => decompIndex(pair._2))
  w += s"#define COMP_COUNT ${compositions.length}"
  w += "static const uint16_t comp_index[COMP_COUNT] = {"
  rows(compositions.map(_.toString), 14)
  w += "};"
  w += ""

  // 4. canonical combining classes: runs, (first code point << 8) | class
  val classRuns = runs(props.combining, codePoints)
  w += s"#define CCC_RUNS ${classRuns.length}"
  w += "static const uint32_t ccc_runs[CCC_RUNS] = {"
  rows(classRuns.map(run => hex(run._1 << 8 | run._2, 8)), 8)
  w += "};"
  w += ""

  val text = w.mkString("\n").getBytes(UTF_8)
  val tableBytes = 4 * recordRuns.length + 2 * distinct.length + 8 * keys.length + 4 * pool.length + 12 * decomposed.length +
    2 * compositions.length + 4 * classRuns.length
  if check then
    val tree = if Files.isRegularFile(header) then Files.readAllBytes(header) else new Array[Byte](0)
    if java.util.Arrays.equals(tree, text) then println(s"idna-tables: $header holds the tables the data gives")
    else
      val treeLines = new String(tree, UTF_8).split("\n", -1)
      val dataLines = new String(text, UTF_8).split("\n", -1)
      val line = treeLines.indices.find(i => i >= dataLines.length || treeLines(i) != dataLines(i)).getOrElse(treeLines.length)
      println(s"FAIL $header is not what the data gives, from its line ${line + 1}")
      Script.exit(1)
  else
    Files.write(header, text)
    println(s"idna-tables: wrote $header, $tableBytes bytes of tables")
}
