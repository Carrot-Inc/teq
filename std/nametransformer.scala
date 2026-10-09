package scala.reflect

/** scala-library's encoding of operator characters in names (`+` as `$plus`) and its decoding. */
object NameTransformer:
  final val MODULE_SUFFIX_STRING = "$"
  final val NAME_JOIN_STRING = "$"
  final val MODULE_INSTANCE_NAME = "MODULE$"
  final val LOCAL_SUFFIX_STRING = " "
  final val SETTER_SUFFIX_STRING = "_$eq"
  final val TRAIT_SETTER_SEPARATOR_STRING = "$_setter_$"

  private val codes: List[(Char, String)] = List(
    '~' -> "$tilde", '=' -> "$eq", '<' -> "$less", '>' -> "$greater", '!' -> "$bang", '#' -> "$hash",
    '%' -> "$percent", '^' -> "$up", '&' -> "$amp", '|' -> "$bar", '*' -> "$times", '/' -> "$div",
    '+' -> "$plus", '-' -> "$minus", ':' -> "$colon", '\\' -> "$bslash", '?' -> "$qmark", '@' -> "$at"
  )

  def encode(name: String): String =
    val out = new StringBuilder
    var i = 0
    while i < name.length do
      val c = name.charAt(i)
      codes.find(_._1 == c) match
        case Some((_, code)) => out.append(code)
        case None if !Character.isJavaIdentifierPart(c) => out.append("$u%04X".format(c.toInt))
        case None => out.append(c)
      i += 1
    out.toString

  def decode(name0: String): String =
    val name = if name0.endsWith("<init>") then name0.stripSuffix("<init>") + "this" else name0
    val out = new StringBuilder
    var i = 0
    while i < name.length do
      val next = if name.charAt(i) == '$' && i + 2 < name.length then decodeAt(name, i) else None
      next match
        case Some((text, width)) =>
          out.append(text)
          i += width
        case None =>
          out.append(name.charAt(i))
          i += 1
    out.toString

  /** The operator or the `$uXXXX` glyph at `i`, as scala-library reads them: an operator's code
    * is two lower-case letters after `$`, a glyph's first hex digit a digit or an upper-case letter. */
  private def decodeAt(name: String, i: Int): Option[(String, Int)] =
    val (ch1, ch2) = (name.charAt(i + 1), name.charAt(i + 2))
    if ch1 >= 'a' && ch1 <= 'z' && ch2 >= 'a' && ch2 <= 'z' then
      codes.find((_, code) => name.startsWith(code, i)).map((c, code) => (c.toString, code.length))
    else if name.length - i >= 6 && ch1 == 'u' && (ch2.isDigit || (ch2 >= 'A' && ch2 <= 'F')) then
      val hex = name.substring(i + 2, i + 6)
      if hex.forall(c => Character.digit(c, 16) >= 0) then Some((Integer.parseInt(hex, 16).toChar.toString, 6)) else None
    else None
