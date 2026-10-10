package dev.teq.sbt

import scala.collection.mutable
import scala.collection.compat.*

/** The JSON of teq's answers: a reader small enough to carry without a dependency, and the
  * strings the analysis writes. */
object Json {
  /** The order of strings by their UTF-8 bytes, which is teq's (`str::cmp`); here rather than in
    * `Export`, since the analysis build compiles this file without that one. */
  val utf8: Ordering[String] = (a, b) =>
    java.util.Arrays.compareUnsigned(a.getBytes(java.nio.charset.StandardCharsets.UTF_8), b.getBytes(java.nio.charset.StandardCharsets.UTF_8))

  sealed trait Value {
    def apply(key: String): Value = this match {
      case Obj(fields) => fields.getOrElse(key, Null)
      case _ => Null
    }
    def items: Seq[Value] = this match {
      case Arr(elements) => elements
      case _ => Nil
    }
    def str: String = this match {
      case Str(s) => s
      case Num(n) => n
      case Bool(b) => b.toString
      case _ => ""
    }
    def strings: Seq[String] = items.map(_.str)
    def bool: Boolean = this match {
      case Bool(b) => b
      case _ => false
    }
    def double: Double = this match {
      case Num(n) => n.toDoubleOption.getOrElse(0.0)
      case _ => 0.0
    }
    def isNull: Boolean = this == Null
  }
  final case class Obj(fields: Map[String, Value]) extends Value
  final case class Arr(elements: Seq[Value]) extends Value
  final case class Str(value: String) extends Value
  final case class Num(text: String) extends Value
  final case class Bool(value: Boolean) extends Value
  case object Null extends Value

  final class Malformed(message: String) extends Exception(message)

  def parse(text: String): Value = {
    val p = new Parser(text)
    val v = p.value()
    p.skipSpace()
    if (p.pos != text.length) throw new Malformed(s"trailing text at ${p.pos}")
    v
  }

  private final class Parser(text: String) {
    var pos = 0
    def skipSpace(): Unit = while (pos < text.length && text.charAt(pos).isWhitespace) pos += 1
    private def fail(what: String): Nothing = throw new Malformed(s"$what at $pos: ${text.slice(pos, pos + 40)}")
    private def expect(c: Char): Unit = {
      skipSpace()
      if (pos < text.length && text.charAt(pos) == c) pos += 1 else fail(s"expected '$c'")
    }
    def value(): Value = {
      skipSpace()
      if (pos >= text.length) fail("unexpected end")
      text.charAt(pos) match {
        case '{' =>
          pos += 1
          val fields = mutable.LinkedHashMap.empty[String, Value]
          skipSpace()
          if (pos < text.length && text.charAt(pos) == '}') { pos += 1; Obj(fields.toMap) }
          else {
            var more = true
            while (more) {
              skipSpace()
              val key = string()
              expect(':')
              fields(key) = value()
              skipSpace()
              if (pos < text.length && text.charAt(pos) == ',') pos += 1 else more = false
            }
            expect('}')
            Obj(fields.toMap)
          }
        case '[' =>
          pos += 1
          val items = Vector.newBuilder[Value]
          skipSpace()
          if (pos < text.length && text.charAt(pos) == ']') { pos += 1; Arr(items.result()) }
          else {
            var more = true
            while (more) {
              items += value()
              skipSpace()
              if (pos < text.length && text.charAt(pos) == ',') pos += 1 else more = false
            }
            expect(']')
            Arr(items.result())
          }
        case '"' => Str(string())
        case 't' if text.startsWith("true", pos) => { pos += 4; Bool(true) }
        case 'f' if text.startsWith("false", pos) => { pos += 5; Bool(false) }
        case 'n' if text.startsWith("null", pos) => { pos += 4; Null }
        case c if c == '-' || c.isDigit =>
          val start = pos
          pos += 1
          while (pos < text.length && (text.charAt(pos).isDigit || "+-.eE".indexOf(text.charAt(pos)) >= 0)) pos += 1
          Num(text.substring(start, pos))
        case _ => fail("unexpected character")
      }
    }
    private def string(): String = {
      if (pos >= text.length || text.charAt(pos) != '"') fail("expected a string")
      pos += 1
      val out = new StringBuilder
      while (pos < text.length && text.charAt(pos) != '"') {
        val c = text.charAt(pos)
        if (c == '\\') {
          pos += 1
          if (pos >= text.length) fail("unfinished escape")
          text.charAt(pos) match {
            case '"' => out += '"'
            case '\\' => out += '\\'
            case '/' => out += '/'
            case 'n' => out += '\n'
            case 'r' => out += '\r'
            case 't' => out += '\t'
            case 'b' => out += '\b'
            case 'f' => out += '\f'
            case 'u' =>
              val hex = text.slice(pos + 1, pos + 5)
              out += Integer.parseInt(hex, 16).toChar
              pos += 4
            case other => fail(s"unknown escape \\$other")
          }
          pos += 1
        }
        else {
          out += c
          pos += 1
        }
      }
      if (pos >= text.length) fail("unterminated string")
      pos += 1
      out.result()
    }
  }

  def string(s: String): String = {
    val escaped = s.flatMap {
      case '"' => "\\\""
      case '\\' => "\\\\"
      case '\n' => "\\n"
      case c if c < ' ' => f"\\u${c.toInt}%04x"
      case c => c.toString
    }
    "\"" + escaped + "\""
  }

  def strings(items: Seq[String]): String = {
    val inline = items.map(string).mkString("[", ", ", "]")
    if (inline.length <= 100) inline else items.map(item => s"    ${string(item)}").mkString("[\n", ",\n", "\n  ]")
  }

  def obj(fields: Seq[(String, String)], indent: String): String =
    fields.map{ case (key, value) => s"$indent  ${string(key)}: $value"}.mkString("{\n", ",\n", s"\n$indent}")
}
