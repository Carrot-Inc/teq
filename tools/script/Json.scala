// JSON for the repository's scripts: a parser and a printer over a small tree. Objects keep their
// keys in order; a number keeps the text it was read as, and one made from a value prints as Python
// prints it, so that a script writes the bytes the Python script it replaced wrote (`json.dumps`:
// `", "` and `": "` between items, or with an indent each item on a line and `","`; non-ASCII
// escaped unless `ascii` is off; a float as `repr` writes it, `1e-05`, `2.0`, `5e-324`; keys sorted
// by code point). The parser reads what `json.loads` reads and refuses the rest (a leading zero, a
// control character in a string), `NaN` and `Infinity` included; a repeated key keeps its first place
// and its last value.

enum Json:
  case Null
  case Bool(value: Boolean)
  case Num(text: String)
  case Str(value: String)
  case Arr(items: Vector[Json])
  case Obj(fields: Vector[(String, Json)])

  // The member of an object (`Json.Null` where it has none), the item of an array.
  def apply(key: String): Json = this match
    case Obj(fields) => fields.collectFirst { case (k, v) if k == key => v }.getOrElse(Json.Null)
    case _ => Json.Null
  def apply(index: Int): Json = this match
    case Arr(items) if index >= 0 && index < items.length => items(index)
    case _ => Json.Null
  def get(key: String): Option[Json] = this match
    case Obj(fields) => fields.collectFirst { case (k, v) if k == key => v }
    case _ => None
  def str: String = this match
    case Str(s) => s
    case other => throw new IllegalArgumentException("not a string: " + Json.write(other))
  def strOrNull: String = this match
    case Str(s) => s
    case _ => null
  def long: Long = this match
    case Num(t) => if t.exists(c => c == '.' || c == 'e' || c == 'E') then t.toDouble.toLong else t.toLong
    case other => throw new IllegalArgumentException("not a number: " + Json.write(other))
  def int: Int = long.toInt
  def double: Double = this match
    case Num(t) => t.toDouble
    case other => throw new IllegalArgumentException("not a number: " + Json.write(other))
  def bool: Boolean = this match
    case Bool(b) => b
    case other => throw new IllegalArgumentException("not a boolean: " + Json.write(other))
  // An array's items, an object's members; none for another value.
  def elements: Vector[Json] = this match
    case Arr(xs) => xs
    case _ => Vector.empty
  def members: Vector[(String, Json)] = this match
    case Obj(fs) => fs
    case _ => Vector.empty
  def isNull: Boolean = this == Json.Null
  // The object with `key` set: replaced in its place, else added at the end.
  def updated(key: String, value: Json): Json = this match
    case Obj(fs) =>
      val at = fs.indexWhere(_._1 == key)
      Obj(if at >= 0 then fs.updated(at, (key, value)) else fs :+ (key -> value))
    case other => throw new IllegalArgumentException("not an object: " + Json.write(other))

object Json:
  final class ParseError(message: String) extends RuntimeException(message)

  def obj(fields: (String, Json)*): Json = Obj(fields.toVector)
  def arr(items: Json*): Json = Arr(items.toVector)
  def str(s: String): Json = if s == null then Null else Str(s)
  def num(n: Long): Json = Num(n.toString)
  def num(n: Int): Json = Num(n.toString)
  def num(d: Double): Json = Num(pythonFloat(d))
  def bool(b: Boolean): Json = Bool(b)

  // ---- parsing ----

  // The value the text holds, white space around it; `ParseError` naming the place of what is wrong.
  def parse(text: String): Json =
    val p = new Parser(text)
    p.space()
    val v = p.value()
    p.space()
    if p.at < text.length then p.fail("extra data")
    v

  private final class Parser(s: String):
    var at = 0
    def fail(what: String): Nothing =
      val line = s.substring(0, at.min(s.length)).count(_ == '\n') + 1
      throw new ParseError(s"$what at line $line, offset $at")
    def space(): Unit =
      while at < s.length && (s.charAt(at) == ' ' || s.charAt(at) == '\n' || s.charAt(at) == '\r' || s.charAt(at) == '\t') do at += 1
    def expect(c: Char): Unit =
      if at >= s.length || s.charAt(at) != c then fail(s"expected '$c'")
      at += 1
    def word(w: String, v: Json): Json =
      if !s.startsWith(w, at) then fail("unexpected character")
      at += w.length
      v
    def value(): Json =
      if at >= s.length then fail("unexpected end")
      s.charAt(at) match
        case '{' => obj()
        case '[' => arr()
        case '"' => Str(string())
        case 't' => word("true", Bool(true))
        case 'f' => word("false", Bool(false))
        case 'n' => word("null", Null)
        // Python's `json` reads these too, and writes them for a double that is no number.
        case 'N' => word("NaN", Num("NaN"))
        case 'I' => word("Infinity", Num("Infinity"))
        case '-' if s.startsWith("-Infinity", at) => word("-Infinity", Num("-Infinity"))
        case c if c == '-' || (c >= '0' && c <= '9') => number()
        case _ => fail("unexpected character")
    // A repeated key keeps its first place and takes its last value, as Python's dict does.
    def obj(): Json =
      at += 1
      val fields = scala.collection.mutable.ArrayBuffer.empty[(String, Json)]
      val seen = scala.collection.mutable.HashMap.empty[String, Int]
      space()
      if at < s.length && s.charAt(at) == '}' then
        at += 1
        Obj(fields.toVector)
      else
        var more = true
        while more do
          space()
          if at >= s.length || s.charAt(at) != '"' then fail("expected a key")
          val k = string()
          space()
          expect(':')
          space()
          val v = value()
          seen.get(k) match
            case Some(i) => fields(i) = (k, v)
            case None =>
              seen(k) = fields.length
              fields += (k -> v)
          space()
          if at < s.length && s.charAt(at) == ',' then at += 1
          else
            expect('}')
            more = false
        Obj(fields.toVector)
    def arr(): Json =
      at += 1
      val items = Vector.newBuilder[Json]
      space()
      if at < s.length && s.charAt(at) == ']' then
        at += 1
        Arr(items.result())
      else
        var more = true
        while more do
          space()
          items += value()
          space()
          if at < s.length && s.charAt(at) == ',' then at += 1
          else
            expect(']')
            more = false
        Arr(items.result())
    // The runs between escapes and the escapes' characters, joined once (an interpreted
    // StringBuilder copies its text at each append).
    def string(): String =
      at += 1
      val parts = scala.collection.mutable.ArrayBuffer.empty[String]
      var done = false
      while !done do
        // The run up to the next quote or escape, copied at once.
        var end = at
        while end < s.length && s.charAt(end) != '"' && s.charAt(end) != '\\' && s.charAt(end) >= ' ' do end += 1
        if end >= s.length then fail("unterminated string")
        if s.charAt(end) < ' ' then
          at = end
          fail("invalid control character")
        if end > at then parts += s.substring(at, end)
        at = end
        if s.charAt(at) == '"' then
          at += 1
          done = true
        else
          if at + 1 >= s.length then fail("unterminated string")
          val e = s.charAt(at + 1)
          at += 2
          e match
            case '"' => parts += "\""
            case '\\' => parts += "\\"
            case '/' => parts += "/"
            case 'b' => parts += "\b"
            case 'f' => parts += "\f"
            case 'n' => parts += "\n"
            case 'r' => parts += "\r"
            case 't' => parts += "\t"
            case 'u' =>
              if at + 4 > s.length then fail("bad \\u escape")
              val hex = s.substring(at, at + 4)
              if !hex.forall(c => Character.digit(c, 16) >= 0) then fail("bad \\u escape")
              parts += Integer.parseInt(hex, 16).toChar.toString
              at += 4
            case _ => fail("bad escape")
      if parts.length == 1 then parts(0) else parts.mkString("", "", "")
    // JSON's grammar: an integer part of one 0 or a digit 1 to 9 and more digits, so that `01` ends
    // at its 0 and what follows is refused where the number stands.
    def number(): Json =
      val start = at
      if s.charAt(at) == '-' then at += 1
      def digits(): Unit =
        val from = at
        while at < s.length && s.charAt(at) >= '0' && s.charAt(at) <= '9' do at += 1
        if at == from then fail("expected a digit")
      if at < s.length && s.charAt(at) == '0' then at += 1 else digits()
      if at < s.length && s.charAt(at) == '.' then
        at += 1
        digits()
      if at < s.length && (s.charAt(at) == 'e' || s.charAt(at) == 'E') then
        at += 1
        if at < s.length && (s.charAt(at) == '+' || s.charAt(at) == '-') then at += 1
        digits()
      Num(s.substring(start, at))

  // ---- printing ----

  // As Python's `json.dumps(value, indent=indent, ensure_ascii=ascii, sort_keys=sortKeys)`.
  // The text's parts are joined once, at the end (an interpreted StringBuilder copies its text at
  // each append).
  def write(value: Json, indent: Int = -1, ascii: Boolean = true, sortKeys: Boolean = false): String =
    val out = scala.collection.mutable.ArrayBuffer.empty[String]
    def go(v: Json, depth: Int): Unit = v match
      case Null => out += "null"
      case Bool(b) => out += (if b then "true" else "false")
      case Num(t) => out += t
      case Str(x) => quote(x, ascii, out)
      case Arr(items) =>
        if items.isEmpty then out += "[]"
        else
          out += "["
          items.zipWithIndex.foreach { (item, i) =>
            if i > 0 then out += (if indent >= 0 then "," else ", ")
            newline(depth + 1)
            go(item, depth + 1)
          }
          newline(depth)
          out += "]"
      case Obj(fields) =>
        if fields.isEmpty then out += "{}"
        else
          out += "{"
          val ordered = if sortKeys then fields.sortWith((a, b) => byCodePoint(a._1, b._1) < 0) else fields
          ordered.zipWithIndex.foreach { case ((k, item), i) =>
            if i > 0 then out += (if indent >= 0 then "," else ", ")
            newline(depth + 1)
            quote(k, ascii, out)
            out += ": "
            go(item, depth + 1)
          }
          newline(depth)
          out += "}"
    def newline(depth: Int): Unit =
      if indent >= 0 then out += "\n" + " " * (depth * indent)
    go(value, 0)
    out.mkString("", "", "")

  // A string in quotes, escaped as Python escapes it.
  def quote(s: String, ascii: Boolean): String =
    val out = scala.collection.mutable.ArrayBuffer.empty[String]
    quote(s, ascii, out)
    out.mkString("", "", "")

  // The quoted string's parts: each run that needs no escape at once, then the escape.
  private def quote(s: String, ascii: Boolean, out: scala.collection.mutable.ArrayBuffer[String]): Unit =
    out += "\""
    var run = 0
    var i = 0
    while i < s.length do
      val c = s.charAt(i)
      val escape = c match
        case '"' => "\\\""
        case '\\' => "\\\\"
        case '\n' => "\\n"
        case '\r' => "\\r"
        case '\t' => "\\t"
        case '\b' => "\\b"
        case '\f' => "\\f"
        case _ if c < 0x20 || (ascii && c > 0x7e) =>
          val hex = Integer.toHexString(c)
          "\\u" + "0000".substring(hex.length) + hex
        case _ => null
      if escape != null then
        if i > run then out += s.substring(run, i)
        out += escape
        run = i + 1
      i += 1
    if s.length > run then out += s.substring(run)
    out += "\""

  // Python's order of strings, by code point (a supplementary character after U+FFFF), where
  // `String.compareTo` compares UTF-16 units.
  def byCodePoint(a: String, b: String): Int =
    var i = 0
    var j = 0
    var c = 0
    while c == 0 && i < a.length && j < b.length do
      val x = a.codePointAt(i)
      val y = b.codePointAt(j)
      c = Integer.compare(x, y)
      i += (if x >= 0x10000 then 2 else 1)
      j += (if y >= 0x10000 then 2 else 1)
    if c != 0 then c else Integer.compare(a.length - i, b.length - j)

  // The digits of the shortest decimal that reads back as the positive finite `a`, the nearest to
  // it of those (ties to an even last digit), and the decimal exponent of the first digit: David
  // Gay's shortest mode, which Python's `repr` uses (`Double.toString` keeps at least two digits,
  // 4.9E-324 where Python writes 5e-324). Each precision from one digit, the decimals just below
  // and just above `a` at it, from `a`'s exact value (its significand times a power of two, as an
  // integer times a power of ten).
  private def shortestDigits(a: Double): (String, Int) =
    val bits = java.lang.Double.doubleToRawLongBits(a)
    val biased = ((bits >>> 52) & 0x7ff).toInt
    val fraction = bits & 0xfffffffffffffL
    val (m, e2) = if biased == 0 then (fraction, -1074) else (fraction | (1L << 52), biased - 1075)
    val (n, e10) = if e2 >= 0 then (BigInt(m) * BigInt(2).pow(e2), 0) else (BigInt(m) * BigInt(5).pow(-e2), e2)
    val all = n.toString
    var p = 1
    var result: (String, Int) = null
    while result == null do
      val head = all.substring(0, p.min(all.length))
      val rest = all.substring(head.length)
      val exp = e10 + rest.length
      val down = BigInt(head)
      val up = if rest.exists(_ != '0') then down + 1 else down
      def reads(v: BigInt): Boolean = java.lang.Double.parseDouble(v.toString + "E" + exp) == a
      val (downOk, upOk) = (reads(down), reads(up))
      if downOk || upOk then
        // The nearer, by what lies past the precision against a half.
        val half = if rest.isEmpty then 0 else if rest.charAt(0) != '5' then rest.charAt(0) - '5' else if rest.substring(1).exists(_ != '0') then 1 else 0
        val pick =
          if downOk && upOk && up != down then (if half < 0 then down else if half > 0 then up else if down % 2 == 0 then down else up)
          else if downOk then down
          else up
        val text = pick.toString
        val digits = text.reverse.dropWhile(_ == '0').reverse
        result = (digits, exp + text.length - 1)
      p += 1
    result

  // A double as Python's `repr` writes it: the shortest digits that read back as it, fixed from
  // 1e-4 to below 1e16 (with `.0` when whole), else `<digits>e<sign><two or more digits>`.
  def pythonFloat(d: Double): String =
    if d.isNaN then "NaN"
    else if d.isInfinite then (if d > 0 then "Infinity" else "-Infinity")
    else if d == 0.0 then (if 1.0 / d < 0 then "-0.0" else "0.0")
    else
      val (digits, e) = shortestDigits(Math.abs(d))
      val sign = if d < 0 then "-" else ""
      if e < -4 || e >= 16 then
        val m = if digits.length == 1 then digits else digits.substring(0, 1) + "." + digits.substring(1)
        val es = Math.abs(e).toString
        sign + m + "e" + (if e < 0 then "-" else "+") + (if es.length < 2 then "0" + es else es)
      else if e < 0 then sign + "0." + ("0" * (-e - 1)) + digits
      else if digits.length <= e + 1 then sign + digits + ("0" * (e + 1 - digits.length)) + ".0"
      else sign + digits.substring(0, e + 1) + "." + digits.substring(e + 1)
