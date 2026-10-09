// The small java.util pieces: TimeUnit conversions, Base64 round trips (basic, URL-safe,
// without padding, MIME), Locale constants and tags, TimeZone ids.
package javautilbits

import java.util.{Base64, Locale, TimeZone}
import java.util.concurrent.TimeUnit

@main def main(): Unit =
  println(TimeUnit.values.mkString(" "))
  println(s"${TimeUnit.SECONDS.toMillis(3)} ${TimeUnit.MILLISECONDS.toSeconds(4500)} ${TimeUnit.HOURS.toMinutes(2)}")
  println(s"${TimeUnit.DAYS.toNanos(1)} ${TimeUnit.NANOSECONDS.toMicros(123456)} ${TimeUnit.MINUTES.toHours(179)}")
  println(s"${TimeUnit.MILLISECONDS.convert(2, TimeUnit.MINUTES)} ${TimeUnit.DAYS.convert(100, TimeUnit.HOURS)}")
  println(s"${TimeUnit.DAYS.toNanos(Long.MaxValue / 2)} ${TimeUnit.NANOSECONDS.toDays(Long.MaxValue)}")
  println(s"${TimeUnit.SECONDS.toMicros(-3)} ${TimeUnit.MICROSECONDS.toMillis(-1500)}")
  println(s"${TimeUnit.valueOf("HOURS")} ${TimeUnit.HOURS.ordinal} ${TimeUnit.HOURS.name}")
  println(TimeUnit.SECONDS.compareTo(TimeUnit.MINUTES))
  val enc = Base64.getEncoder
  val dec = Base64.getDecoder
  for s <- List("", "f", "fo", "foo", "foob", "fooba", "foobar", "h\u00e9llo w\u00f6rld") do
    val e = enc.encodeToString(s.toArray.map(_.toByte))
    val back = dec.decode(e).map(b => (b & 255).toChar).mkString
    println(s"[$s] $e ${back == s}")
  val bytes = Array[Byte](-5, -1, -65, 0, 62, 63, 127, -128)
  println(enc.encodeToString(bytes))
  println(Base64.getUrlEncoder.encodeToString(bytes))
  println(Base64.getUrlEncoder.withoutPadding.encodeToString(bytes))
  println(Base64.getEncoder.withoutPadding.encodeToString(Array[Byte](97, 98)))
  println(Base64.getUrlDecoder.decode("-_-_AD4_f4A").mkString(","))
  println(Base64.getDecoder.decode("YWI").mkString(","))
  println(enc.encode(Array[Byte](120, 121, 122)).map(_.toChar).mkString)
  println(dec.decode(Array[Byte](101, 72, 108, 54)).mkString(","))
  println(Base64.getMimeEncoder.encodeToString(Array.fill[Byte](60)(65)))
  try dec.decode("a$b=")
  catch case e: IllegalArgumentException => println("IllegalArgumentException: " + e.getMessage)
  try dec.decode("abcde")
  catch case e: IllegalArgumentException => println("IllegalArgumentException: " + e.getMessage)
  for l <- List(Locale.US, Locale.UK, Locale.ENGLISH, Locale.ROOT, Locale.FRANCE, Locale.GERMANY, Locale.GERMAN, Locale.CANADA, Locale.JAPAN, new Locale("es", "MX"), new Locale("pt"), Locale.forLanguageTag("en-AU")) do
    println(s"[$l] ${l.getLanguage} ${l.getCountry} ${l.toLanguageTag}")
  println(Locale.US == new Locale("en", "US"))
  println(Locale.US.hashCode == new Locale("en", "US").hashCode)
  println(Locale.of("de", "AT"))
  println(s"${"TITLE".toLowerCase(Locale.ROOT)} ${"title".toUpperCase(Locale.US)}")
  val tz = TimeZone.getTimeZone("America/Los_Angeles")
  println(tz.getID)
  println(TimeZone.getTimeZone("UTC").getID)

