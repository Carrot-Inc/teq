package java.util

final class Locale(val tag: String)

object Locale:
  val US: Locale = new Locale("en-US")
  val ROOT: Locale = new Locale("")
