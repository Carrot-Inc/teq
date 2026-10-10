package dev.teq.sbt

import scala.collection.compat.*

/** `teq.lock`'s canonical form (docs/TARGETS.md, "The export and the project verbs", the format): the
  * subset of YAML 1.2 teq's readers take, written from the export's tree byte for byte as teq's
  * `task::lock::write` and the example's `check-export.py` write it. */
object Lock {
  val FileName = "teq.lock"
  val Format = 1
  /** YAML's bound on an implicit key, in characters as written. */
  val MaxKey = 1024

  /** The root's keys that come first, in this order: the header a launcher reads. */
  private val Header = Seq("teq", "format", "binaries")

  /** Where a record is written on one line as a flow mapping, unless it holds a list: a
    * classpath's entries and a stage layer's mappings, by the keys above them (`*` any key or
    * index; `task::lock::RECORDS` in teq). */
  private val Records = Seq(Seq("projects", "*", "configurations", "*", "classpath", "*"), Seq("projects", "*", "stage", "layers", "*", "*"))

  /** The words a YAML reader may take for null or a truth value, YAML 1.1's among them, and the
    * core schema's infinities and not-a-number. */
  private val Reserved = Set(
    "null", "Null", "NULL", "true", "True", "TRUE", "false", "False", "FALSE", "yes", "Yes", "YES", "no", "No", "NO", "on", "On", "ON",
    "off", "Off", "OFF", "y", "Y", "n", "N", ".inf", ".Inf", ".INF", ".nan", ".NaN", ".NAN",
  )

  private def digit(c: Char) = c >= '0' && c <= '9'
  private def letter(c: Char) = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')

  /** A word of a plain string: ASCII letters, digits and `_ . / + - : @`, led by a letter, a
    * digit, `_`, `.` or `/`, and not ended by `:`. */
  def word(s: String): Boolean =
    s.nonEmpty && s.forall(c => letter(c) || digit(c) || "_./+-:@".indexOf(c) >= 0) &&
      (letter(s(0)) || digit(s(0)) || "_./".indexOf(s(0)) >= 0) && s.last != ':'

  /** Whether a string is written plain where a key or a flow mapping's value stands: a word that
    * no YAML reader takes for null, a truth value, a number or a date. */
  def plain(s: String): Boolean = word(s) && !Reserved(s) && !numberLike(s) && !radix(s)

  /** Whether a string is written plain on a line of its own: one plain word, or words joined by
    * single spaces, the first plain. */
  def plainLine(s: String): Boolean = {
    val words = s.split(" ", -1)
    plain(words(0)) && words.forall(word)
  }

  private def numberLike(s: String): Boolean =
    (digit(s(0)) || (s(0) == '.' && s.length > 1 && digit(s(1)))) && s.forall(c => digit(c) || "._:-+eEtTzZ".indexOf(c) >= 0) && s.count(_ == '.') <= 1

  private def radix(s: String): Boolean =
    s.length > 2 && s(0) == '0' && "xob".indexOf(s(1)) >= 0 && s.drop(2).forall(c => digit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F') || c == '_')

  /** A string in double quotes: `"` and `\` escaped, line feed, carriage return and tab as `\n`,
    * `\r` and `\t`, and as a `u` escape of four hex digits every other character YAML does not print or a reader may take
    * for a line break (the C0 and C1 controls, DEL, U+2028, U+2029, U+FEFF, U+FFFE, U+FFFF). */
  def quoted(s: String): String = {
    val out = new StringBuilder("\"")
    s.foreach {
      case '"' => out ++= "\\\""
      case '\\' => out ++= "\\\\"
      case '\n' => out ++= "\\n"
      case '\r' => out ++= "\\r"
      case '\t' => out ++= "\\t"
      case c if c < ' ' || (c >= 0x7f && c <= 0x9f) || c == 0x2028 || c == 0x2029 || c == 0xfeff || c == 0xfffe || c == 0xffff =>
        out ++= f"\\u${c.toInt}%04x"
      case c => out += c
    }
    (out += '"').result()
  }

  private def key(k: String): String = if (plain(k)) k else quoted(k)

  /** What the lock cannot hold, each by where it is: a key longer than YAML allows an implicit
    * key, and a string or key holding a lone surrogate, which no UTF-8 writes. */
  def problems(value: Json.Value): Seq[String] = {
    def lone(s: String) = s.indices.exists { i =>
      val c = s(i)
      (Character.isHighSurrogate(c) && !(i + 1 < s.length && Character.isLowSurrogate(s(i + 1)))) ||
      (Character.isLowSurrogate(c) && !(i > 0 && Character.isHighSurrogate(s(i - 1))))
    }
    def where(path: Vector[String]) = if (path.isEmpty) "the root" else path.mkString(".")
    def walk(v: Json.Value, path: Vector[String]): Seq[String] = v match {
      case Json.Obj(fields) =>
        fields.toSeq.sortBy(_._1)(using Json.utf8).flatMap { case (k, x) =>
          val written = key(k)
          val long = Option.when(written.codePointCount(0, written.length) > MaxKey)(
            s"the key ${k.take(60)}... at ${where(path)} is ${written.codePointCount(0, written.length)} characters as ${FileName} writes it, more than YAML's $MaxKey for a key"
          )
          val broken = Option.when(lone(k))(s"the key ${quoted(k)} at ${where(path)} holds a lone surrogate, which ${FileName} cannot write")
          long.toSeq ++ broken ++ walk(x, path :+ k)
        }
      case Json.Arr(items) => items.zipWithIndex.flatMap{ case (x, i) => walk(x, path :+ i.toString)}
      case Json.Str(s) if lone(s) => Seq(s"the string ${quoted(s)} at ${where(path)} holds a lone surrogate, which ${FileName} cannot write")
      case Json.Null => Seq(s"${where(path)} is null, which ${FileName} does not hold")
      case _ => Nil
    }
    walk(value, Vector.empty)
  }

  /** The canonical text of an export's tree: a line per entry or element, two spaces deeper per
    * level, keys sorted by their UTF-8 bytes (the root's header first), a record on one line,
    * strings plain where `plain` or `plainLine` allows and double-quoted elsewhere, a trailing
    * newline. */
  def canonical(value: Json.Value): String = {
    def isRecord(path: Vector[String], v: Json.Value): Boolean = v match {
      case Json.Obj(_) if !holdsList(v) => Records.exists(r => r.size == path.size && r.zip(path).forall{ case (a, b) => a == "*" || a == b})
      case _ => false
    }
    def holdsList(v: Json.Value): Boolean = v match {
      case Json.Arr(_) => true
      case Json.Obj(fields) => fields.values.exists(holdsList)
      case _ => false
    }
    def sorted(fields: Map[String, Json.Value], root: Boolean): Seq[(String, Json.Value)] = {
      val all = fields.toSeq.sortBy(_._1)(using Json.utf8)
      if (root) Header.flatMap(h => fields.get(h).map(h -> _)) ++ all.filterNot(f => Header.contains(f._1)) else all
    }
    def scalar(v: Json.Value, line: Boolean): String = v match {
      case Json.Str(s) => if ((if (line) plainLine(s) else plain(s))) s else quoted(s)
      case Json.Num(n) => n
      case Json.Bool(b) => b.toString
      case Json.Arr(_) => "[]"
      case Json.Obj(_) => "{}"
      case Json.Null => throw new IllegalArgumentException(s"$FileName holds no null")
    }
    def flow(v: Json.Value): String = v match {
      case Json.Obj(fields) if fields.nonEmpty => sorted(fields, root = false).map{ case (k, x) => s"${key(k)}: ${flow(x)}"}.mkString("{", ", ", "}")
      case other => scalar(other, line = false)
    }
    def onLine(v: Json.Value, path: Vector[String]): Option[String] = v match {
      case _ if isRecord(path, v) => Some(flow(v))
      case Json.Obj(fields) if fields.nonEmpty => None
      case Json.Arr(items) if items.nonEmpty => None
      case other => Some(scalar(other, line = true))
    }
    def lines(v: Json.Value, depth: Int, path: Vector[String]): Vector[String] = {
      val indent = "  " * depth
      v match {
        case Json.Obj(fields) =>
          sorted(fields, path.isEmpty).toVector.flatMap { case (k, x) =>
            onLine(x, path :+ k) match {
              case Some(text) => Vector(s"$indent${key(k)}: $text")
              case None => s"$indent${key(k)}:" +: lines(x, depth + 1, path :+ k)
            }
          }
        case Json.Arr(items) =>
          items.toVector.flatMap { x =>
            onLine(x, path :+ "*") match {
              case Some(text) => Vector(s"$indent- $text")
              case None =>
                val block = lines(x, depth + 1, path :+ "*")
                s"$indent- ${block.head.drop(indent.length + 2)}" +: block.tail
            }
          }
        case other => Vector(indent + scalar(other, line = true))
      }
    }
    lines(value, 0, Vector.empty).mkString("", "\n", "\n")
  }
}
