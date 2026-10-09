package meridian.core.json

/** The JSON tree, its compact writer and its reader. Numbers keep their text, so that a value
  * written by one side reads back on the other without a floating-point detour. */
enum Json:
  case Str(value: String)
  case Num(text: String)
  case Bool(value: Boolean)
  case Arr(items: List[Json])
  case Obj(fields: List[(String, Json)])
  case Null

  def render: String =
    val out = new StringBuilder
    Json.write(this, out)
    out.toString

  def get(key: String): Option[Json] = this match
    case Obj(fields) => fields.find(_._1 == key).map(_._2)
    case _ => None

  def isNull: Boolean = this == Null

object Json:
  def obj(fields: (String, Json)*): Json = Obj(fields.toList)
  def arr(items: Json*): Json = Arr(items.toList)
  def num(n: Long): Json = Num(n.toString)
  def num(n: Int): Json = Num(n.toString)

  def write(json: Json, out: StringBuilder): Unit = json match
    case Str(s) => writeString(s, out)
    case Num(t) => out.append(t)
    case Bool(b) => out.append(if b then "true" else "false")
    case Null => out.append("null")
    case Arr(items) =>
      out.append('[')
      var first = true
      for item <- items do
        if !first then out.append(',')
        first = false
        write(item, out)
      out.append(']')
    case Obj(fields) =>
      out.append('{')
      var first = true
      for (key, value) <- fields do
        if !first then out.append(',')
        first = false
        writeString(key, out)
        out.append(':')
        write(value, out)
      out.append('}')

  def writeString(s: String, out: StringBuilder): Unit =
    out.append('"')
    var i = 0
    while i < s.length do
      val c = s.charAt(i)
      c match
        case '"' => out.append("\\\"")
        case '\\' => out.append("\\\\")
        case '\n' => out.append("\\n")
        case '\r' => out.append("\\r")
        case '\t' => out.append("\\t")
        case other =>
          if other < ' ' then out.append("\\u00").append(hex(other >> 4)).append(hex(other & 15))
          else out.append(other)
      i += 1
    out.append('"')

  private def hex(n: Int): Char = "0123456789abcdef".charAt(n)

  def parse(text: String): Either[String, Json] =
    val reader = new Reader(text)
    reader.skipSpace()
    val result = reader.value()
    result.flatMap { json =>
      reader.skipSpace()
      if reader.atEnd then Right(json) else Left(s"trailing text at ${reader.pos}")
    }

  private final class Reader(text: String):
    var pos = 0
    def atEnd: Boolean = pos >= text.length
    def peek: Char = if atEnd then '\u0000' else text.charAt(pos)
    def skipSpace(): Unit =
      while !atEnd && (peek == ' ' || peek == '\n' || peek == '\t' || peek == '\r') do pos += 1
    def expect(c: Char): Either[String, Unit] =
      if peek == c then
        pos += 1
        Right(())
      else Left(s"expected '$c' at $pos")

    def value(): Either[String, Json] =
      skipSpace()
      peek match
        case '{' => obj()
        case '[' => arr()
        case '"' => string().map(Json.Str(_))
        case 't' => literal("true", Json.Bool(true))
        case 'f' => literal("false", Json.Bool(false))
        case 'n' => literal("null", Json.Null)
        case c if c == '-' || (c >= '0' && c <= '9') => number()
        case _ => Left(if atEnd then "unexpected end of input" else s"unexpected '${peek}' at $pos")

    private def literal(word: String, json: Json): Either[String, Json] =
      if pos + word.length <= text.length && text.substring(pos, pos + word.length) == word then
        pos += word.length
        Right(json)
      else Left(s"unexpected token at $pos")

    private def number(): Either[String, Json] =
      val start = pos
      if peek == '-' then pos += 1
      while !atEnd && (peek >= '0' && peek <= '9' || peek == '.' || peek == 'e' || peek == 'E' || peek == '+' || peek == '-') do pos += 1
      Right(Json.Num(text.substring(start, pos)))

    private def string(): Either[String, String] =
      expect('"').flatMap { _ =>
        val out = new StringBuilder
        var error: Option[String] = None
        var done = false
        while !done && error.isEmpty do
          if atEnd then error = Some("unterminated string")
          else
            val c = peek
            pos += 1
            c match
              case '"' => done = true
              case '\\' =>
                val e = peek
                pos += 1
                e match
                  case '"' => out.append('"')
                  case '\\' => out.append('\\')
                  case '/' => out.append('/')
                  case 'n' => out.append('\n')
                  case 'r' => out.append('\r')
                  case 't' => out.append('\t')
                  case 'b' => out.append('\b')
                  case 'f' => out.append('\f')
                  case 'u' =>
                    val code = text.substring(pos, pos + 4)
                    pos += 4
                    out.append(Integer.parseInt(code, 16).toChar)
                  case other => error = Some(s"invalid escape '\\$other'")
              case other => out.append(other)
        error.toLeft(out.toString)
      }

    private def arr(): Either[String, Json] =
      expect('[').flatMap { _ =>
        skipSpace()
        if peek == ']' then
          pos += 1
          Right(Json.Arr(Nil))
        else
          var items: List[Json] = Nil
          var error: Option[String] = None
          var done = false
          while !done && error.isEmpty do
            value() match
              case Left(e) => error = Some(e)
              case Right(v) =>
                items = v :: items
                skipSpace()
                if peek == ',' then pos += 1
                else if peek == ']' then
                  pos += 1
                  done = true
                else error = Some(s"expected ',' or ']' at $pos")
          error.toLeft(Json.Arr(items.reverse))
      }

    private def obj(): Either[String, Json] =
      expect('{').flatMap { _ =>
        skipSpace()
        if peek == '}' then
          pos += 1
          Right(Json.Obj(Nil))
        else
          var fields: List[(String, Json)] = Nil
          var error: Option[String] = None
          var done = false
          while !done && error.isEmpty do
            skipSpace()
            string() match
              case Left(e) => error = Some(e)
              case Right(key) =>
                skipSpace()
                expect(':') match
                  case Left(e) => error = Some(e)
                  case Right(_) =>
                    value() match
                      case Left(e) => error = Some(e)
                      case Right(v) =>
                        fields = (key, v) :: fields
                        skipSpace()
                        if peek == ',' then pos += 1
                        else if peek == '}' then
                          pos += 1
                          done = true
                        else error = Some(s"expected ',' or '}' at $pos")
          error.toLeft(Json.Obj(fields.reverse))
      }
