package scala2lib.timecodecs

// A package object that extends a Scala 2 trait mixing two unrelated others, as kantan.codecs-java8's
// `kantan.codecs.strings.java8` package object extends `TimeCodecCompanion`
// (tests/classpath/jvm/scala2_package_object_ctor.scala).

trait TimeDecoders[E, F] {
  def decode(text: String): Either[F, E]
  def decoder(format: String): String = "decoder " + format
}

trait TimeEncoders[E] {
  def encoder(format: String): String = "encoder " + format
}

trait TimeCodecs[E, F] extends TimeDecoders[E, F] with TimeEncoders[E] {
  def codec(format: String): String = decoder(format) + "/" + encoder(format)
}

package object time extends TimeCodecs[String, Int] {
  def decode(text: String): Either[Int, String] = Right(text)
}
