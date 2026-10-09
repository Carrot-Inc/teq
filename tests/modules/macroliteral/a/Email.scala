package mla

import java.util.Locale

// A model whose parsing a string interpolator's macro runs at each expansion downstream: its
// body selects the lean std's `toLowerCase(locale)` and `contains(part)`, which scala-library has
// as the JDK's `toLowerCase(Locale)` and `contains(CharSequence)`.
final case class Email private (toStr: String)

object Email:
  private def apply(str: String): Unit = ()

  def fromStringEither(str: String): Either[String, Email] = fromString(str).toRight("not an email: " + str)

  def fromString(str: String): Option[Email] =
    val normalized = str.trim.toLowerCase(Locale.ROOT)
    if normalized.length > 1 && normalized.contains("@") && normalized.length < 100 then
      Some(new Email(normalized.replace("+", "")))
    else
      None

  def shout(e: Email): String = e.toStr.toUpperCase(Locale.ROOT)
